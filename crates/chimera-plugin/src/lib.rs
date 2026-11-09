//! Native MIDI instrument adapter. Planning and body reconstruction never run in process().
mod float_mode;
mod lab;
mod layers;
mod midi;
mod poly;
mod presets;
mod speech;
#[cfg(test)]
mod tests;
mod ui;
use chimera::{
    anatomy::human::{ArticKey, ArticPatch, BodyMix},
    engine::Engine,
    phonetics::planner::vowel_artic,
    voice::human_voice::SourcePatch,
};
use midi::Notes;
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
};

#[derive(Enum, PartialEq, Debug, Clone, Copy)]
enum Mode {
    #[name = "Vowels"]
    Vowels,
    #[name = "Phrase loop"]
    Loop,
    #[name = "MIDI syllables"]
    Syllables,
}
#[derive(Clone, Copy)]
pub enum Task {
    Compile,
    SaveMotion,
}
struct Shared {
    bank: Mutex<speech::Bank>,
    revision: AtomicU64,
    error: Mutex<String>,
    preview: AtomicBool,
    rewind: AtomicBool,
    local_keys: AtomicU32,
    local_base: AtomicU32,
    bpm: AtomicU32,
    active_syllable: AtomicUsize,
    phase: AtomicU32,
    articulation: [AtomicU32; 17],
    layer_articulation: [[AtomicU32; 17]; 2],
    level: AtomicU32,
    pitch: AtomicU32,
    sounding: AtomicBool,
    layer_banks: Mutex<[speech::Bank; 2]>,
    session_revision: AtomicU64,
    motion_state: AtomicU32,
    recorded: [[AtomicU32; 11]; 256],
    hit: AtomicU32,
    beat: AtomicU32,
    playing: AtomicBool,
    mic_level: AtomicU32,
    mic_hz: AtomicU32,
    mic_confidence: AtomicU32,
    pressure: [AtomicU32; 16],
    slide: [AtomicU32; 16],
    modwheel: AtomicU32,
    compile_requested: AtomicBool,
    compile_busy: AtomicBool,
    cpu_load: AtomicU32,
    voices: AtomicUsize,
}
impl Default for Shared {
    fn default() -> Self {
        Self {
            bank: Mutex::new(speech::Bank::new(false)),
            revision: AtomicU64::new(0),
            error: Mutex::new(String::new()),
            preview: AtomicBool::new(false),
            rewind: AtomicBool::new(false),
            local_keys: AtomicU32::new(0),
            local_base: AtomicU32::new(48),
            bpm: AtomicU32::new(120.0_f32.to_bits()),
            active_syllable: AtomicUsize::new(usize::MAX),
            phase: AtomicU32::new(0),
            articulation: std::array::from_fn(|i| {
                AtomicU32::new((chimera::anatomy::human::NEUTRAL.to_array()[i] as f32).to_bits())
            }),
            layer_articulation: std::array::from_fn(|_| {
                std::array::from_fn(|i| {
                    AtomicU32::new(
                        (chimera::anatomy::human::NEUTRAL.to_array()[i] as f32).to_bits(),
                    )
                })
            }),
            level: AtomicU32::new(0),
            pitch: AtomicU32::new(110.0_f32.to_bits()),
            sounding: AtomicBool::new(false),
            layer_banks: Mutex::new(std::array::from_fn(|_| speech::Bank::new(false))),
            session_revision: AtomicU64::new(1),
            motion_state: AtomicU32::new(0),
            recorded: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU32::new(0))),
            hit: AtomicU32::new(0),
            beat: AtomicU32::new(0),
            playing: AtomicBool::new(false),
            mic_level: AtomicU32::new(0),
            mic_hz: AtomicU32::new(110_f32.to_bits()),
            mic_confidence: AtomicU32::new(0),
            pressure: std::array::from_fn(|_| AtomicU32::new(0)),
            slide: std::array::from_fn(|_| AtomicU32::new(0.5_f32.to_bits())),
            modwheel: AtomicU32::new(0),
            compile_requested: AtomicBool::new(false),
            compile_busy: AtomicBool::new(false),
            cpu_load: AtomicU32::new(0),
            voices: AtomicUsize::new(0),
        }
    }
}
fn publish_text(params: &Controls, shared: &Shared) {
    let text = params.lyrics.lock().unwrap().clone();
    let mut saved = params.lab.session.lock().unwrap();
    saved.sanitize();
    let session = saved.clone();
    drop(saved);
    let bpm = f32::from_bits(shared.bpm.load(Ordering::Relaxed)) as f64;
    let compile = |body| {
        speech::compile_for(
            &text,
            body,
            &session,
            params.lab.natural.value(),
            bpm,
            params.step_beats.value() as f64,
            params.lab.swing.value() as f64,
            params.lab.consonants.value() as f64,
        )
    };
    match (
        compile(BodyMix::MALE),
        compile(BodyMix::FEMALE),
        compile(BodyMix::CHILD),
    ) {
        (Ok(mut bank), Ok(a), Ok(b)) => {
            let mut dest = shared.bank.lock().unwrap();
            let mut layer = shared.layer_banks.lock().unwrap();
            let revision = shared.revision.load(Ordering::Relaxed) + 1;
            bank.revision = revision;
            *dest = bank;
            *layer = [a, b];
            shared.revision.store(revision, Ordering::Release);
            shared.error.lock().unwrap().clear();
        }
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
            *shared.error.lock().unwrap() = error
        }
    }
}

