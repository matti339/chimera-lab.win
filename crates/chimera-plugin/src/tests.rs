use super::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    io::Write,
};
thread_local! { static COUNT: Cell<Option<usize>> = const { Cell::new(None) }; }
struct Counter;
#[global_allocator]
static ALLOC: Counter = Counter;
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, size) }
    }
}
pub(super) fn note(on: bool, note: u8) -> NoteEvent<()> {
    if on {
        NoteEvent::NoteOn {
            timing: 0,
            voice_id: None,
            channel: 0,
            note,
            velocity: 0.85,
        }
    } else {
        NoteEvent::NoteOff {
            timing: 0,
            voice_id: None,
            channel: 0,
            note,
            velocity: 0.0,
        }
    }
}
#[test]
fn native_audio_is_finite_non_silent_and_releases_at_common_sample_rates() {
    for rate in [44100.0, 48000.0, 96000.0] {
        let mut plugin = ChimeraPlugin::default();
        plugin.prepare(rate);
        assert_eq!(plugin.render_sample(0.5, 0.99, 0.99), 0.0);
        plugin.notes.handle(note(true, 57));
        plugin.update_voice();
        let mut energy = 0.0_f64;
        for _ in 0..rate as usize {
            let sample = plugin.render_sample(0.5, 0.99, 0.99);
            assert!(sample.is_finite() && sample.abs() <= 1.0);
            energy += (sample as f64).powi(2);
        }
        assert!(energy / rate as f64 > 1e-8, "silent at {rate}: {energy}");
        plugin.notes.handle(note(false, 57));
        plugin.update_voice();
        for _ in 0..rate as usize {
            plugin.render_sample(0.5, 0.99, 0.99);
        }
        assert_eq!(plugin.render_sample(0.5, 0.99, 0.99), 0.0);
    }
}
#[test]
fn midi_controls_and_audio_allocate_nothing_after_initialization() {
    let mut plugin = ChimeraPlugin::default();
    plugin.prepare(48000.0);
    COUNT.with(|c| c.set(Some(0)));
    plugin.notes.handle(note(true, 57));
    plugin.update_voice();
    for i in 0..48000 {
        if i == 16000 {
            plugin.notes.handle(note(true, 60));
            plugin.update_voice();
        }
        if i == 32000 {
            plugin.notes.handle(note(false, 60));
            plugin.update_voice();
        }
        plugin.render_sample(0.5, 0.99, 0.99);
    }
    plugin.reset_notes();
    let count = COUNT.with(|c| c.replace(None).unwrap());
    assert_eq!(count, 0, "allocation in realtime path");
}
#[test]
fn render_demo_if_requested() {
    let Ok(path) = std::env::var("CHIMERA_PLUGIN_DEMO") else {
        return;
    };
    let mut plugin = ChimeraPlugin::default();
    plugin.prepare(48000.0);
    let mut samples = Vec::new();
    for (i, key) in [48, 52, 55, 60, 57].into_iter().enumerate() {
        plugin.notes.handle(note(true, key));
        plugin.update_voice();
        plugin
            .engine
            .as_mut()
            .unwrap()
            .set_artic(&plugin.vowels[i], None);
        for _ in 0..36000 {
            samples.push(plugin.render_sample(0.5, 0.998, 0.999));
        }
        plugin.notes.handle(note(false, key));
        plugin.update_voice();
        for _ in 0..12000 {
            samples.push(plugin.render_sample(0.5, 0.998, 0.999));
        }
    }
    let mut f = std::fs::File::create(path).unwrap();
    let bytes = samples.len() as u32 * 2;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + bytes).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16_u32.to_le_bytes()).unwrap();
    f.write_all(&1_u16.to_le_bytes()).unwrap();
    f.write_all(&1_u16.to_le_bytes()).unwrap();
    f.write_all(&48000_u32.to_le_bytes()).unwrap();
    f.write_all(&96000_u32.to_le_bytes()).unwrap();
    f.write_all(&2_u16.to_le_bytes()).unwrap();
    f.write_all(&16_u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&bytes.to_le_bytes()).unwrap();
    for sample in samples {
        f.write_all(&((sample * 32767.0) as i16).to_le_bytes())
            .unwrap();
    }
}

