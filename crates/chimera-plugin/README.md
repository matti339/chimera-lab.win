# Chimera Lab 0.4 — clearer controls and optional polyphony

Community Apple Silicon VST3 adaptation of Jameson Hodge's CHIMERA. Lab 0.4 updates Lab 0.3 using the same plugin identity and existing parameter IDs. **Chimera Voice 0.2 remains separate and unchanged.** The neutral Classic output still matches its saved v0.2 audio reference sample for sample.

## Getting started

Load **Chimera Community → Chimera Lab** in Live. The top of the editor reads **LAB 04**. The main **Play** page now shows four performance controls: Air, Effort, Size and Movement. Drag the mouth handles, pick a vowel, and play MIDI notes. Output level and Stop sound are always accessible. Detailed settings expand only when needed.

Use **Factory voices** for Classic 0.2, Slime chamber, Tongue clicks, Lip suction, Inward breath, Throat creature, Talking bass, Soft choir, Moving vowels, and the new **Poly choir · 2 voices** / **Poly choir · 4 voices** patches.

For laptop keys, enable Live's Computer MIDI Keyboard and arm the track. The **Keyboard & display** menu contains the optional local-audition fallback and interface scaling. Local audition generates audio internally; it does not record MIDI into Live. Lyrics capture text input; Return applies the words and releases lyric focus.

## Poly choir

The **Poly choir** button beside Play mode switches on independent chord voices. Choose **2, 4 or 6** as the maximum voice count. Start with two for the lowest load, or four for normal chords. Six is a high-load option. Only notes actually sounding are synthesized; a six-voice limit does not continuously render six singers.

Each slot has its own physical voice, pitch, envelope, sustain and release. Releasing one note no longer retunes the remaining chord. When the limit is reached, a released slot is reused first, then the oldest held slot. A short transition softens voice stealing. MIDI channels, sustain, pressure, slide and bend remain independent by channel. A stolen held note is not resurrected automatically.

Polyphony uses a repeating mix of male, female and small physical voices: one singer per MIDI note. It bypasses the older Anatomy blend, Choir blend and automatic Harmony layers. Those controls remain available with Poly choir off. It does not multiply every note into another three-voice stack. Stereo spread and Choir detune work in both modes. Switch between classic and polyphonic modes between phrases: switching clears held notes.

- **Vowels:** every note gets its own sung vowel and release.
- **Words from keys** (the existing MIDI Syllables mode): notes advance the phrase; note-ons at exactly the same sample share a syllable. Sustain holds each voice. Staggered note-ons advance independently unless Hold/Legato is enabled. Released notes finish their own consonants.
- **Words on the beat** (the existing Phrase Loop mode): start Live and hold a chord; each note sings the same transport-synced phrase at its own pitch. Unlike classic Phrase Loop, polyphonic playback needs MIDI notes and does not generate an automatic root note.

The experimental microphone follower is available only with Poly choir off.

## Where the controls moved

| Page | What it does |
|---|---|
| Play | Four macros; Tone & envelope expands vowel, breath, brightness, tension, vibrato, glide, attack and release. Phrase rhythm/default pitch appear when relevant. |
| Mouth | Main tongue, jaw, lips and larynx controls; detailed tongue-tip, root and resonance controls expand below. |
| Textures | Wet resonance and percussion controls, ten sound pads, expandable Throat character and Rhythm sequencer. |
| Motion | Record a mouth movement loop and morph between four stored shapes. |
| Words | Hold, Legato, natural timing, syllable lengths/rests and alternative pronunciations. |
| Choir | Polyphony/voice limit or classic layering; expandable expression and microphone controls. |

Lyrics are hidden in Vowels mode. In either word mode, type up to 240 characters / 32 syllables, then Apply words or Return. Rewind returns to the start. Natural lyric timing enables custom phrase lengths, swing and consonant emphasis; select a syllable, edit it, then Apply phrasing. Enter a one-syllable alternative spelling rather than IPA. A standalone `~` extends the previous vowel. Dictionary parsing and synthesis run locally.

In **Textures**, pads and MIDI notes 24–33 trigger Kick, Hi-hat, Open hat, Snare, Rimshot, Inward snare, Tongue click, Lip pop, Crash and Hum, respectively. Percussion level must be above zero for those MIDI notes to become drum triggers. Enable the Rhythm sequencer and start Live or Audition pattern; click steps to cycle sounds, right-click for accents. The dot is a rest. Physical mouth gestures have preparation time, so audible transients may lag their triggers. Wet resonance is a short feedback effect, not a saliva simulation.

In **Motion**, start Live, choose the loop length, click Record one loop and move the mouth. Raise Motion playback depth afterward. Store shapes A–D, raise Mouth morph amount and move the XY pad. Motion, shapes, words and patterns save with presets and Sets.

Preset Save is in **Save…**. User presets are stored in `~/Library/Application Support/Chimera Lab/Presets`; the same name replaces a preset. **Explore** contains Mutate and its pitch/lyric-timing locks. Undo/Redo retains up to 32 editor snapshots separately from Live's undo. Loading a pre-0.4 user preset resets the new polyphony controls to their classic defaults.

The display uses one interactive 2D profile to reduce clutter and drawing work. It is not the rotating 3D view from the upstream video. With polyphony, the profile shows an active voice rather than six separate heads.

## CPU: measured benefits and limits

The main bug was silent choir layers still running their full physical models. They now stop rendering once the output envelope is silent. Note ranking is cached when MIDI changes instead of repeatedly scanning all channels during control updates. Disabled layers skip unnecessary control work. The editor caches unchanged geometry and draws one profile.