#[derive(Params)]
struct Controls {
    #[nested]
    lab: lab::LabControls,
    #[persist = "editor-v2"]
    editor: Arc<EguiState>,
    #[persist = "lyrics"]
    lyrics: Arc<Mutex<String>>,
    #[id = "mode"]
    mode: EnumParam<Mode>,
    #[id = "stepbeats"]
    step_beats: FloatParam,
    #[id = "rootnote"]
    root_note: IntParam,
    #[id = "tonguex"]
    tongue_x: FloatParam,
    #[id = "tonguey"]
    tongue_y: FloatParam,
    #[id = "tipx"]
    tip_x: FloatParam,
    #[id = "tipy"]
    tip_y: FloatParam,
    #[id = "jaw"]
    jaw: FloatParam,
    #[id = "lips"]
    lips: FloatParam,
    #[id = "lipprot"]
    lip_prot: FloatParam,
    #[id = "larynx"]
    larynx: FloatParam,
    #[id = "root"]
    root: FloatParam,
    #[id = "twang"]
    twang: FloatParam,
    #[id = "vowel"]
    vowel: FloatParam,
    #[id = "breath"]
    breath: FloatParam,
    #[id = "brightness"]
    brightness: FloatParam,
    #[id = "tension"]
    tension: FloatParam,
    #[id = "nasal"]
    nasal: FloatParam,
    #[id = "vibrato"]
    vibrato: FloatParam,
    #[id = "glide"]
    glide: FloatParam,
    #[id = "attack"]
    attack: FloatParam,
    #[id = "release"]
    release: FloatParam,
    #[id = "gain"]
    gain: FloatParam,
}
fn control(name: &str, value: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, value, FloatRange::Linear { min, max })
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            lab: lab::LabControls::default(),
            editor: EguiState::from_size(1020, 760),
            lyrics: Arc::new(Mutex::new("hello from the other side".to_owned())),
            mode: EnumParam::new("Mode", Mode::Vowels),
            step_beats: control("Beats / syllable", 1.0, 0.25, 4.0).with_step_size(0.25),
            root_note: IntParam::new("Phrase pitch", 48, IntRange::Linear { min: 24, max: 84 }),
            tongue_x: control("Tongue front / back", 0.0, -2.0, 2.0),
            tongue_y: control("Tongue height", 0.0, -2.0, 2.0),
            tip_x: control("Tongue tip front", 0.0, -2.0, 2.0),
            tip_y: control("Tongue tip height", 0.0, -2.0, 2.0),
            jaw: control("Jaw opening", 0.0, -1.0, 1.5),
            lips: control("Lip opening", 0.0, -1.0, 1.5),
            lip_prot: control("Lip protrusion", 0.0, -0.5, 1.0),
            larynx: control("Larynx", 0.0, -1.6, 1.6),
            root: control("Tongue root", 0.0, -1.0, 1.0),
            twang: control("Throat narrowing", 0.0, 0.0, 1.0),
            vowel: control("Vowel morph", 0.0, 0.0, 4.0).with_value_to_string(Arc::new(|v| {
                let names = ["AH", "EH", "EE", "OH", "OO"];
                let i = (v.floor() as usize).min(4);
                if i == 4 || v.fract() < 0.01 {
                    names[i].to_owned()
                } else {
                    format!("{} → {} {:.0}%", names[i], names[i + 1], v.fract() * 100.0)
                }
            })),
            breath: control("Breathiness", 0.15, 0.0, 1.0),
            brightness: control("Brightness", 0.25, 0.0, 1.0),
            tension: control("Tension", 0.5, 0.0, 1.0),
            nasal: control("Nasality", 0.0, 0.0, 1.0),
            vibrato: control("Vibrato depth", 15.0, 0.0, 100.0).with_unit(" cents"),
            glide: control("Glide", 35.0, 0.0, 300.0).with_unit(" ms"),
            attack: control("Attack", 10.0, 1.0, 500.0).with_unit(" ms"),
            release: control("Release", 120.0, 5.0, 1500.0).with_unit(" ms"),
            gain: control("Output", -6.0, -36.0, 12.0)
                .with_unit(" dB")
                .with_smoother(SmoothingStyle::Linear(20.0)),
        }
    }
}

