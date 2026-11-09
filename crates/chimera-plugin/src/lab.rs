//! Optional performance extensions. Every control defaults to the 0.2 signal path.
use super::*;
use chimera::{
    control::score::{Gesture, Targets},
    phonetics::beatbox::{BEAT_SOUNDS, BeatBuild},
};
use serde::{Deserialize, Serialize};
#[derive(Params)]
pub struct LabControls {
    #[id = "lab_poly"]
    pub poly: BoolParam,
    #[id = "lab_voice_limit"]
    pub voice_limit: IntParam,
    #[id = "lab_air"]
    pub air: FloatParam,
    #[id = "lab_effort"]
    pub effort: FloatParam,
    #[id = "lab_size"]
    pub size: FloatParam,
    #[id = "lab_movement"]
    pub movement: FloatParam,
    #[id = "lab_wet"]
    pub wet: FloatParam,
    #[id = "lab_fry"]
    pub fry: FloatParam,
    #[id = "lab_rough"]
    pub rough: FloatParam,
    #[id = "lab_ventricular"]
    pub ventricular: FloatParam,
    #[id = "lab_suction"]
    pub suction: FloatParam,
    #[id = "lab_curl"]
    pub curl: FloatParam,
    #[id = "lab_lateral"]
    pub lateral: FloatParam,
    #[id = "lab_noise_level"]
    pub noise_level: FloatParam,
    #[id = "lab_noise_decay"]
    pub noise_decay: FloatParam,
    #[id = "lab_noise_tone"]
    pub noise_tone: FloatParam,
    #[id = "lab_motion_length"]
    pub motion_length: FloatParam,
    #[id = "lab_motion_depth"]
    pub motion_depth: FloatParam,
    #[id = "lab_morph_x"]
    pub morph_x: FloatParam,
    #[id = "lab_morph_y"]
    pub morph_y: FloatParam,
    #[id = "lab_morph_depth"]
    pub morph_depth: FloatParam,
    #[id = "lab_voice"]
    pub voice: FloatParam,
    #[id = "lab_choir"]
    pub choir: FloatParam,
    #[id = "lab_spread"]
    pub spread: FloatParam,
    #[id = "lab_detune"]
    pub detune: FloatParam,
    #[id = "lab_harmony"]
    pub harmony: FloatParam,
    #[id = "lab_swing"]
    pub swing: FloatParam,
    #[id = "lab_consonants"]
    pub consonants: FloatParam,
    #[id = "lab_vibrato_delay"]
    pub vibrato_delay: FloatParam,
    #[id = "lab_scoop"]
    pub scoop: FloatParam,
    #[id = "lab_bend_range"]
    pub bend_range: FloatParam,
    #[id = "lab_expression"]
    pub expression: FloatParam,
    #[id = "lab_mic_gain"]
    pub mic_gain: FloatParam,
    #[id = "lab_mic_gate"]
    pub mic_gate: FloatParam,
    #[id = "lab_ui_scale"]
    pub ui_scale: FloatParam,
    #[id = "lab_hold"]
    pub hold: BoolParam,
    #[id = "lab_legato"]
    pub legato: BoolParam,
    #[id = "lab_natural"]
    pub natural: BoolParam,
    #[id = "lab_sequencer"]
    pub sequencer: BoolParam,
    #[id = "lab_mic"]
    pub mic: BoolParam,
    #[persist = "lab-session"]
    pub session: Arc<Mutex<Session>>,
}
impl Default for LabControls {
    fn default() -> Self {
        Self {
            poly: BoolParam::new("Polyphonic choir", false),
            voice_limit: IntParam::new("Voice limit", 4, IntRange::Linear { min: 2, max: 6 }),
            air: control("Air", 0.0, 0.0, 1.0),
            effort: control("Effort", 0.0, 0.0, 1.0),
            size: control("Size", 0.0, -1.0, 1.0),
            movement: control("Movement", 0.0, 0.0, 1.0),
            wet: control("Wet resonance", 0.0, 0.0, 1.0),
            fry: control("Vocal fry", 0.0, 0.0, 1.0),
            rough: control("Throat roughness", 0.0, 0.0, 1.0),
            ventricular: control("False-fold narrowing", 0.0, 0.0, 1.0),
            suction: control("Tongue hollow / suction", 0.0, 0.0, 1.0),
            curl: control("Tongue curl", 0.0, 0.0, 1.0),
            lateral: control("Lateral airflow", 0.0, 0.0, 1.0),
            noise_level: control("Mouth percussion level", 0.0, -0.0, 1.0),
            noise_decay: control("Mouth percussion decay", 0.18, 0.04, 1.0),
            noise_tone: control("Click pitch", 0.5, 0.0, 1.0),
            motion_length: control("Motion loop beats", 4.0, 1.0, 16.0),
            motion_depth: control("Motion playback depth", 0.0, 0.0, 1.0),
            morph_x: control("Mouth morph X", 0.0, 0.0, 1.0),
            morph_y: control("Mouth morph Y", 0.0, 0.0, 1.0),
            morph_depth: control("Mouth morph amount", 0.0, 0.0, 1.0),
            voice: control("Anatomy blend", 0.0, 0.0, 2.0),
            choir: control("Choir blend", 0.0, 0.0, 1.0),
            spread: control("Stereo spread", 0.0, 0.0, 1.0),
            detune: control("Choir detune", 7.0, 0.0, 30.0),
            harmony: control("Harmony interval", 7.0, -24.0, 24.0),
            swing: control("Swing", 0.0, 0.0, 0.6),
            consonants: control("Consonant emphasis", 1.0, 0.5, 2.0),
            vibrato_delay: control("Vibrato bloom time", 0.0, 0.0, 1.5),
            scoop: control("Pitch scoop", 0.0, -12.0, 12.0),
            bend_range: control("Pitch bend range", 2.0, 1.0, 48.0),
            expression: control("Expression response", 0.0, 0.0, 1.0),
            mic_gain: control("Microphone sensitivity", 1.0, 0.1, 8.0),
            mic_gate: control("Microphone threshold", 0.015, 0.001, 0.2),
            ui_scale: control("Interface scale", 1.0, 0.7, 1.4),
            hold: BoolParam::new("Hold syllable", false),
            legato: BoolParam::new("Legato holds syllable", false),
            natural: BoolParam::new("Natural lyric timing", false),
            sequencer: BoolParam::new("Mouth sequencer", false),
            mic: BoolParam::new("Microphone pitch + breath", false),
            session: Arc::new(Mutex::new(Session::default())),
        }
    }
}