fn with_mode(mode: Mode, words: &str) -> ChimeraPlugin {
    let mut plugin = ChimeraPlugin::default();
    Arc::get_mut(&mut plugin.params).unwrap().mode = EnumParam::new("Mode", mode);
    *plugin.params.lyrics.lock().unwrap() = words.into();
    plugin.prepare(48000.0);
    plugin.update_voice();
    plugin
}
#[test]
fn note_off_retains_pitch_through_the_release_and_final_consonants() {
    for mode in [Mode::Vowels, Mode::Syllables] {
        let mut plugin = with_mode(mode, "world");
        plugin.handle_note(note(true, 69));
        plugin.update_voice();
        assert_eq!(plugin.engine.as_ref().unwrap().voice().source.f0, 440.0);
        plugin.handle_note(note(false, 69));
        plugin.update_voice();
        assert_eq!(plugin.engine.as_ref().unwrap().voice().source.f0, 440.0);
    }
}
#[test]
fn syllables_advance_only_on_notes_and_release_with_no_allocations() {
    let mut plugin = with_mode(Mode::Syllables, "hello world");
    COUNT.with(|c| c.set(Some(0)));
    plugin.handle_note(note(true, 48));
    plugin.update_voice();
    let first = plugin.current_syllable;
    let mut energy = 0.0;
    for _ in 0..24000 {
        let y = plugin.render_sample(0.5, 0.99, 0.99);
        energy += y * y;
    }
    plugin.handle_note(note(false, 48));
    let next_after_off = plugin.next_syllable;
    for _ in 0..48000 {
        plugin.render_sample(0.5, 0.99, 0.99);
    }
    let released = plugin.envelope;
    plugin.handle_note(note(true, 52));
    plugin.update_voice();
    let second = plugin.current_syllable;
    plugin.handle_note(NoteEvent::MidiPitchBend {
        timing: 0,
        channel: 0,
        value: 1.0,
    });
    plugin.update_voice();
    let next_after_bend = plugin.next_syllable;
    for _ in 0..24000 {
        plugin.render_sample(0.5, 0.99, 0.99);
    }
    let count = COUNT.with(|c| c.replace(None).unwrap());
    assert_eq!(count, 0);
    assert_eq!(first, 0);
    assert_eq!(second, 1);
    assert_eq!(next_after_off, 1);
    assert_eq!(next_after_bend, 2);
    assert_eq!(released, 0.0);
    assert!(energy > 0.01);
}
#[test]
fn loop_scheduling_and_plan_handoff_allocate_nothing() {
    let mut plugin = with_mode(Mode::Loop, "hello world");
    *plugin.params.lyrics.lock().unwrap() = "take me home".into();
    publish_text(&plugin.params, &plugin.shared);
    COUNT.with(|c| c.set(Some(0)));
    let updated = plugin.sync_bank();
    plugin.loop_running = true;
    let mut energy = 0.0;
    for i in 0..144000 {
        plugin.loop_tick(i as f64 / 24000.0, 120.0, 1.0);
        let y = plugin.render_sample(0.5, 0.99, 0.99);
        energy += y * y;
    }
    let cycle = plugin.loop_cycle;
    let count = COUNT.with(|c| c.replace(None).unwrap());
    assert_eq!(count, 0);
    assert!(updated);
    assert!(energy > 0.1);
    assert_eq!(cycle, Some(1));
}
#[test]
fn local_key_release_on_focus_loss_cannot_leave_a_note_held() {
    let mut plugin = with_mode(Mode::Syllables, "one two");
    plugin.shared.local_keys.store(1, Ordering::Release);
    plugin.read_local_keys();
    assert!(plugin.notes.active().is_some());
    assert_eq!(plugin.next_syllable, 1);
    plugin.read_local_keys();
    assert_eq!(plugin.next_syllable, 1); // no repeated advancement per GUI frame
    plugin.shared.local_keys.store(0, Ordering::Release);
    plugin.read_local_keys();
    assert!(plugin.notes.active().is_none());
    assert!(!plugin.live_on);
}
#[test]
fn lyrics_survive_state_restore_and_rebuild_without_the_editor() {
    let source = with_mode(Mode::Syllables, "never gonna give you up");
    let fields = source.params.serialize_fields();
    let mut restored = ChimeraPlugin::default();
    restored.params.deserialize_fields(&fields);
    restored.prepare(48000.0);
    assert_eq!(
        *restored.params.lyrics.lock().unwrap(),
        "never gonna give you up"
    );
    assert_eq!(source.bank.len, restored.bank.len);
    assert_eq!(source.bank.labels, restored.bank.labels);
}
#[test]
fn mouth_modulation_changes_the_planned_articulators() {
    let mut plugin = with_mode(Mode::Loop, "hello");
    let p = Arc::get_mut(&mut plugin.params).unwrap();
    p.tongue_x = control("Tongue front", 0.75, -2.0, 2.0);
    let mut modified = plugin.bank.phrase.clone();
    plugin.params.modulate(&mut modified);
    let mut changed = 0;
    for (a, b) in plugin.bank.phrase.iter().zip(&modified) {
        if let Some(value) = a.targets.get(ArticKey::TbX) {
            assert!((b.targets.get(ArticKey::TbX).unwrap() - value - 0.75).abs() < 1e-12);
            changed += 1;
        }
    }
    assert!(changed > 0);
}
#[test]
fn render_words_demo_if_requested() {
    let Ok(path) = std::env::var("CHIMERA_WORDS_DEMO") else {
        return;
    };
    let mut plugin = with_mode(Mode::Loop, "hello from the other side");
    plugin.loop_running = true;
    let mut audio = Vec::new();
    for i in 0..192000 {
        // two beats per second, four seconds
        plugin.loop_tick(i as f64 / 24000.0, 120.0, 1.0);
        audio.push(plugin.render_sample(0.5, 0.99, 0.999));
    }
    let mut f = std::fs::File::create(path).unwrap();
    let bytes = audio.len() as u32 * 2;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + bytes).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16_u32.to_le_bytes()).unwrap();
    f.write_all(&1_u16.to_le_bytes()).unwrap();
    f.write_all(&1_u16.to_le_bytes()).unwrap();
    f.write_all(&48000_u32.to_le_bytes()).unwrap();
    f.write_all(&96000_u32.to_le_bytes()).unwrap();
    f.write_all(&2_u16.to_le_bytes()).unwrap();
    f.write_all(&16_u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&bytes.to_le_bytes()).unwrap();
    for y in audio {
        f.write_all(&((y * 32767.0) as i16).to_le_bytes()).unwrap();
    }
}