impl Controls {
    fn offsets(&self) -> [(ArticKey, f64); 10] {
        [
            (ArticKey::TbX, self.tongue_x.value() as f64),
            (ArticKey::TbY, self.tongue_y.value() as f64),
            (ArticKey::TtX, self.tip_x.value() as f64),
            (ArticKey::TtY, self.tip_y.value() as f64),
            (ArticKey::Jaw, self.jaw.value() as f64),
            (ArticKey::LipAp, self.lips.value() as f64),
            (ArticKey::LipProt, self.lip_prot.value() as f64),
            (ArticKey::Larynx, self.larynx.value() as f64),
            (ArticKey::Root, self.root.value() as f64),
            (ArticKey::Epilarynx, self.twang.value() as f64),
        ]
    }
    fn base_artic(&self, vowels: &[ArticPatch; 5]) -> ArticPatch {
        let v = self.vowel.value().clamp(0.0, 4.0);
        let i = v.floor() as usize;
        let mut a = ArticPatch::blend(&[
            (1.0 - v.fract() as f64, vowels[i]),
            (v.fract() as f64, vowels[(i + 1).min(4)]),
        ]);
        for (key, offset) in self.offsets() {
            let base = a
                .get(key)
                .unwrap_or_else(|| chimera::anatomy::human::NEUTRAL.get(key));
            a.set(key, base + offset);
        }
        a.set(ArticKey::Velum, self.nasal.value() as f64 * 1.15 - 0.15);
        a
    }
    fn modulate(&self, gestures: &mut [chimera::control::score::Gesture]) {
        let offsets = self.offsets();
        for g in gestures {
            for (key, offset) in offsets {
                if let Some(value) = g.targets.get(key) {
                    g.targets.set(key, value + offset);
                }
            }
            if let Some(value) = g.targets.get(ArticKey::Velum) {
                g.targets
                    .set(ArticKey::Velum, value + self.nasal.value() as f64);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Playback {
    playing: bool,
    bpm: f64,
    beat: Option<f64>,
}
pub struct ChimeraPlugin {
    pending_tasks: u8,
    params: Arc<Controls>,
    engine: Option<Engine>,
    vowels: [ArticPatch; 5],
    notes: Notes,
    envelope: f32,
    sample_rate: f32,
    panic: Arc<AtomicBool>,
    shared: Arc<Shared>,
    bank: speech::Bank,
    scratch: Vec<chimera::control::score::Gesture>,
    next_syllable: usize,
    current_syllable: usize,
    held_syllable: Option<usize>,
    live_on: bool,
    live_start: f64,
    tail_end: f64,
    loop_running: bool,
    loop_cycle: Option<i64>,
    expected_beat: Option<f64>,
    previous_tempo: f64,
    previous_step: f64,
    previous_mode: Mode,
    previous_offsets: [f64; 11],
    preview_beat: f64,
    key_mask: u32,
    key_base: u8,
    telemetry_count: usize,
    panic_latched: bool,
    performance: Option<lab::Performance>,
    layers: Option<layers::Layers>,
    poly: Option<poly::Polyphony>,
    previous_poly: bool,
    warm_voice: Option<chimera::voice::human_voice::HumanVoice>,
    warm_layers: Option<[chimera::voice::human_voice::HumanVoice; 2]>,
    control_counter: usize,
    record_start: Option<f64>,
    record_index: usize,
    stereo: [f32; 2],
    planning_signature: [u32; 5],
    previous_mix: [f32; 2],
}
impl Default for ChimeraPlugin {
    fn default() -> Self {
        Self {
            pending_tasks: 0,
            params: Arc::new(Controls::default()),
            engine: None,
            vowels: [ArticPatch::default(); 5],
            notes: Notes::default(),
            envelope: 0.0,
            sample_rate: 48000.0,
            panic: Arc::new(AtomicBool::new(false)),
            shared: Arc::new(Shared::default()),
            bank: speech::Bank::new(true),
            scratch: Vec::with_capacity(speech::MAX_GESTURES),
            next_syllable: 0,
            current_syllable: 0,
            held_syllable: None,
            live_on: false,
            live_start: 0.0,
            tail_end: 0.0,
            loop_running: false,
            loop_cycle: None,
            expected_beat: None,
            previous_tempo: 120.0,
            previous_step: 1.0,
            previous_mode: Mode::Vowels,
            previous_offsets: [0.0; 11],
            preview_beat: 0.0,
            key_mask: 0,
            key_base: 48,
            telemetry_count: 0,
            panic_latched: false,
            performance: None,
            layers: None,
            poly: None,
            previous_poly: false,
            warm_voice: None,
            warm_layers: None,
            control_counter: 0,
            record_start: None,
            record_index: 0,
            stereo: [0.; 2],
            planning_signature: [0; 5],
            previous_mix: [0.; 2],
        }
    }
}
impl ChimeraPlugin {
    fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let mut engine = Engine::new(sample_rate as f64);
        // The first control steps grow internal scratch buffers: warm them before audio starts.
        engine.render(&mut [0.0; 256]);
        self.warm_voice = Some(engine.voice().clone());
        self.engine = Some(engine);
        self.performance = Some(lab::Performance::new(sample_rate));
        self.layers = Some(layers::Layers::new(sample_rate));
        self.poly = Some(poly::Polyphony::new(sample_rate));
        self.warm_layers = Some(std::array::from_fn(|i| {
            self.layers.as_ref().unwrap().engines[i].voice().clone()
        }));
        for (i, vowel) in ["ɑ", "ɛ", "i", "o", "u"].iter().enumerate() {
            self.vowels[i] = vowel_artic(vowel, BodyMix::MALE).expect("bundled vowel");
        }
        publish_text(&self.params, &self.shared);
        self.sync_bank();
        self.reset_notes();
    }
    fn sync_bank(&mut self) -> bool {
        if self.shared.revision.load(Ordering::Acquire) == self.bank.revision {
            return false;
        }
        if let Ok(bank) = self.shared.bank.try_lock() {
            let Ok(layer) = self.shared.layer_banks.try_lock() else {
                return false;
            };
            let changed = self.bank.labels != bank.labels || self.bank.len != bank.len;
            self.bank.copy_from(&bank);
            if let Some(layers) = &mut self.layers {
                for i in 0..2 {
                    layers.banks[i].copy_from(&layer[i]);
                }
            }
            if changed {
                self.next_syllable = 0;
            }
            self.loop_cycle = None;
            return changed;
        }
        false
    }
    fn reset_notes(&mut self) {
        if let Some(poly) = &mut self.poly {
            poly.reset();
        }
        self.notes = Notes::default();
        self.shared
            .pressure
            .iter()
            .for_each(|x| x.store(0, Ordering::Relaxed));
        self.shared
            .slide
            .iter()
            .for_each(|x| x.store(0.5_f32.to_bits(), Ordering::Relaxed));
        self.shared.modwheel.store(0, Ordering::Relaxed);
        if let Some(l) = &mut self.layers {
            l.reset();
        }
        if let Some(p) = &mut self.performance {
            p.reset();
        }
        self.stereo = [0.; 2];
        self.envelope = 0.0;
        self.live_on = false;
        self.tail_end = 0.0;
        self.loop_running = false;
        self.loop_cycle = None;
        self.expected_beat = None;
        self.next_syllable = 0;
        self.held_syllable = None;
        self.key_mask = 0;
        self.shared.local_keys.store(0, Ordering::Relaxed);
        self.shared
            .active_syllable
            .store(usize::MAX, Ordering::Relaxed);
        if let Some(engine) = &mut self.engine {
            engine.set_gate(0.0);
            engine.stop_speech();
        }
    }
    fn update_voice(&mut self) {
        if self.params.lab.poly.value() {
            if let (Some(poly), Some(performance)) = (&mut self.poly, &self.performance) {
                poly.update(&self.params, &self.notes, performance, &self.shared);
            }
            return;
        }
        let Some(engine) = &mut self.engine else {
            return;
        };
        let p = &self.params;
        let mut artic = p.base_artic(&self.vowels);
        if let Some(performance) = &self.performance {
            performance.articulate(p, &mut artic);
        }
        let channel = self.notes.active().map_or(0, |n| n.0 as usize);
        let expression = p.lab.expression.value();
        let pressure =
            f32::from_bits(self.shared.pressure[channel].load(Ordering::Relaxed)) * expression;
        let slide =
            (f32::from_bits(self.shared.slide[channel].load(Ordering::Relaxed)) - 0.5) * expression;
        if slide != 0. {
            artic.set(
                ArticKey::TbY,
                artic.get(ArticKey::TbY).unwrap_or(6.85) + slide as f64,
            );
        }

        let tip_free = p.tip_x.value() == 0.0 && p.tip_y.value() == 0.0;
        engine.set_artic(&artic, Some(tip_free));
        let active = self.notes.active();
        let (frequency, velocity) = active
            .map(|(channel, note, vel)| {
                let semitone = note as f64 - 69.0
                    + self.notes.bend[channel as usize] as f64 * p.lab.bend_range.value() as f64;
                (440.0 * 2.0_f64.powf(semitone / 12.0), vel)
            })
            .unwrap_or((
                440.0 * 2.0_f64.powf((p.root_note.value() as f64 - 69.0) / 12.0),
                0.85,
            ));
        let age = self.performance.as_ref().map_or(0., |v| v.age);
        let mic_active = p.lab.mic.value()
            && self.performance.as_ref().is_some_and(|v| {
                v.mic.level * p.lab.mic_gain.value() > p.lab.mic_gate.value()
                    && v.mic.confidence > 0.78
            });
        let frequency = if mic_active {
            self.performance.as_ref().unwrap().mic.hz as f64
        } else {
            frequency
        };
        let frequency =
            frequency * 2.0_f64.powf(p.lab.scoop.value() as f64 * (-age / 0.12).exp() / 12.);
        let vibrato_factor = if p.lab.vibrato_delay.value() > 0. {
            (age / p.lab.vibrato_delay.value() as f64).clamp(0., 1.)
        } else {
            1.
        };
        let source = SourcePatch {
            // Preserve the sung pitch through note-off consonants and release.
            f0: (active.is_some() || mic_active || p.mode.value() == Mode::Loop)
                .then_some(frequency),
            glide: Some(p.glide.value() as f64 / 1000.0),
            pressure: Some(
                (5.0 + 5.0 * velocity as f64)
                    * (1. + p.lab.effort.value() as f64 * 0.6 + pressure as f64 * 0.6),
            ),
            breathiness: Some((p.breath.value() + p.lab.air.value() * 0.65).min(1.) as f64),
            brightness: Some(p.brightness.value() as f64),
            tension: Some(p.tension.value() as f64),
            vibrato_depth: Some(
                (p.vibrato.value() as f64
                    + f32::from_bits(self.shared.modwheel.load(Ordering::Relaxed)) as f64
                        * expression as f64
                        * 60.)
                    * vibrato_factor,
            ),
            vibrato_rate: Some(5.5),
            drift: Some(0.0),
            roughness: Some(0.05 + p.lab.rough.value() as f64 * 0.75),
            fry: Some(p.lab.fry.value() as f64),
            ..Default::default()
        };
        engine.set_source(&source);
        // Keep breath flowing through our output release envelope; close it once silent.
        let gate = if p.mode.value() == Mode::Vowels
            && (active.is_some() || mic_active || self.envelope > 0.00001)
        {
            1.
        } else {
            0.
        };
        engine.set_gate(gate);
        if let (Some(l), Some(performance)) = (&mut self.layers, &self.performance) {
            l.update(p, &source, &self.notes, gate, performance, &self.shared);
        }
    }

    fn schedule_live(&mut self, restart: bool) {
        if self.bank.len == 0 {
            return;
        }
        let Some(e) = &mut self.engine else {
            return;
        };
        if restart {
            self.live_start = e.clock();
        }
        let elapsed = e.clock() - self.live_start;
        self.scratch.clear();
        self.scratch
            .extend_from_slice(&self.bank.on[self.current_syllable]);
        self.params.modulate(&mut self.scratch);
        e.schedule(&self.scratch, -elapsed, true);
        if let Some(l) = &mut self.layers {
            l.live(self.current_syllable, elapsed, &self.params);
        }
        self.live_on = true;
        self.shared
            .active_syllable
            .store(self.current_syllable, Ordering::Relaxed);
    }
    fn handle_note(&mut self, event: NoteEvent<()>) {
        if let Some(c) = event.channel().filter(|c| *c < 16) {
            match event {
                NoteEvent::MidiChannelPressure { pressure, .. }
                | NoteEvent::PolyPressure { pressure, .. } => self.shared.pressure[c as usize]
                    .store(pressure.clamp(0., 1.).to_bits(), Ordering::Relaxed),
                NoteEvent::MidiCC { cc: 74, value, .. } => self.shared.slide[c as usize]
                    .store(value.clamp(0., 1.).to_bits(), Ordering::Relaxed),
                NoteEvent::MidiCC { cc: 1, value, .. } => self
                    .shared
                    .modwheel
                    .store(value.clamp(0., 1.).to_bits(), Ordering::Relaxed),
                _ => {}
            }
        }
        if let NoteEvent::NoteOn { note, velocity, .. } = event {
            if velocity > 0. {
                if let Some(p) = &mut self.performance {
                    p.age = 0.;
                }
                if (24..34).contains(&note) && self.params.lab.noise_level.value() > 0. {
                    if let Some(p) = &mut self.performance {
                        p.trigger((note - 24) as usize, 0.5 + velocity, &self.params);
                    }
                    return;
                }
            }
        }

        let previous = self.notes.active();
        self.notes.handle(event);
        if matches!(event, NoteEvent::NoteOn {velocity, ..} if velocity>0.0) {
            self.panic_latched = false;
        }
        if self.params.lab.poly.value() {
            if let (Some(poly), Some(layers)) = (&mut self.poly, &self.layers) {
                poly.event(event, &self.notes, &self.params, &self.bank, layers);
            }
            return;
        }
        if self.params.mode.value() != Mode::Syllables || self.bank.len == 0 {
            return;
        }
        if matches!(event, NoteEvent::NoteOn {velocity, ..} if velocity>0.0) {
            let hold = self.params.lab.hold.value()
                || (self.params.lab.legato.value() && previous.is_some());
            if hold {
                if let Some(index) = self.held_syllable {
                    self.current_syllable = index;
                    if !self.live_on {
                        self.schedule_live(true);
                    }
                    return;
                }
            }
            let index = self.next_syllable % self.bank.len;
            self.next_syllable = (index + 1) % self.bank.len;
            if self.bank.held[index] {
                if let Some(last) = self.held_syllable {
                    self.current_syllable = last;
                    if !self.live_on {
                        self.schedule_live(true);
                    }
                    return;
                }
            }
            self.current_syllable = index;
            if self.bank.rests[index] {
                self.live_on = false;
                self.tail_end = 0.;
                if let Some(e) = &mut self.engine {
                    e.stop_speech();
                }
                if let Some(l) = &mut self.layers {
                    l.reset();
                }
                return;
            }
            self.held_syllable = Some(index);
            self.schedule_live(true);
        } else if previous != self.notes.active() {
            if self.notes.active().is_some() {
                self.schedule_live(true);
            } else if self.live_on {
                let e = self.engine.as_mut().unwrap();
                e.release(speech::NOTE_TAG);
                e.release(speech::NOTE_TAG + 1.0);
                self.scratch.clear();
                self.scratch
                    .extend_from_slice(&self.bank.off[self.current_syllable]);
                self.params.modulate(&mut self.scratch);
                e.schedule(&self.scratch, 0.0, false);
                let tail = self.scratch.iter().map(|g| g.end()).fold(0.15, f64::max);
                self.tail_end = e.clock() + tail + 0.15;
                if let Some(l) = &mut self.layers {
                    l.release(self.current_syllable, &self.params);
                }
                self.live_on = false;
            }
        }
    }
    fn read_local_keys(&mut self) {
        let keys = self.shared.local_keys.load(Ordering::Acquire);
        let base = self.shared.local_base.load(Ordering::Relaxed) as u8;
        if keys == self.key_mask && base == self.key_base {
            return;
        }
        for i in 0..17 {
            if self.key_mask & (1 << i) != 0 && (keys & (1 << i) == 0 || base != self.key_base) {
                self.handle_note(NoteEvent::NoteOff {
                    timing: 0,
                    voice_id: None,
                    channel: 15,
                    note: self.key_base + i,
                    velocity: 0.0,
                });
            }
        }
        for i in 0..17 {
            if keys & (1 << i) != 0 && (self.key_mask & (1 << i) == 0 || base != self.key_base) {
                self.handle_note(NoteEvent::NoteOn {
                    timing: 0,
                    voice_id: None,
                    channel: 15,
                    note: base + i,
                    velocity: 0.85,
                });
            }
        }
        self.key_mask = keys;
        self.key_base = base;
    }
    fn loop_tick(&mut self, beat: f64, bpm: f64, step: f64) {
        let length = self.bank.beats * step;
        if length <= 0.0 || !beat.is_finite() {
            return;
        }
        let cycle = (beat / length).floor() as i64;
        let phase = beat.rem_euclid(length);
        if self.loop_cycle != Some(cycle) {
            speech::scale_phrase(&self.bank, 60.0 / bpm * step, &mut self.scratch);
            self.params.modulate(&mut self.scratch);
            if let Some(e) = &mut self.engine {
                e.schedule(&self.scratch, -phase * 60.0 / bpm, true);
            }
            if let Some(l) = &mut self.layers {
                l.phrase(60. / bpm * step, phase * 60. / bpm, &self.params);
            }
            self.loop_cycle = Some(cycle);
        }
        let syl = self.bank.boundaries[..self.bank.len]
            .partition_point(|x| *x <= phase / step)
            .saturating_sub(1);
        self.shared
            .active_syllable
            .store(syl.min(self.bank.len - 1), Ordering::Relaxed);
        self.shared
            .phase
            .store((phase as f32 / length as f32).to_bits(), Ordering::Relaxed);
    }
    fn process_audio(
        &mut self,
        buffer: &mut Buffer,
        auxiliary: &mut AuxiliaryBuffers,
        transport: Playback,
        mut next_event: impl FnMut() -> Option<NoteEvent<()>>,
    ) -> ProcessStatus {
        let _float_mode = float_mode::FloatMode::enter();
        if self.panic.swap(false, Ordering::Relaxed) {
            self.reset_notes();
            self.panic_latched = true;
            self.shared.preview.store(false, Ordering::Relaxed);
        }
        if self.shared.rewind.swap(false, Ordering::Relaxed) {
            if let Some(poly) = &mut self.poly {
                poly.rewind();
            }
            self.next_syllable = 0;
            self.loop_cycle = None;
            self.preview_beat = 0.0;
            self.panic_latched = false;
        }
        let revision = self.shared.session_revision.load(Ordering::Acquire);
        if let Some(performance) = &mut self.performance {
            if performance.revision != revision {
                if let Ok(session) = self.params.lab.session.try_lock() {
                    performance.sync(&session, revision);
                }
            }
        }
        let hit = self.shared.hit.swap(0, Ordering::AcqRel);
        if hit > 0 {
            if let Some(performance) = &mut self.performance {
                performance.trigger((hit - 1) as usize, 1., &self.params);
            }
        }
        let mix = [self.params.lab.voice.value(), self.params.lab.choir.value()];
        if mix != self.previous_mix {
            self.loop_cycle = None;
            if self.live_on {
                let elapsed = self
                    .engine
                    .as_ref()
                    .map_or(0., |e| e.clock() - self.live_start);
                if let Some(l) = &mut self.layers {
                    l.live(self.current_syllable, elapsed, &self.params);
                }
            }
            self.previous_mix = mix;
        }
        let changed = self.sync_bank();
        let mode = self.params.mode.value();
        if mode != self.previous_mode
            || changed
            || self.previous_poly != self.params.lab.poly.value()
        {
            self.previous_poly = self.params.lab.poly.value();
            self.reset_notes();
            self.previous_mode = mode;
            self.panic_latched = false;
        }
        let mut offsets = [0.0; 11];
        for (i, (_, value)) in self.params.offsets().into_iter().enumerate() {
            offsets[i] = value;
        }
        offsets[10] = self.params.nasal.value() as f64;
        let mouth_changed = offsets != self.previous_offsets;
        self.previous_offsets = offsets;
        if mouth_changed && self.params.lab.poly.value() {
            if let (Some(poly), Some(layers)) = (&mut self.poly, &self.layers) {
                poly.remodulate(&self.params, &self.bank, layers);
            }
        }
        if mouth_changed && self.live_on {
            self.schedule_live(false);
        }
        self.read_local_keys();
        self.update_voice();
        let bpm = transport.bpm;
        let signature = [
            (bpm as f32).to_bits(),
            self.params.step_beats.value().to_bits(),
            self.params.lab.swing.value().to_bits(),
            self.params.lab.consonants.value().to_bits(),
            self.params.lab.natural.value() as u32,
        ];
        if signature != self.planning_signature {
            if self.params.lab.natural.value() || self.planning_signature[4] != 0 {
                self.pending_tasks |= 1;
            }
            self.planning_signature = signature;
        }
        let preview = self.shared.preview.load(Ordering::Relaxed);
        let playing = transport.playing;
        self.shared
            .playing
            .store(playing || preview, Ordering::Relaxed);
        let beat = if playing {
            transport.beat.unwrap_or(self.preview_beat)
        } else {
            self.preview_beat
        };
        let increment = bpm / (60.0 * self.sample_rate as f64);
        let step = self.params.step_beats.value() as f64;
        let jumped = playing
            && self
                .expected_beat
                .is_some_and(|expected| (expected - beat).abs() > increment * 2.0);
        if jumped || bpm != self.previous_tempo || step != self.previous_step || mouth_changed {
            if let Some(poly) = &mut self.poly {
                poly.invalidate_phrase();
            }
            self.loop_cycle = None;
            if jumped {
                if let Some(p) = &mut self.performance {
                    p.last_step = i64::MIN;
                    p.noise.stop_speech();
                    p.noise_tail = 0.;
                }
            }
        }
        self.previous_tempo = bpm;
        self.previous_step = step;
        self.shared
            .bpm
            .store((bpm as f32).to_bits(), Ordering::Relaxed);
        if !playing && !preview {
            if let Some(p) = &mut self.performance {
                p.last_step = i64::MIN;
            }
            self.panic_latched = false;
        }
        let running =
            mode == Mode::Loop && (playing || preview) && !self.panic_latched && self.bank.len > 0;
        if self.loop_running && !running {
            if let Some(e) = &mut self.engine {
                e.stop_speech();
                e.set_gate(0.0);
            }
            self.loop_cycle = None;
        }
        self.loop_running = running;
        self.expected_beat = if playing {
            Some(beat + buffer.samples() as f64 * increment)
        } else {
            None
        };
        let attack = (-1.0 / (self.params.attack.value() * 0.001 * self.sample_rate)).exp();
        let release = (-1.0 / (self.params.release.value() * 0.001 * self.sample_rate)).exp();
        let mut event = next_event();
        let input = auxiliary.inputs.first().map(|b| b.as_slice_immutable());
        for (index, channels) in buffer.iter_samples().enumerate() {
            let live_beat = beat + index as f64 * increment;
            if let Some(performance) = &mut self.performance {
                performance.age += 1. / self.sample_rate as f64;
                if self.params.lab.mic.value() && !self.params.lab.poly.value() {
                    let x = input
                        .and_then(|b| b.first())
                        .and_then(|b| b.get(index))
                        .copied()
                        .unwrap_or(0.);
                    performance.mic.push(x);
                }
            }
            if self.control_counter % 128 == 0 {
                self.shared
                    .beat
                    .store((live_beat as f32).to_bits(), Ordering::Relaxed);
                if let Some(performance) = &mut self.performance {
                    performance.update(&self.params, live_beat);
                    self.shared
                        .mic_level
                        .store(performance.mic.level.to_bits(), Ordering::Relaxed);
                    self.shared
                        .mic_hz
                        .store(performance.mic.hz.to_bits(), Ordering::Relaxed);
                    self.shared
                        .mic_confidence
                        .store(performance.mic.confidence.to_bits(), Ordering::Relaxed);
                    if self.params.lab.poly.value() {
                        if let Some(poly) = &mut self.poly {
                            poly.motion(&self.params, performance);
                        }
                    }
                    if !self.params.lab.poly.value() {
                        if let Some(e) = &mut self.engine {
                            if let Some(g) = performance
                                .gesture(&self.params, self.params.base_artic(&self.vowels))
                            {
                                e.schedule(&[g], 0., false);
                            }
                        }
                        if let Some(l) = &mut self.layers {
                            l.motion(&self.params, performance);
                        }
                    }
                }
                self.update_voice();
                self.record_motion(live_beat, playing || preview);
            }
            self.control_counter = self.control_counter.wrapping_add(1);
            if self.control_counter % 512 == 0 {
                if self.params.lab.poly.value() {
                    if let Some(poly) = &self.poly {
                        poly.telemetry(&self.shared);
                    }
                }
                if let Some(l) = &self.layers {
                    for i in 0..2 {
                        for (dest, v) in self.shared.layer_articulation[i]
                            .iter()
                            .zip(l.engines[i].voice().artic.x.to_array())
                        {
                            dest.store((v as f32).to_bits(), Ordering::Relaxed);
                        }
                    }
                }
                if self.envelope == 0. && !self.params.lab.poly.value() {
                    if let Some(p) = &self.performance {
                        if p.noise.clock() < p.noise_tail {
                            for (dest, v) in self
                                .shared
                                .articulation
                                .iter()
                                .zip(p.noise.voice().artic.x.to_array())
                            {
                                dest.store((v as f32).to_bits(), Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
            if self.params.lab.sequencer.value() && (playing || preview) && !self.panic_latched {
                if let Some(p) = &mut self.performance {
                    p.sequence(live_beat, &self.params);
                }
            }
            while let Some(e) = event {
                if e.timing() > index as u32 {
                    break;
                }
                self.handle_note(e);
                self.update_voice();
                event = next_event();
            }
            if running && !self.params.lab.poly.value() {
                self.loop_tick(beat + index as f64 * increment, bpm, step);
            }
            let gain = util::db_to_gain(self.params.gain.smoothed.next());
            let stereo = if self.params.lab.poly.value() {
                self.poly.as_mut().unwrap().render(
                    &self.params,
                    &self.bank,
                    self.layers.as_ref().unwrap(),
                    live_beat,
                    bpm,
                    running,
                    attack,
                    release,
                    gain,
                )
            } else {
                let value = self.render_sample(gain, attack, release);
                if let Some(l) = &mut self.layers {
                    l.mix(value, &self.params, self.envelope, gain)
                } else {
                    [value; 2]
                }
            };
            let noise = self.performance.as_mut().map_or(0., |p| p.render_noise())
                * self.params.lab.noise_level.value()
                * gain;
            let wet = self.params.lab.wet.value();
            let common = (stereo[0] + stereo[1]) * 0.5 + noise;
            let processed = self.performance.as_mut().map_or(common, |p| {
                p.wet(common, wet, self.params.lab.noise_tone.value())
            });
            let added = processed - (stereo[0] + stereo[1]) * 0.5;
            self.stereo = [stereo[0] + added, stereo[1] + added];
            let peak = self.stereo[0].abs().max(self.stereo[1].abs()).min(1.);
            let old = f32::from_bits(self.shared.level.load(Ordering::Relaxed));
            self.shared
                .level
                .store((old * 0.9995).max(peak).to_bits(), Ordering::Relaxed);
            for (i, sample) in channels.into_iter().enumerate() {
                let y = self.stereo[i.min(1)];
                *sample = if y.is_finite() { y.clamp(-1., 1.) } else { 0. };
            }
        }
        if playing
            || preview
            || self.notes.active().is_some()
            || self.envelope > 0.
            || self.params.lab.mic.value()
        {
            self.preview_beat = beat + buffer.samples() as f64 * increment;
        } else {
            self.preview_beat = 0.0;
        }
        let poly_active =
            self.params.lab.poly.value() && self.poly.as_ref().is_some_and(|v| v.active() > 0);
        self.shared.voices.store(
            if self.params.lab.poly.value() {
                self.poly.as_ref().map_or(0, |p| p.active())
            } else if self.envelope > 0. {
                if self.params.lab.choir.value() > 0. {
                    3
                } else {
                    1
                }
            } else {
                0
            },
            Ordering::Relaxed,
        );
        self.shared.sounding.store(
            poly_active
                || self.envelope > 0.00001
                || self
                    .performance
                    .as_ref()
                    .is_some_and(|p| p.noise.clock() < p.noise_tail),
            Ordering::Relaxed,
        );
        if self.shared.motion_state.load(Ordering::Acquire) == 3 {
            self.shared.motion_state.store(4, Ordering::Release);
            self.pending_tasks |= 2;
        }
        ProcessStatus::KeepAlive
    }
    fn record_motion(&mut self, beat: f64, running: bool) {
        let state = self.shared.motion_state.load(Ordering::Acquire);
        if state == 0 {
            self.record_start = None;
            self.record_index = 0;
            return;
        }
        if !running || state >= 3 {
            return;
        }
        let start = *self.record_start.get_or_insert_with(|| beat.ceil());
        if beat < start {
            return;
        }
        self.shared.motion_state.store(2, Ordering::Release);
        let phase = (beat - start) / self.params.lab.motion_length.value() as f64;
        let end = ((phase * 256.).floor() as usize).min(255);
        let pose = lab::pose(&self.params);
        while self.record_index <= end {
            for (i, v) in pose.iter().enumerate() {
                self.shared.recorded[self.record_index][i].store(v.to_bits(), Ordering::Relaxed);
            }
            self.record_index += 1;
        }
        if phase >= 1. {
            self.shared.motion_state.store(3, Ordering::Release);
            self.record_start = None;
            self.record_index = 0;
        }
    }
    fn render_sample(&mut self, gain: f32, attack: f32, release: f32) -> f32 {
        let speech_active = self.loop_running
            || self.live_on
            || self
                .engine
                .as_ref()
                .is_some_and(|e| e.clock() < self.tail_end);
        let target = if self.params.mode.value() == Mode::Vowels {
            self.notes
                .active()
                .map(|(_, _, v)| v.sqrt())
                .unwrap_or_else(|| {
                    if self.params.lab.mic.value() && !self.params.lab.poly.value() {
                        self.performance.as_ref().map_or(0., |p| {
                            if p.mic.confidence > 0.78
                                && p.mic.level * self.params.lab.mic_gain.value()
                                    > self.params.lab.mic_gate.value()
                            {
                                (p.mic.level * self.params.lab.mic_gain.value() * 5.).min(1.)
                            } else {
                                0.
                            }
                        })
                    } else {
                        0.
                    }
                })
        } else if speech_active {
            0.92
        } else {
            0.0
        };
        let coefficient = if target > self.envelope {
            attack
        } else {
            release
        };
        self.envelope = target + coefficient * (self.envelope - target);
        if target == 0.0 && self.envelope < 0.00001 {
            self.envelope = 0.0;
            if let Some(engine) = &mut self.engine {
                engine.set_gate(0.0);
            }
            return 0.0;
        }
        let mut sample = [0.0];
        if let Some(engine) = &mut self.engine {
            engine.render(&mut sample);
        }
        self.telemetry_count += 1;
        if self.telemetry_count >= 512 {
            self.telemetry_count = 0;
            if let Some(e) = &self.engine {
                for (dest, value) in self
                    .shared
                    .articulation
                    .iter()
                    .zip(e.voice().artic.x.to_array())
                {
                    dest.store((value as f32).to_bits(), Ordering::Relaxed);
                }
                self.shared
                    .pitch
                    .store((e.voice().source.f0 as f32).to_bits(), Ordering::Relaxed);
            }
        }
        let value = sample[0] * self.envelope * gain;
        let old_level = f32::from_bits(self.shared.level.load(Ordering::Relaxed));
        self.shared.level.store(
            (old_level * 0.9995).max(value.abs()).to_bits(),
            Ordering::Relaxed,
        );
        if value.is_finite() {
            value.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }
}
impl Plugin for ChimeraPlugin {
    const NAME: &'static str = "Chimera Lab";
    const VENDOR: &'static str = "Chimera Community";
    const URL: &'static str = "https://github.com/jamesondh/chimera-vocal-synth";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[new_nonzero_u32(1)],
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = Task;
    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        config: &BufferConfig,
        _: &mut impl InitContext<Self>,
    ) -> bool {
        if !(22050.0..=192000.0).contains(&config.sample_rate) {
            return false;
        }
        self.prepare(config.sample_rate);
        true
    }
    fn reset(&mut self) {
        self.reset_notes();
        if let (Some(e), Some(w)) = (&mut self.engine, &self.warm_voice) {
            e.reset_human_from(w);
        }
        if let (Some(l), Some(w)) = (&mut self.layers, &self.warm_layers) {
            for i in 0..2 {
                l.engines[i].reset_human_from(&w[i]);
            }
        }
        self.control_counter = 0;
    }
    fn editor(&mut self, executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::editor(
            self.params.clone(),
            self.shared.clone(),
            self.panic.clone(),
            executor,
        )
    }
    fn task_executor(&mut self) -> TaskExecutor<Self> {
        let params = self.params.clone();
        let shared = self.shared.clone();
        Box::new(move |task| match task {
            Task::Compile => {
                shared.compile_requested.store(false, Ordering::Release);
                publish_text(&params, &shared);
                shared.compile_busy.store(false, Ordering::Release);
            }
            Task::SaveMotion => {
                let frames = shared
                    .recorded
                    .iter()
                    .map(|pose| {
                        pose.each_ref()
                            .map(|x| f32::from_bits(x.load(Ordering::Relaxed)))
                    })
                    .collect();
                params.lab.session.lock().unwrap().motion = frames;
                shared.session_revision.fetch_add(1, Ordering::Release);
                shared.motion_state.store(0, Ordering::Release);
            }
        })
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        auxiliary: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let t = context.transport();
        let transport = Playback {
            playing: t.playing,
            bpm: t
                .tempo
                .filter(|v| v.is_finite() && *v > 0.)
                .unwrap_or(120.)
                .clamp(20., 400.),
            beat: t.pos_beats(),
        };
        let started = std::time::Instant::now();
        let result = self.process_audio(buffer, auxiliary, transport, || context.next_event());
        if buffer.samples() > 0 {
            let load = (started.elapsed().as_secs_f32() * self.sample_rate
                / buffer.samples() as f32
                * 100.)
                .min(999.);
            let old = f32::from_bits(self.shared.cpu_load.load(Ordering::Relaxed));
            self.shared
                .cpu_load
                .store((old * 0.9 + load * 0.1).to_bits(), Ordering::Relaxed);
        }
        let tasks = std::mem::take(&mut self.pending_tasks);
        if tasks & 1 != 0 {
            self.shared.compile_requested.store(true, Ordering::Release);
        }
        if self.shared.compile_requested.load(Ordering::Acquire)
            && !self.shared.compile_busy.swap(true, Ordering::AcqRel)
        {
            context.execute_background(Task::Compile);
        }
        if tasks & 2 != 0 {
            context.execute_background(Task::SaveMotion);
        }
        result
    }
}
impl Vst3Plugin for ChimeraPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"ChimeraLab000003";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}
nih_export_vst3!(ChimeraPlugin);