pub const MOTION_FRAMES: usize = 256;
pub const POSE_KEYS: [ArticKey; 11] = [
    ArticKey::TbX,
    ArticKey::TbY,
    ArticKey::TtX,
    ArticKey::TtY,
    ArticKey::Jaw,
    ArticKey::LipAp,
    ArticKey::LipProt,
    ArticKey::Larynx,
    ArticKey::Root,
    ArticKey::Epilarynx,
    ArticKey::Velum,
];
pub type Pose = [f32; 11];
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub motion: Vec<Pose>,
    pub shapes: [Pose; 4],
    pub shape_set: [bool; 4],
    pub steps: [u8; 16], // 0 is rest; 1..10 selects the upstream mouth gesture
    pub accents: [f32; 16],
    pub durations: [f32; 32],
    pub rests: [bool; 32],
    pub pronunciation: Vec<String>,
    pub preset_name: String,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            motion: Vec::new(),
            shapes: [[0.; 11]; 4],
            shape_set: [false; 4],
            steps: [0; 16],
            accents: [1.; 16],
            durations: [1.; 32],
            rests: [false; 32],
            pronunciation: vec![String::new(); 32],
            preset_name: "Classic 0.2".into(),
        }
    }
}
impl Session {
    pub fn sanitize(&mut self) {
        self.motion.truncate(MOTION_FRAMES);
        for pose in self.motion.iter_mut().chain(self.shapes.iter_mut()) {
            for v in pose {
                *v = if v.is_finite() { v.clamp(-3., 3.) } else { 0. };
            }
        }
        for v in &mut self.steps {
            *v = (*v).min(10);
        }
        for v in &mut self.accents {
            *v = if v.is_finite() { v.clamp(0.5, 1.5) } else { 1. };
        }
        for v in &mut self.durations {
            *v = if v.is_finite() { v.clamp(0.25, 8.) } else { 1. };
        }
        self.pronunciation.resize(32, String::new());
        self.pronunciation.truncate(32);
        for s in &mut self.pronunciation {
            *s = s.chars().take(48).collect();
        }
    }
}
pub fn pose(p: &Controls) -> Pose {
    let mut out = [0.; 11];
    for (i, (_, v)) in p.offsets().iter().enumerate() {
        out[i] = *v as f32;
    }
    out[10] = p.nasal.value();
    out
}
pub fn morph(shapes: &[Pose; 4], x: f32, y: f32) -> Pose {
    let w = [(1. - x) * (1. - y), x * (1. - y), (1. - x) * y, x * y];
    std::array::from_fn(|i| (0..4).map(|j| shapes[j][i] * w[j]).sum())
}
pub struct Performance {
    pub motion: [Pose; MOTION_FRAMES],
    pub motion_len: usize,
    pub shapes: [Pose; 4],
    pub steps: [u8; 16],
    pub accents: [f32; 16],
    pub revision: u64,
    pub delta: Pose,
    pub beat: f64,
    pub age: f64,
    pub last_step: i64,
    pub noise: Engine,
    pub noise_tail: f64,
    warm_noise: chimera::voice::human_voice::HumanVoice,
    pub noise_scratch: Vec<Gesture>,
    delay: Vec<f32>,
    delay_pos: usize,
    lowpass: f32,
    pub mic: PitchFollower,
}
impl Performance {
    pub fn new(rate: f32) -> Self {
        let mut noise = Engine::new(rate as f64);
        noise.render(&mut [0.; 256]);
        Self {
            motion: [[0.; 11]; MOTION_FRAMES],
            motion_len: 0,
            shapes: [[0.; 11]; 4],
            steps: [0; 16],
            accents: [1.; 16],
            revision: 0,
            delta: [0.; 11],
            beat: 0.,
            age: 0.,
            last_step: i64::MIN,
            warm_noise: noise.voice().clone(),
            noise,
            noise_tail: 0.,
            noise_scratch: Vec::with_capacity(64),
            delay: vec![0.; (rate * 0.045) as usize + 4],
            delay_pos: 0,
            lowpass: 0.,
            mic: PitchFollower::new(rate),
        }
    }
    pub fn sync(&mut self, s: &Session, revision: u64) {
        self.motion_len = s.motion.len().min(MOTION_FRAMES);
        self.motion[..self.motion_len].copy_from_slice(&s.motion[..self.motion_len]);
        self.shapes = s.shapes;
        self.steps = s.steps.map(|x| x.min(10));
        self.accents = s.accents.map(|x| x.clamp(0.5, 1.5));
        self.revision = revision;
    }
    pub fn reset(&mut self) {
        self.noise.reset_human_from(&self.warm_noise);
        self.noise_tail = 0.;
        self.delta = [0.; 11];
        self.last_step = i64::MIN;
        self.age = 0.;
        self.delay.fill(0.);
        self.lowpass = 0.;
        self.mic.reset();
    }
    pub fn update(&mut self, p: &Controls, beat: f64) {
        self.beat = beat;
        let l = &p.lab;
        let base = pose(p);
        self.delta = [0.; 11];
        if self.motion_len > 1 && l.motion_depth.value() > 0. {
            let phase = beat.rem_euclid(l.motion_length.value() as f64)
                / l.motion_length.value() as f64
                * self.motion_len as f64;
            let a = phase.floor() as usize % self.motion_len;
            let b = (a + 1) % self.motion_len;
            let f = phase.fract() as f32;
            for i in 0..11 {
                self.delta[i] += (self.motion[a][i] * (1. - f) + self.motion[b][i] * f - base[i])
                    * l.motion_depth.value();
            }
        }
        if l.morph_depth.value() > 0. {
            let m = morph(&self.shapes, l.morph_x.value(), l.morph_y.value());
            for i in 0..11 {
                self.delta[i] += (m[i] - base[i]) * l.morph_depth.value();
            }
        }
        let phase = std::f64::consts::TAU * beat / 2.;
        let movement = l.movement.value();
        self.delta[0] += phase.sin() as f32 * movement * 0.55;
        self.delta[1] += (phase * 0.5).cos() as f32 * movement * 0.45;
        self.delta[5] += (phase * 2.).sin() as f32 * movement * 0.25;
        self.delta[7] -= l.size.value() * 1.2;
    }
    pub fn articulate(&self, p: &Controls, patch: &mut ArticPatch) {
        for (i, k) in POSE_KEYS.iter().enumerate() {
            let default = chimera::anatomy::human::NEUTRAL.get(*k);
            patch.set(*k, patch.get(*k).unwrap_or(default) + self.delta[i] as f64);
        }
        patch.set(ArticKey::Ventricular, p.lab.ventricular.value() as f64);
        patch.set(ArticKey::Dorsum, p.lab.suction.value() as f64);
        patch.set(ArticKey::TtCurl, p.lab.curl.value() as f64);
        patch.set(ArticKey::Lateral, p.lab.lateral.value() as f64);
    }
    pub fn gesture(&self, p: &Controls, base: ArticPatch) -> Option<Gesture> {
        let l = &p.lab;
        if self.delta.iter().all(|x| x.abs() < 1e-6)
            && l.ventricular.value() == 0.
            && l.suction.value() == 0.
            && l.curl.value() == 0.
            && l.lateral.value() == 0.
        {
            return None;
        }
        let mut a = base;
        self.articulate(p, &mut a);
        let mut targets = Targets::default();
        for (i, k) in POSE_KEYS.iter().enumerate() {
            if self.delta[i].abs() > 1e-6 {
                targets.set(*k, a.get(*k).unwrap());
            }
        }
        for (k, v) in [
            (ArticKey::Ventricular, l.ventricular.value()),
            (ArticKey::Dorsum, l.suction.value()),
            (ArticKey::TtCurl, l.curl.value()),
            (ArticKey::Lateral, l.lateral.value()),
        ] {
            if v > 0. {
                targets.set(k, v as f64);
            }
        }
        Some(Gesture::new(0., 0.018, 0.003, 0.006, 35., targets).label("motion"))
    }
    pub fn trigger(&mut self, index: usize, accent: f32, p: &Controls) {
        let Some(sound) = BEAT_SOUNDS.get(index) else {
            return;
        };
        self.noise_scratch.clear();
        (sound.build)(
            BodyMix::MALE,
            &BeatBuild {
                f0: 60. + p.lab.noise_tone.value() as f64 * 300.,
                len: p.lab.noise_decay.value() as f64,
                accent: accent.clamp(0.5, 1.5) as f64,
            },
            &mut self.noise_scratch,
        );
        let start = self.noise_scratch.iter().map(|g| g.t0).fold(0., f64::min);
        let stretch = 0.7 + p.lab.wet.value() as f64 * 0.8;
        for g in &mut self.noise_scratch {
            g.t0 = (g.t0 - start) * stretch;
            g.t1 = (g.t1 - start) * stretch;
            g.ramp_in *= stretch;
            g.ramp_out *= stretch;
        }
        self.noise.set_artic(
            &chimera::phonetics::beatbox::beat_ready(BodyMix::MALE),
            Some(false),
        );
        self.noise.set_source(&SourcePatch {
            pressure: Some(8. + p.lab.effort.value() as f64 * 5.),
            roughness: Some(p.lab.rough.value() as f64),
            ..Default::default()
        });
        self.noise.set_gate(0.);
        self.noise.schedule(&self.noise_scratch, 0., false);
        self.noise_tail = self.noise.clock()
            + self
                .noise_scratch
                .iter()
                .map(Gesture::end)
                .fold(0.2, f64::max)
            + 0.35;
    }
    pub fn sequence(&mut self, beat: f64, p: &Controls) {
        let swing = p.lab.swing.value() as f64;
        let pair = (beat * 2.).floor();
        let local = beat * 2. - pair;
        let step = (pair * 2.) as i64 + if local >= (1. + swing) / 2. { 1 } else { 0 };
        if step != self.last_step {
            self.last_step = step;
            let i = step.rem_euclid(16) as usize;
            let hit = self.steps[i];
            if hit > 0 {
                self.trigger((hit - 1) as usize, self.accents[i], p);
            }
        }
    }
    pub fn render_noise(&mut self) -> f32 {
        if self.noise.clock() >= self.noise_tail {
            return 0.;
        }
        let mut b = [0.];
        self.noise.render(&mut b);
        b[0]
    }
    pub fn wet(&mut self, x: f32, amount: f32, tone: f32) -> f32 {
        if amount <= 0. {
            return x;
        }
        let delay = (self.delay.len() as f32 * (0.18 + 0.65 * tone)) as usize;
        let read = (self.delay_pos + self.delay.len() - delay) % self.delay.len();
        let d = self.delay[read];
        self.lowpass += 0.22 * (d - self.lowpass);
        self.delay[self.delay_pos] = (x + self.lowpass * amount * 0.65).tanh();
        self.delay_pos = (self.delay_pos + 1) % self.delay.len();
        x * (1. - 0.22 * amount) + self.lowpass * amount * 0.45
    }
}
/// Bounded autocorrelation on a decimated microphone signal; no heap work in process.
pub struct PitchFollower {
    ring: [f32; 1024],
    pos: usize,
    count: usize,
    decimate: usize,
    phase: usize,
    rate: f32,
    pub level: f32,
    pub hz: f32,
    pub confidence: f32,
}
impl PitchFollower {
    pub fn new(rate: f32) -> Self {
        let d = (rate / 8000.).round().max(1.) as usize;
        Self {
            ring: [0.; 1024],
            pos: 0,
            count: 0,
            decimate: d,
            phase: 0,
            rate: rate / d as f32,
            level: 0.,
            hz: 110.,
            confidence: 0.,
        }
    }
    pub fn reset(&mut self) {
        self.ring.fill(0.);
        self.pos = 0;
        self.count = 0;
        self.phase = 0;
        self.level = 0.;
        self.hz = 110.;
        self.confidence = 0.;
    }
    pub fn push(&mut self, x: f32) {
        let x = if x.is_finite() { x.clamp(-2., 2.) } else { 0. };
        self.level = self.level * 0.999 + x.abs() * 0.001;
        self.phase += 1;
        if self.phase < self.decimate {
            return;
        }
        self.phase = 0;
        self.ring[self.pos] = x;
        self.pos = (self.pos + 1) % 1024;
        self.count += 1;
        if self.count < 640 || self.count % 128 != 0 {
            return;
        }
        let at = |offset: usize| self.ring[(self.pos + 1024 - 1 - offset) % 1024];
        let mut best = 0.;
        let mut lag_best = 0;
        for lag in (self.rate / 700.) as usize..=(self.rate / 65.) as usize {
            let mut corr = 0.;
            let mut a = 0.;
            let mut b = 0.;
            for i in 0..384 {
                let x = at(i);
                let y = at(i + lag);
                corr += x * y;
                a += x * x;
                b += y * y;
            }
            let score = corr / (a * b + 1e-16).sqrt();
            if score > best + 0.004 {
                best = score;
                lag_best = lag;
            }
        }
        self.confidence = best;
        if best > 0.78 && lag_best > 0 {
            let next = self.rate / lag_best as f32;
            self.hz = if (next / self.hz).ln().abs() > 0.3 {
                next
            } else {
                self.hz * 0.6 + next * 0.4
            };
        }
    }
}