The header's **Audio %** measures smoothed audio-callback time against the time available for one buffer. It is not total machine CPU and excludes editor drawing. The voice count shows allocated note slots, including releases and held slots waiting for Phrase Loop transport.

Release-build results on the development Mac, 48 kHz / 256 samples, four-second sustained/idle runs, median of three:

| Case | Lab 0.3 | Lab 0.4 |
|---|---:|---:|
| Neutral idle | 0.072% | 0.062% |
| Solo sustained voice | 15.62% | 15.42% |
| Classic layered choir sustained | 44.69% | 45.44% |
| Classic choir, silent | 28.77% | 0.070% |
| Polyphonic, two held notes | — | 31.75% |
| Polyphonic, four held notes | — | 61.72% |
| Polyphonic, six held notes | — | 93.71% |
| Polyphonic, silent | — | 0.048% |

These are percentages of one core's realtime processing budget, not Activity Monitor percentages. The major improvement is idle cost; there is no demonstrated reduction in active physical-synthesis cost. Small active-mode differences are within the observed run/load variation. Six voices leave little headroom; additional percussion/effects, higher sample rates and other Live processing add work. Use a smaller voice limit for live performance and freeze/render dense passages if necessary. No reduced-fidelity synthesis or oversampling change was introduced.

## Validation and current limitations

9 November 2026:

- Rust workspace: **143 passed, 2 ignored**. Final plugin suite: **31 passed**. Keyboard routing: **2 passed**.
- Classic sample-for-sample reference and native upstream golden tests pass without re-blessing.
- New regressions cover independent releases, sustain, voice stealing, no resurrection of stolen keys, simultaneous-chord syllables, bounded voice limits, channel-specific all-sounds-off, panic, sample-rate changes, finite output and allocation-free callbacks/reset. Phrase-loop output is checked across 64- and 1024-sample buffers.
- Tracktion pluginval 1.0.4, strictness 5: **SUCCESS**, including final editor lifecycle, automation, saved state and bus tests at common sample rates/buffer sizes. A separate Steinberg validator was not run.
- TypeScript typecheck passed. Browser `bun run test` is blocked because this Mac lacks the `wasm32-unknown-unknown` Rust standard library; native VST3 tests pass separately.
- Ableton Live loaded 0.4 and saved its processor state successfully. A fresh-instance manual polyphony test and full visual editor inspection in Live remain outstanding; desktop automation timed out. Native callback tests and pluginval are not a substitute for those host-specific checks.
- Physical microphone routing, acoustic pitch tracking and full MPE-controller interaction were not manually tested. Microphone following remains experimental (roughly 65–700 Hz with analysis delay), not speech recognition or a vocoder.

## Research behind the changes

The layout applies [Nielsen Norman Group's progressive-disclosure guidance](https://www.nngroup.com/articles/progressive-disclosure/): keep frequent controls visible and reveal secondary settings on demand. The voice-limit control follows [Ableton's CPU optimization guidance](https://help.ableton.com/hc/en-us/articles/12911009486108-Optimizing-CPU-Intensive-Devices), which identifies polyphony and idle plugin load as practical CPU considerations. [u-he's Diva manual](https://u-he.com/downloads/manuals/plugins/diva/Diva-user-guide.pdf) provides a useful precedent for explicit voice/quality tradeoffs and cautions that plugin multicore processing can interact with host scheduling. Here the chosen implementation is a bounded, allocation-free voice pool; it does not introduce worker-thread synchronization into Live's audio callback.

## Install, source and rollback

Download the Mac archive from this fork's GitHub Releases and extract it. Run `Install Chimera Lab.command`, or copy `Chimera Lab.vst3` to `~/Library/Audio/Plug-Ins/VST3/`. The installer backs up an existing Lab bundle in `~/Library/Audio/Plug-Ins/Chimera Lab Backups/` before replacing it; restore a backup bundle to roll back. Quit Live before replacing a loaded plugin, then reopen it and rescan in Settings → Plug-Ins if needed. The separately identified Chimera Voice plugin is not replaced.

This is an experimental community adaptation, not an official upstream release. Development of the adapter was AI-assisted. Please report reproducible problems in this fork's issue tracker.

Apple Silicon only, locally ad-hoc signed, not notarized. AU/Intel/Windows binaries are not included. To rebuild from the corresponding source archive, use Rust/Cargo 1.89+, Python 3 and Apple Command Line Tools:

```sh
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo build --locked --release -p chimera-plugin
python3 tools/plugin/bundle.py
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --locked -p chimera-plugin
DEVELOPER_DIR=/Library/Developer/CommandLineTools CHIMERA_CPU_PROFILE=1 cargo test --locked --release -p chimera-plugin cpu_profile_when_requested -- --nocapture
```

Output: `target/bundled/Chimera Lab.vst3`. Copy `tools/plugin/install.command` beside that bundle to distribute the installer. Cargo.lock pins the dependencies; NIH-plug is pinned at de421011f41a6d10fc8c7a6084e4f4dee0143683 and the patched egui-baseview is included in the vendor directory.

Based on CHIMERA upstream commit 092b16ce12b927d473c884ca2dfae53ce1b995df. The engine is MIT; the community plugin adapter is GPL-3.0-or-later due to its selected VST3 bindings. NIH-plug is ISC; egui-baseview is MIT; CMUdict has its own included notice. Preserve the included licenses and corresponding-source requirements when redistributing.