#[test]
fn classic_matches_the_saved_v02_audio_sample_for_sample() {
    let reference = include_bytes!("../fixtures/classic-0.2-48k.f32");
    let mut p = ChimeraPlugin::default();
    p.prepare(48000.);
    p.notes.handle(note(true, 57));
    p.update_voice();
    for (i, bytes) in reference.chunks_exact(4).enumerate() {
        if i == 32000 {
            p.notes.handle(note(false, 57));
            p.update_voice();
        }
        let y = p.render_sample(0.5, 0.99, 0.99);
        let expected = f32::from_le_bytes(bytes.try_into().unwrap());
        assert_eq!(y, expected, "Classic changed at sample {i}");
    }
}
#[test]
fn hold_legato_and_melisma_preserve_syllable_and_release_coda() {
    let mut p = with_mode(Mode::Syllables, "love ~ you");
    p.handle_note(note(true, 60));
    let first = p.current_syllable;
    p.handle_note(note(true, 64));
    assert_eq!(p.current_syllable, first);
    assert_eq!(p.next_syllable, 2);
    p.handle_note(note(false, 60));
    p.handle_note(note(false, 64));
    assert!(!p.live_on);
    Arc::get_mut(&mut p.params).unwrap().lab.hold = BoolParam::new("Hold", true);
    p.handle_note(note(true, 67));
    assert_eq!(p.current_syllable, first);
    assert_eq!(p.next_syllable, 2);
    p.handle_note(note(false, 67));
    Arc::get_mut(&mut p.params).unwrap().lab.hold = BoolParam::new("Hold", false);
    p.handle_note(note(true, 69));
    assert_eq!(p.current_syllable, 2);
}
#[test]
fn natural_timing_keeps_lengths_rests_and_body_specific_plans() {
    let mut session = lab::Session::default();
    session.durations[0] = 2.;
    session.rests[1] = true;
    let a = speech::compile_for(
        "one two three",
        BodyMix::MALE,
        &session,
        true,
        120.,
        1.,
        0.,
        1.,
    )
    .unwrap();
    let b = speech::compile_for(
        "one two three",
        BodyMix::FEMALE,
        &session,
        true,
        120.,
        1.,
        0.,
        1.,
    )
    .unwrap();
    assert_eq!(a.beats, 4.);
    assert_eq!(&a.boundaries[..4], &[0., 2., 3., 4.]);
    assert!(a.rests[1]);
    assert_ne!(a.phrase, b.phrase);
    assert!(
        a.phrase
            .iter()
            .all(|g| g.t0 >= 0. && g.t1 <= 4. && g.t1 > g.t0)
    );
    let slow = speech::compile_for(
        "shoe",
        BodyMix::MALE,
        &lab::Session::default(),
        true,
        60.,
        1.,
        0.,
        1.,
    )
    .unwrap();
    let fast = speech::compile_for(
        "shoe",
        BodyMix::MALE,
        &lab::Session::default(),
        true,
        120.,
        1.,
        0.,
        1.,
    )
    .unwrap();
    let consonant = |b: &speech::Bank, seconds: f64| {
        b.phrase
            .iter()
            .find(|g| g.strength >= 20.)
            .map(|g| (g.t1 - g.t0) * seconds)
            .unwrap()
    };
    assert!((consonant(&slow, 1.) - consonant(&fast, 0.5)).abs() < 0.025);
}
#[test]
fn microphone_tracks_pitch_and_rejects_silence_without_allocations() {
    for rate in [44100., 48000., 96000.] {
        let mut p = lab::PitchFollower::new(rate);
        COUNT.with(|c| c.set(Some(0)));
        for i in 0..rate as usize {
            p.push((std::f32::consts::TAU * 220. * i as f32 / rate).sin() * 0.2);
        }
        let allocations = COUNT.with(|c| c.replace(None).unwrap());
        assert_eq!(allocations, 0);
        assert!(p.confidence > 0.9);
        assert!((p.hz - 220.).abs() < 8., "{} Hz at {rate}", p.hz);
        for _ in 0..rate as usize {
            p.push(0.);
        }
        assert!(p.level < 0.001 && p.confidence < 0.1);
    }
}
#[test]
fn motion_capture_playback_and_shapes_survive_state_recall() {
    let mut p = ChimeraPlugin::default();
    p.prepare(48000.);
    p.shared.motion_state.store(1, Ordering::Release);
    for i in 0..257 {
        Arc::get_mut(&mut p.params).unwrap().tongue_x = control("Tongue", i as f32 / 256., -2., 2.);
        p.record_motion(i as f64 * 4. / 256., true);
    }
    assert_eq!(p.shared.motion_state.load(Ordering::Acquire), 3);
    p.task_executor()(Task::SaveMotion);
    let restored = ChimeraPlugin::default();
    restored
        .params
        .deserialize_fields(&p.params.serialize_fields());
    let session = restored.params.lab.session.lock().unwrap();
    assert_eq!(session.motion.len(), 256);
    assert!(session.motion[0][0] < 0.01);
    assert!(session.motion[255][0] > 0.99);
    let mut shapes = [[0.; 11]; 4];
    shapes[1][0] = 1.;
    shapes[2][0] = 2.;
    shapes[3][0] = 3.;
    assert_eq!(lab::morph(&shapes, 0.5, 0.5)[0], 1.5);
}
#[test]
fn reset_replays_the_same_classic_audio_without_allocating() {
    let mut p = ChimeraPlugin::default();
    p.prepare(48000.);
    let mut renders = Vec::new();
    for _ in 0..2 {
        p.reset();
        p.handle_note(note(true, 60));
        p.update_voice();
        renders.push(
            (0..10000)
                .map(|_| p.render_sample(0.5, 0.99, 0.99))
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(renders[0], renders[1]);
    COUNT.with(|c| c.set(Some(0)));
    p.reset();
    let count = COUNT.with(|c| c.replace(None).unwrap());
    assert_eq!(count, 0);
}
struct HostBuffers {
    left: Vec<f32>,
    right: Vec<f32>,
    input: Vec<f32>,
}
impl HostBuffers {
    fn new(size: usize) -> Self {
        Self {
            left: vec![0.; size],
            right: vec![0.; size],
            input: vec![0.; size],
        }
    }
    fn run(
        &mut self,
        p: &mut ChimeraPlugin,
        play: Playback,
        events: &[NoteEvent<()>],
        count_alloc: bool,
    ) -> usize {
        let mut output = Buffer::default();
        let mut side = Buffer::default();
        let size = self.left.len();
        unsafe {
            output.set_slices(size, |v| {
                v.push(&mut self.left);
                v.push(&mut self.right);
            });
            side.set_slices(size, |v| v.push(&mut self.input));
        }
        let mut ins = [side];
        let mut auxiliary = AuxiliaryBuffers {
            inputs: &mut ins,
            outputs: &mut [],
        };
        let mut events = events.iter().copied();
        if count_alloc {
            COUNT.with(|c| c.set(Some(0)));
        }
        p.process_audio(&mut output, &mut auxiliary, play, || events.next());
        if count_alloc {
            COUNT.with(|c| c.replace(None).unwrap())
        } else {
            0
        }
    }
}
#[test]
fn full_audio_callback_layers_percussion_motion_and_mic_are_realtime_safe() {
    let mut p = ChimeraPlugin::default();
    let c = Arc::get_mut(&mut p.params).unwrap();
    c.lab.choir = control("Choir", 0.6, 0., 1.);
    c.lab.wet = control("Wet", 0.7, 0., 1.);
    c.lab.noise_level = control("Clicks", 0.7, 0., 1.);
    c.lab.movement = control("Motion", 0.8, 0., 1.);
    c.lab.sequencer = BoolParam::new("Sequence", true);
    c.lab.mic = BoolParam::new("Mic", true);
    c.lab.session.lock().unwrap().steps = [7, 0, 8, 0, 6, 0, 7, 0, 0, 8, 0, 6, 7, 0, 8, 0];
    p.prepare(48000.);
    let mut b = HostBuffers::new(256);
    let mut energy = 0.;
    let chord = [note(true, 48), note(true, 55), note(true, 60)];
    for block in 0..240 {
        for (i, x) in b.input.iter_mut().enumerate() {
            *x = (std::f32::consts::TAU * 180. * (block * 256 + i) as f32 / 48000.).sin() * 0.1;
        }
        let n = b.run(
            &mut p,
            Playback {
                playing: true,
                bpm: 120.,
                beat: Some(block as f64 * 256. / 24000.),
            },
            if block == 0 { &chord } else { &[] },
            true,
        );
        assert_eq!(n, 0, "allocation in full callback");
        for y in &b.left {
            assert!(y.is_finite() && y.abs() <= 1.);
            energy += y * y;
        }
    }
    assert!(energy > 0.01, "silent callback");
    p.panic.store(true, Ordering::Release);
    b.run(
        &mut p,
        Playback {
            playing: false,
            bpm: 120.,
            beat: Some(0.),
        },
        &[],
        true,
    );
    assert!(b.left.iter().all(|v| v.abs() < 1e-7));
}
#[test]
fn host_state_and_factory_presets_include_every_new_control() {
    let p = Controls::default();
    let map = p.param_map();
    let ids: std::collections::HashSet<_> = map.iter().map(|x| &x.0).collect();
    assert_eq!(ids.len(), map.len());
    let wet = presets::factory(1);
    assert!(wet.values["lab_wet"] > 0.);
    assert!(wet.session.steps.contains(&7));
    let json = serde_json::to_vec(&wet).unwrap();
    let recalled: presets::Snapshot = serde_json::from_slice(&json).unwrap();
    assert_eq!(wet.fingerprint(), recalled.fingerprint());
}

#[test]
fn modulation_and_click_patterns_are_independent_of_host_buffer_size() {
    fn render(size: usize) -> Vec<f32> {
        let mut p = ChimeraPlugin::default();
        let c = Arc::get_mut(&mut p.params).unwrap();
        c.lab.movement = control("Move", 0.7, 0., 1.);
        c.lab.wet = control("Wet", 0.5, 0., 1.);
        c.lab.noise_level = control("Clicks", 0.5, 0., 1.);
        c.lab.sequencer = BoolParam::new("Seq", true);
        c.lab.session.lock().unwrap().steps = [7; 16];
        p.prepare(48000.);
        let mut b = HostBuffers::new(size);
        let mut result = Vec::new();
        let onset = [note(true, 48)];
        for pos in (0..16384).step_by(size) {
            b.run(
                &mut p,
                Playback {
                    playing: true,
                    bpm: 120.,
                    beat: Some(pos as f64 / 24000.),
                },
                if pos == 0 { &onset } else { &[] },
                false,
            );
            result.extend_from_slice(&b.left);
        }
        result
    }
    let small = render(64);
    for size in [256, 1024] {
        let large = render(size);
        let difference = small
            .iter()
            .zip(large)
            .map(|(a, b)| (a - b).abs())
            .fold(0., f32::max);
        assert!(difference < 1e-6, "block {size}: {difference}");
    }
}
#[test]
fn render_lab_demos_and_measure_cpu_when_requested() {
    let Ok(folder) = std::env::var("CHIMERA_LAB_DEMOS") else {
        return;
    };
    for scene in ["Classic", "Slime-Chamber", "Throat-Creature", "Choir"] {
        let mut p = ChimeraPlugin::default();
        let c = Arc::get_mut(&mut p.params).unwrap();
        match scene {
            "Slime-Chamber" => {
                c.lab.wet = control("Wet", 0.85, 0., 1.);
                c.lab.noise_level = control("Clicks", 0.7, 0., 1.);
                c.lab.noise_tone = control("Tone", 0.16, 0., 1.);
                c.lab.noise_decay = control("Decay", 0.3, 0.04, 1.);
                c.lab.sequencer = BoolParam::new("Seq", true);
                c.lab.session.lock().unwrap().steps =
                    [7, 0, 8, 0, 0, 6, 0, 8, 7, 0, 0, 8, 0, 6, 8, 0];
            }
            "Throat-Creature" => {
                c.lab.fry = control("Fry", 0.65, 0., 1.);
                c.lab.rough = control("Rough", 0.6, 0., 1.);
                c.lab.ventricular = control("Narrow", 0.65, 0., 1.);
                c.lab.wet = control("Wet", 0.3, 0., 1.);
                c.lab.movement = control("Move", 0.35, 0., 1.);
                c.lab.size = control("Size", 0.6, -1., 1.);
                c.vowel = control("Vowel", 3.4, 0., 4.);
            }
            "Choir" => {
                c.lab.choir = control("Choir", 0.65, 0., 1.);
                c.lab.spread = control("Spread", 0.8, 0., 1.);
                c.lab.air = control("Air", 0.3, 0., 1.);
            }
            _ => {}
        }
        p.prepare(48000.);
        p.params.gain.smoothed.reset(p.params.gain.value());
        let mut b = HostBuffers::new(256);
        let mut samples = Vec::new();
        let start = std::time::Instant::now();
        for block in 0..1500 {
            let ev = if scene == "Slime-Chamber" {
                None
            } else if block % 375 == 0 {
                Some(note(true, [48, 55, 60, 52][block / 375]))
            } else if block % 375 == 330 {
                Some(note(false, [48, 55, 60, 52][block / 375]))
            } else {
                None
            };
            b.run(
                &mut p,
                Playback {
                    playing: true,
                    bpm: 120.,
                    beat: Some(block as f64 * 256. / 24000.),
                },
                ev.as_slice(),
                false,
            );
            for i in 0..256 {
                samples.push(b.left[i]);
                samples.push(b.right[i]);
            }
        }
        let elapsed = start.elapsed().as_secs_f64();
        eprintln!(
            "{scene}: 8 seconds stereo in {elapsed:.3}s ({:.1}% of one core realtime budget)",
            elapsed / 8. * 100.
        );
        let path = std::path::Path::new(&folder).join(format!("Chimera-Lab-{scene}.wav"));
        let mut f = std::fs::File::create(path).unwrap();
        let bytes = samples.len() as u32 * 2;
        f.write_all(b"RIFF").unwrap();
        f.write_all(&(36 + bytes).to_le_bytes()).unwrap();
        f.write_all(b"WAVEfmt ").unwrap();
        f.write_all(&16_u32.to_le_bytes()).unwrap();
        f.write_all(&1_u16.to_le_bytes()).unwrap();
        f.write_all(&2_u16.to_le_bytes()).unwrap();
        f.write_all(&48000_u32.to_le_bytes()).unwrap();
        f.write_all(&192000_u32.to_le_bytes()).unwrap();
        f.write_all(&4_u16.to_le_bytes()).unwrap();
        f.write_all(&16_u16.to_le_bytes()).unwrap();
        f.write_all(b"data").unwrap();
        f.write_all(&bytes.to_le_bytes()).unwrap();
        for x in samples {
            f.write_all(&((x.clamp(-1., 1.) * 32767.) as i16).to_le_bytes())
                .unwrap();
        }
    }
}

#[test]
fn cpu_profile_when_requested() {
    if std::env::var_os("CHIMERA_CPU_PROFILE").is_none() {
        return;
    }
    for scene in [
        "Idle",
        "Mono",
        "Choir",
        "Choir idle",
        "Poly 2",
        "Poly 4",
        "Poly 6",
        "Poly idle",
    ] {
        let mut runs = Vec::new();
        for _ in 0..3 {
            let mut p = ChimeraPlugin::default();
            if scene.starts_with("Choir") {
                Arc::get_mut(&mut p.params).unwrap().lab.choir = control("Choir", 0.65, 0., 1.);
            }
            if scene.starts_with("Poly") {
                let c = Arc::get_mut(&mut p.params).unwrap();
                c.lab.poly = BoolParam::new("Poly", true);
                c.lab.voice_limit = IntParam::new("Limit", 6, IntRange::Linear { min: 2, max: 6 });
            }
            p.prepare(48000.);
            p.params.gain.smoothed.reset(p.params.gain.value());
            let mut b = HostBuffers::new(256);
            let ev = if scene == "Mono" || scene == "Choir" {
                vec![note(true, 48)]
            } else if let Ok(count) = scene.trim_start_matches("Poly ").parse::<usize>() {
                [48, 52, 55, 60, 64, 67][..count]
                    .iter()
                    .map(|n| note(true, *n))
                    .collect()
            } else {
                vec![]
            };
            b.run(
                &mut p,
                Playback {
                    playing: false,
                    bpm: 120.,
                    beat: None,
                },
                &ev,
                false,
            );
            let start = std::time::Instant::now();
            for _ in 0..750 {
                b.run(
                    &mut p,
                    Playback {
                        playing: false,
                        bpm: 120.,
                        beat: None,
                    },
                    &[],
                    false,
                );
                std::hint::black_box(&b.left);
            }
            runs.push(start.elapsed().as_secs_f64() / 4. * 100.);
        }
        runs.sort_by(f64::total_cmp);
        eprintln!(
            "CPU {scene}: median {:.3}% of one core (48k / 256, 4 s, three runs); {:?}",
            runs[1], runs
        );
    }
}

fn poly_plugin(mode: Mode, limit: i32) -> ChimeraPlugin {
    let mut p = ChimeraPlugin::default();
    let c = Arc::get_mut(&mut p.params).unwrap();
    c.lab.poly = BoolParam::new("Polyphonic choir", true);
    c.lab.voice_limit = IntParam::new("Voice limit", limit, IntRange::Linear { min: 2, max: 6 });
    c.mode = EnumParam::new("Mode", mode);
    *c.lyrics.lock().unwrap() = "hello world".into();
    p.prepare(48000.);
    p.params.gain.smoothed.reset(p.params.gain.value());
    p
}
#[test]
fn polyphonic_notes_release_independently_and_steal_without_resurrection() {
    let mut p = poly_plugin(Mode::Vowels, 2);
    let mut b = HostBuffers::new(256);
    let play = Playback {
        playing: false,
        bpm: 120.,
        beat: None,
    };
    assert_eq!(
        b.run(&mut p, play, &[note(true, 48), note(true, 55)], true),
        0
    );
    assert_eq!(
        &p.poly.as_ref().unwrap().keys()[..2],
        &[Some((0, 48)), Some((0, 55))]
    );
    assert_eq!(b.run(&mut p, play, &[note(true, 60)], true), 0);
    assert_eq!(
        &p.poly.as_ref().unwrap().keys()[..2],
        &[Some((0, 60)), Some((0, 55))]
    );
    b.run(&mut p, play, &[note(false, 48), note(false, 60)], false);
    for _ in 0..320 {
        assert_eq!(b.run(&mut p, play, &[], true), 0);
    }
    assert_eq!(
        &p.poly.as_ref().unwrap().keys()[..2],
        &[None, Some((0, 55))]
    );
    assert!(b.left.iter().any(|x| x.abs() > 1e-5));
    b.run(&mut p, play, &[note(false, 55)], false);
    for _ in 0..320 {
        b.run(&mut p, play, &[], false);
    }
    assert_eq!(p.poly.as_ref().unwrap().active(), 0);
    assert!(b.left.iter().all(|x| *x == 0.));
}
#[test]
fn poly_sustain_chords_syllables_and_reset_are_realtime_safe() {
    let mut p = poly_plugin(Mode::Syllables, 4);
    let mut b = HostBuffers::new(256);
    let play = Playback {
        playing: false,
        bpm: 120.,
        beat: None,
    };
    assert_eq!(
        b.run(
            &mut p,
            play,
            &[note(true, 48), note(true, 55), note(true, 60)],
            true
        ),
        0
    );
    assert_eq!(&p.poly.as_ref().unwrap().syllables()[..3], &[0, 0, 0]);
    let pedal = |v| NoteEvent::MidiCC {
        timing: 0,
        channel: 0,
        cc: 64,
        value: v,
    };
    b.run(
        &mut p,
        play,
        &[pedal(1.), note(false, 48), note(false, 55), note(false, 60)],
        false,
    );
    for _ in 0..100 {
        b.run(&mut p, play, &[], false);
    }
    assert_eq!(p.poly.as_ref().unwrap().active(), 3);
    b.run(&mut p, play, &[pedal(0.)], false);
    for _ in 0..400 {
        assert_eq!(b.run(&mut p, play, &[], true), 0);
    }
    assert_eq!(p.poly.as_ref().unwrap().active(), 0);
    b.run(&mut p, play, &[note(true, 62)], false);
    assert_eq!(p.poly.as_ref().unwrap().syllables()[0], 1);
    COUNT.with(|c| c.set(Some(0)));
    p.reset();
    assert_eq!(COUNT.with(|c| c.replace(None).unwrap()), 0);
    assert_eq!(p.poly.as_ref().unwrap().active(), 0);
}
#[test]
fn poly_phrase_loop_audio_is_finite_and_block_size_independent() {
    fn render(size: usize) -> Vec<f32> {
        let mut p = poly_plugin(Mode::Loop, 4);
        let mut b = HostBuffers::new(size);
        let mut out = Vec::new();
        for pos in (0..32768).step_by(size) {
            let events = if pos == 0 {
                vec![
                    note(true, 48),
                    note(true, 55),
                    note(true, 60),
                    note(true, 64),
                ]
            } else {
                vec![]
            };
            assert_eq!(
                b.run(
                    &mut p,
                    Playback {
                        playing: true,
                        bpm: 120.,
                        beat: Some(pos as f64 / 24000.)
                    },
                    &events,
                    true
                ),
                0
            );
            out.extend_from_slice(&b.left);
        }
        out
    }
    let a = render(64);
    let b = render(1024);
    assert!(a.iter().all(|x| x.is_finite() && x.abs() <= 1.));
    assert!(a.iter().map(|x| x * x).sum::<f32>() > 0.01);
    assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6));
}
#[test]
fn silent_legacy_choir_does_not_advance_physical_engines() {
    let mut p = ChimeraPlugin::default();
    Arc::get_mut(&mut p.params).unwrap().lab.choir = control("Choir", 0.65, 0., 1.);
    p.prepare(48000.);
    let clocks = p
        .layers
        .as_ref()
        .unwrap()
        .engines
        .each_ref()
        .map(|e| e.clock());
    let mut b = HostBuffers::new(256);
    for _ in 0..100 {
        b.run(
            &mut p,
            Playback {
                playing: false,
                bpm: 120.,
                beat: None,
            },
            &[],
            false,
        );
    }
    assert_eq!(
        clocks,
        p.layers
            .as_ref()
            .unwrap()
            .engines
            .each_ref()
            .map(|e| e.clock())
    );
    b.run(
        &mut p,
        Playback {
            playing: false,
            bpm: 120.,
            beat: None,
        },
        &[note(true, 48)],
        false,
    );
    assert!(p.layers.as_ref().unwrap().engines[0].clock() > clocks[0]);
}

