//! Bounded physical-model polyphony. No allocation or body rebuilding in the callback.
use super::*;
use chimera::{control::score::Gesture, voice::human_voice::HumanVoice};
const MAX: usize = 6;
struct Voice {
    engine: Engine,
    warm: HumanVoice,
    vowels: [ArticPatch; 5],
    key: Option<(u8, u8)>,
    held: bool,
    velocity: f32,
    envelope: f32,
    age: u64,
    order: u64,
    syllable: usize,
    release_at: f64,
    cycle: Option<i64>,
    last: f32,
    steal_tail: f32,
}
pub struct Polyphony {
    voices: [Voice; MAX],
    scratch: Vec<Gesture>,
    serial: u64,
    clock: u64,
    last_onset: Option<u64>,
    next_syllable: usize,
    held_syllable: Option<usize>,
    revision: u64,
    rate: f32,
    steal_decay: f32,
}
fn bank<'a>(i: usize, main: &'a speech::Bank, layers: &'a layers::Layers) -> &'a speech::Bank {
    if i % 3 == 0 {
        main
    } else {
        &layers.banks[i % 3 - 1]
    }
}
impl Polyphony {
    pub fn new(rate: f32) -> Self {
        Self {
            voices: std::array::from_fn(|i| {
                let body = [BodyMix::MALE, BodyMix::FEMALE, BodyMix::CHILD][i % 3];
                let mut engine = Engine::new(rate as f64);
                engine.set_body(body);
                engine.render(&mut [0.; 256]);
                let warm = engine.voice().clone();
                Voice {
                    engine,
                    warm,
                    vowels: ["ɑ", "ɛ", "i", "o", "u"].map(|v| vowel_artic(v, body).unwrap()),
                    key: None,
                    held: false,
                    velocity: 0.,
                    envelope: 0.,
                    age: 0,
                    order: 0,
                    syllable: 0,
                    release_at: 0.,
                    cycle: None,
                    last: 0.,
                    steal_tail: 0.,
                }
            }),
            scratch: Vec::with_capacity(speech::MAX_GESTURES),
            serial: 0,
            clock: 0,
            last_onset: None,
            next_syllable: 0,
            held_syllable: None,
            revision: 0,
            rate,
            steal_decay: (-1. / (rate * 0.004)).exp(),
        }
    }
    pub fn reset(&mut self) {
        for v in &mut self.voices {
            v.engine.reset_human_from(&v.warm);
            v.key = None;
            v.held = false;
            v.envelope = 0.;
            v.steal_tail = 0.;
            v.last = 0.;
            v.cycle = None;
        }
        self.serial = 0;
        self.clock = 0;
        self.rewind();
    }
    pub fn rewind(&mut self) {
        self.next_syllable = 0;
        self.held_syllable = None;
        self.last_onset = None;
        for v in &mut self.voices {
            v.cycle = None;
        }
    }
    pub fn invalidate_phrase(&mut self) {
        for v in &mut self.voices {
            v.cycle = None;
        }
    }
    pub fn motion(&mut self, p: &Controls, performance: &lab::Performance) {
        if p.mode.value() == Mode::Vowels {
            return;
        }
        for v in &mut self.voices {
            if v.key.is_some() {
                if let Some(g) = performance.gesture(p, p.base_artic(&v.vowels)) {
                    v.engine.schedule(&[g], 0., false);
                }
            }
        }
    }
    pub fn remodulate(&mut self, p: &Controls, main: &speech::Bank, layers: &layers::Layers) {
        for (i, v) in self.voices.iter_mut().enumerate() {
            if v.key.is_some() && v.held && p.mode.value() == Mode::Syllables {
                self.scratch.clear();
                self.scratch
                    .extend_from_slice(&bank(i, main, layers).on[v.syllable]);
                p.modulate(&mut self.scratch);
                v.engine.schedule(
                    &self.scratch,
                    -(v.engine.clock() - 256. / self.rate as f64),
                    true,
                );
            }
        }
    }
    pub fn telemetry(&self, shared: &Shared) {
        if let Some(v) = self
            .voices
            .iter()
            .find(|v| v.key.is_some() && v.envelope > 0.)
        {
            for (dest, value) in shared
                .articulation
                .iter()
                .zip(v.engine.voice().artic.x.to_array())
            {
                dest.store((value as f32).to_bits(), Ordering::Relaxed);
            }
        }
    }
    pub fn active(&self) -> usize {
        self.voices.iter().filter(|v| v.key.is_some()).count()
    }
    pub fn event(
        &mut self,
        event: NoteEvent<()>,
        notes: &Notes,
        p: &Controls,
        main: &speech::Bank,
        layers: &layers::Layers,
    ) {
        // Release the original slot, never retune surviving notes when another key is released.
        for (i, v) in self.voices.iter_mut().enumerate() {
            if let Some((ch, n)) = v.key {
                if matches!(event,NoteEvent::Choke{channel,note,..} if channel==ch && note==n)
                    || matches!(event,NoteEvent::MidiCC{channel,cc:120,..} if channel==ch)
                {
                    v.key = None;
                    v.envelope = 0.;
                    v.steal_tail = 0.;
                    continue;
                }
                if v.held && !notes.sounding(ch, n) {
                    v.held = false;
                    if p.mode.value() == Mode::Syllables {
                        v.engine.release(speech::NOTE_TAG);
                        v.engine.release(speech::NOTE_TAG + 1.);
                        self.scratch.clear();
                        self.scratch
                            .extend_from_slice(&bank(i, main, layers).off[v.syllable]);
                        p.modulate(&mut self.scratch);
                        let tail = self.scratch.iter().map(|g| g.end()).fold(0.15, f64::max);
                        v.engine.schedule(&self.scratch, 0., false);
                        v.release_at = v.engine.clock() + tail + 0.15;
                    }
                }
            }
        }
        let NoteEvent::NoteOn {
            channel,
            note,
            velocity,
            ..
        } = event
        else {
            return;
        };
        if channel >= 16 || note >= 128 || velocity <= 0. {
            return;
        }
        let limit = p.lab.voice_limit.value() as usize;
        let count_held = self.voices.iter().filter(|v| v.held).count();
        let index = (0..limit)
            .find(|&i| self.voices[i].key == Some((channel, note)))
            .or_else(|| (0..limit).find(|&i| self.voices[i].key.is_none()))
            .unwrap_or_else(|| {
                (0..limit)
                    .min_by_key(|&i| (self.voices[i].held, self.voices[i].order))
                    .unwrap()
            });
        let mut syllable = 0;
        if p.mode.value() == Mode::Syllables && main.len > 0 {
            let same_chord = self.last_onset == Some(self.clock);
            let hold = p.lab.hold.value() || (p.lab.legato.value() && count_held > 0) || same_chord;
            if hold && self.held_syllable.is_some() {
                syllable = self.held_syllable.unwrap();
            } else {
                syllable = self.next_syllable % main.len;
                self.next_syllable = (syllable + 1) % main.len;
                if main.held[syllable] {
                    syllable = self.held_syllable.unwrap_or(syllable);
                }
                self.held_syllable = Some(syllable);
            }
        }
        self.last_onset = Some(self.clock);
        self.serial += 1;
        let v = &mut self.voices[index];
        let tail = v.last;
        v.engine.reset_human_from(&v.warm);
        v.key = Some((channel, note));
        v.held = true;
        v.velocity = velocity;
        v.envelope = 0.;
        v.age = 0;
        v.order = self.serial;
        v.syllable = syllable;
        v.release_at = 0.;
        v.cycle = None;
        v.steal_tail = tail;
        if p.mode.value() == Mode::Syllables {
            let b = bank(index, main, layers);
            if b.len == 0 || b.rests[syllable] {
                v.key = None;
                v.held = false;
                return;
            }
            self.scratch.clear();
            self.scratch.extend_from_slice(&b.on[syllable]);
            p.modulate(&mut self.scratch);
            v.engine.schedule(&self.scratch, 0., true);
        }
    }
    pub fn update(
        &mut self,
        p: &Controls,
        notes: &Notes,
        performance: &lab::Performance,
        shared: &Shared,
    ) {
        for (i, v) in self.voices.iter_mut().enumerate() {
            if i >= p.lab.voice_limit.value() as usize {
                v.held = false;
                v.release_at = 0.;
            }
            let Some((ch, n)) = v.key else {
                continue;
            };
            let age = v.age as f64 / self.rate as f64;
            let expression = p.lab.expression.value();
            let pressure =
                f32::from_bits(shared.pressure[ch as usize].load(Ordering::Relaxed)) * expression;
            let slide = (f32::from_bits(shared.slide[ch as usize].load(Ordering::Relaxed)) - 0.5)
                * expression;
            let mut a = p.base_artic(&v.vowels);
            performance.articulate(p, &mut a);
            if slide != 0. {
                a.set(
                    ArticKey::TbY,
                    a.get(ArticKey::TbY).unwrap_or(6.85) + slide as f64,
                );
            }
            v.engine
                .set_artic(&a, Some(p.tip_x.value() == 0. && p.tip_y.value() == 0.));

            let detune = p.lab.detune.value() as f64 * ((i % 3) as f64 - 1.) / 100.;
            let semitone = n as f64 - 69.
                + notes.bend[ch as usize] as f64 * p.lab.bend_range.value() as f64
                + detune
                + p.lab.scoop.value() as f64 * (-age / 0.12).exp();
            let bloom = if p.lab.vibrato_delay.value() > 0. {
                (age / p.lab.vibrato_delay.value() as f64).min(1.)
            } else {
                1.
            };
            v.engine.set_source(&SourcePatch {
                f0: Some(440. * 2_f64.powf(semitone / 12.)),
                glide: Some(p.glide.value() as f64 / 1000.),
                pressure: Some(
                    (5. + 5. * v.velocity as f64)
                        * (1. + p.lab.effort.value() as f64 * 0.6 + pressure as f64 * 0.6),
                ),
                breathiness: Some((p.breath.value() + p.lab.air.value() * 0.65).min(1.) as f64),
                brightness: Some(p.brightness.value() as f64),
                tension: Some(p.tension.value() as f64),
                vibrato_depth: Some(
                    (p.vibrato.value() as f64
                        + f32::from_bits(shared.modwheel.load(Ordering::Relaxed)) as f64
                            * expression as f64
                            * 60.)
                        * bloom,
                ),
                vibrato_rate: Some(5.5),
                drift: Some(0.),
                roughness: Some(0.05 + p.lab.rough.value() as f64 * 0.75),
                fry: Some(p.lab.fry.value() as f64),
                ..Default::default()
            });
            v.engine.set_gate(if p.mode.value() == Mode::Vowels {
                1.
            } else {
                0.
            });
        }
    }
    pub fn render(
        &mut self,
        p: &Controls,
        main: &speech::Bank,
        layers: &layers::Layers,
        beat: f64,
        bpm: f64,
        running: bool,
        attack: f32,
        release: f32,
        gain: f32,
    ) -> [f32; 2] {
        let mut out = [0.; 2];
        self.clock += 1;
        if self.revision != main.revision {
            for v in &mut self.voices {
                v.cycle = None;
            }
            self.revision = main.revision;
        }
        for (i, v) in self.voices.iter_mut().enumerate() {
            if v.key.is_none() {
                continue;
            }
            let mode = p.mode.value();
            if mode == Mode::Loop && running {
                let b = bank(i, main, layers);
                let length = b.beats * p.step_beats.value() as f64;
                if length > 0. {
                    let cycle = (beat / length).floor() as i64;
                    if v.cycle != Some(cycle) {
                        speech::scale_phrase(
                            b,
                            60. / bpm * p.step_beats.value() as f64,
                            &mut self.scratch,
                        );
                        p.modulate(&mut self.scratch);
                        v.engine.schedule(
                            &self.scratch,
                            -beat.rem_euclid(length) * 60. / bpm,
                            true,
                        );
                        v.cycle = Some(cycle);
                    }
                }
            } else if mode == Mode::Loop && v.cycle.is_some() {
                v.engine.stop_speech();
                v.cycle = None;
            }
            let sounding = (v.held && (mode != Mode::Loop || running))
                || (mode == Mode::Syllables && v.engine.clock() < v.release_at);
            let target = if sounding { v.velocity } else { 0. };
            v.envelope =
                target + (v.envelope - target) * if target > v.envelope { attack } else { release };
            if !sounding && v.envelope < 1e-5 {
                v.envelope = 0.;
                v.steal_tail = 0.;
                v.last = 0.;
                if !v.held {
                    v.key = None;
                }
                continue;
            }
            let mut b = [0.];
            v.engine.render(&mut b);
            v.age += 1;
            let y = b[0] * v.envelope * gain * 0.45 + v.steal_tail;
            v.steal_tail *= self.steal_decay;
            if v.steal_tail.abs() < 1e-7 {
                v.steal_tail = 0.;
            }
            v.last = y;
            let pan = ((i % 3) as f32 - 1.) * p.lab.spread.value() * 0.5;
            out[0] += y * (1. - pan);
            out[1] += y * (1. + pan);
        }
        out
    }
}

#[cfg(test)]
impl Polyphony {
    pub fn keys(&self) -> [Option<(u8, u8)>; MAX] {
        self.voices.each_ref().map(|v| v.key)
    }
    pub fn syllables(&self) -> [usize; MAX] {
        self.voices.each_ref().map(|v| v.syllable)
    }
}
