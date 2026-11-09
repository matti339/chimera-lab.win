use super::*;
use chimera::control::score::Gesture;
pub const BODIES: [BodyMix; 2] = [BodyMix::FEMALE, BodyMix::CHILD];
pub struct Layers {
    pub engines: [Engine; 2],
    pub banks: [speech::Bank; 2],
    pub vowels: [[ArticPatch; 5]; 2],
    pub scratch: Vec<Gesture>,
    pub last: [f32; 2],
    weights: [f32; 3],
    smoothing: f32,
}
impl Layers {
    pub fn new(rate: f32) -> Self {
        let engines = std::array::from_fn(|i| {
            let mut e = Engine::new(rate as f64);
            e.set_body(BODIES[i]);
            e.render(&mut [0.; 256]);
            e
        });
        Self {
            engines,
            banks: std::array::from_fn(|_| speech::Bank::new(true)),
            vowels: std::array::from_fn(|i| {
                ["ɑ", "ɛ", "i", "o", "u"].map(|v| vowel_artic(v, BODIES[i]).unwrap())
            }),
            scratch: Vec::with_capacity(speech::MAX_GESTURES),
            last: [0.; 2],
            weights: [1., 0., 0.],
            smoothing: 1. - (-1. / (rate * 0.015)).exp(),
        }
    }
    pub fn reset(&mut self) {
        for e in &mut self.engines {
            e.set_gate(0.);
            e.stop_speech();
        }
        self.last = [0.; 2];
        self.weights = [1., 0., 0.];
    }
    pub fn update(
        &mut self,
        p: &Controls,
        source: &SourcePatch,
        notes: &Notes,
        gate: f64,
        performance: &lab::Performance,
        shared: &Shared,
    ) {
        if p.lab.voice.value() == 0.
            && p.lab.choir.value() == 0.
            && self.weights[1] < 0.00001
            && self.weights[2] < 0.00001
        {
            return;
        }
        for i in 0..2 {
            let mut a = p.base_artic(&self.vowels[i]);
            performance.articulate(p, &mut a);
            self.engines[i].set_artic(&a, Some(p.tip_x.value() == 0. && p.tip_y.value() == 0.));
            let mut s = *source;
            if let Some(f) = s.f0 {
                let choir = p.lab.choir.value();
                let hz = if choir > 0. {
                    notes
                        .ranked(i + 1)
                        .map(|(ch, n, _)| {
                            440. * 2.0_f64.powf(
                                (n as f64 - 69.
                                    + notes.bend[ch as usize] as f64
                                        * p.lab.bend_range.value() as f64)
                                    / 12.,
                            )
                        })
                        .unwrap_or(
                            f * 2.0_f64.powf(if i == 0 {
                                p.lab.harmony.value() as f64 / 12.
                            } else {
                                -1.
                            }),
                        )
                } else {
                    f
                };
                s.f0 = Some(
                    hz * 2.0_f64.powf(if choir > 0. {
                        p.lab.detune.value() as f64 * (if i == 0 { 1. } else { -1. }) / 1200.
                    } else {
                        0.
                    }),
                );
            }
            if p.lab.expression.value() > 0. {
                let (channel, _, velocity) = notes
                    .ranked(i + 1)
                    .or_else(|| notes.active())
                    .unwrap_or((0, 48, 0.85));
                let pressure =
                    f32::from_bits(shared.pressure[channel as usize].load(Ordering::Relaxed))
                        * p.lab.expression.value();
                s.pressure = Some(
                    (5. + 5. * velocity as f64)
                        * (1. + p.lab.effort.value() as f64 * 0.6 + pressure as f64 * 0.6),
                );
                let slide =
                    (f32::from_bits(shared.slide[channel as usize].load(Ordering::Relaxed)) - 0.5)
                        * p.lab.expression.value();
                if slide != 0. {
                    a.set(
                        ArticKey::TbY,
                        a.get(ArticKey::TbY).unwrap_or(6.85) + slide as f64,
                    );
                    self.engines[i].set_artic(&a, None);
                }
            }
            self.engines[i].set_source(&s);
            self.engines[i].set_gate(gate);
        }
    }
    pub fn live(&mut self, index: usize, elapsed: f64, p: &Controls) {
        for i in 0..2 {
            self.scratch.clear();
            self.scratch.extend_from_slice(&self.banks[i].on[index]);
            p.modulate(&mut self.scratch);
            self.engines[i].schedule(&self.scratch, -elapsed, true);
        }
    }
    pub fn release(&mut self, index: usize, p: &Controls) {
        for i in 0..2 {
            let e = &mut self.engines[i];
            e.release(speech::NOTE_TAG);
            e.release(speech::NOTE_TAG + 1.);
            self.scratch.clear();
            self.scratch.extend_from_slice(&self.banks[i].off[index]);
            p.modulate(&mut self.scratch);
            e.schedule(&self.scratch, 0., false);
        }
    }
    pub fn phrase(&mut self, scale: f64, phase_seconds: f64, p: &Controls) {
        for i in 0..2 {
            speech::scale_phrase(&self.banks[i], scale, &mut self.scratch);
            p.modulate(&mut self.scratch);
            self.engines[i].schedule(&self.scratch, -phase_seconds, true);
        }
    }
    pub fn motion(&mut self, p: &Controls, performance: &lab::Performance) {
        for i in 0..2 {
            if let Some(g) = performance.gesture(p, p.base_artic(&self.vowels[i])) {
                self.engines[i].schedule(&[g], 0., false);
            }
        }
    }
    pub fn mix(&mut self, main: f32, p: &Controls, envelope: f32, gain: f32) -> [f32; 2] {
        let x = p.lab.voice.value();
        let choir = p.lab.choir.value();
        let mut weights = [
            (1. - x).max(0.),
            (1. - (x - 1.).abs()).max(0.),
            (x - 1.).max(0.),
        ];
        for w in &mut weights {
            *w = *w * (1. - choir) + choir / 3.;
        }
        for i in 0..3 {
            self.weights[i] += self.smoothing * (weights[i] - self.weights[i]);
            if (weights[i] - self.weights[i]).abs() < 1e-7 {
                self.weights[i] = weights[i];
            }
        }
        let weights = self.weights;
        let mut signals = [main, 0., 0.];
        for i in 0..2 {
            if weights[i + 1] > 0.00001 && envelope > 0. {
                let mut b = [0.];
                self.engines[i].render(&mut b);
                signals[i + 1] = b[0] * envelope * gain;
            }
        }
        let spread = p.lab.spread.value();
        let mut left = 0.;
        let mut right = 0.;
        for i in 0..3 {
            let pan = if i == 1 {
                -spread
            } else if i == 2 {
                spread
            } else {
                0.
            };
            left += signals[i] * weights[i] * (1. - pan * 0.5);
            right += signals[i] * weights[i] * (1. + pan * 0.5);
        }
        self.last = [left, right];
        self.last
    }
}