#[test]
fn poly_limits_channels_and_panic_work_at_multiple_sample_rates() {
    for rate in [44100., 96000.] {
        let mut p = poly_plugin(Mode::Vowels, 6);
        p.prepare(rate);
        let mut b = HostBuffers::new(128);
        let play = Playback {
            playing: false,
            bpm: 120.,
            beat: None,
        };
        let events: Vec<_> = (0..6)
            .map(|i| NoteEvent::NoteOn {
                timing: i as u32 * 8,
                voice_id: None,
                channel: i,
                note: 48 + i * 3,
                velocity: 0.8,
            })
            .collect();
        assert_eq!(b.run(&mut p, play, &events, true), 0);
        assert_eq!(p.poly.as_ref().unwrap().active(), 6);
        for _ in 0..80 {
            assert_eq!(b.run(&mut p, play, &[], true), 0);
            assert!(
                b.left
                    .iter()
                    .chain(&b.right)
                    .all(|x| x.is_finite() && x.abs() <= 1.)
            );
        }
        assert!(b.left.iter().any(|x| x.abs() > 1e-5));
        b.run(
            &mut p,
            play,
            &[NoteEvent::MidiCC {
                timing: 0,
                channel: 1,
                cc: 120,
                value: 0.,
            }],
            false,
        );
        assert_eq!(p.poly.as_ref().unwrap().active(), 5);
        assert_eq!(p.poly.as_ref().unwrap().keys()[1], None);
        Arc::get_mut(&mut p.params).unwrap().lab.voice_limit =
            IntParam::new("Limit", 2, IntRange::Linear { min: 2, max: 6 });
        for _ in 0..((rate * 2.) as usize / 128) {
            b.run(&mut p, play, &[], false);
        }
        assert_eq!(p.poly.as_ref().unwrap().active(), 1);
        p.panic.store(true, Ordering::Release);
        assert_eq!(b.run(&mut p, play, &[], true), 0);
        assert_eq!(p.poly.as_ref().unwrap().active(), 0);
        assert!(b.left.iter().all(|x| *x == 0.));
    }
}
