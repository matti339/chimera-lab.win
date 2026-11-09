# Chimera Lab

**A playable physical voice, from vowels and words to wet clicks and strange throat sounds.**

Chimera Lab is an experimental **Mac Apple Silicon VST3 instrument**, developed for Ableton Live. It is a community adaptation of [Jameson Hodge’s CHIMERA](https://github.com/jamesondh/chimera-vocal-synth), whose Rust engine models the vocal tract instead of playing back voice samples. Jameson and the upstream contributors created the underlying synthesizer. This fork adds the native plugin and its performance controls; it is not an official upstream release.

[Download / releases](https://github.com/Franzlazarus/chimera-lab/releases) · [Plugin guide](crates/chimera-plugin/README.md) · [Contribute](CONTRIBUTING.md) · [Report a bug](https://github.com/Franzlazarus/chimera-lab/issues)

## Play with it

- Drag the tongue, jaw, lips and larynx in an interactive 2D mouth view.
- Type words and loop them with the host tempo, or advance syllables with MIDI notes.
- Switch on **Poly choir** for independent chords, with a 2 / 4 / 6 voice limit.
- Explore tongue clicks, lip pops, inward breath, wet resonance, throat textures and rhythmic mouth gestures.
- Record mouth motion, morph between stored shapes, and save patches with your DAW session.
- Start from Classic, Slime chamber, Throat creature, Talking bass or the poly choir patches.

The basic Classic sound is protected by a sample-for-sample regression fixture. The plugin uses a 2D editor; the rotating 3D heads in upstream videos belong to the browser project.

## Install

Download the Mac arm64 archive from **Releases**, extract it, quit your DAW, then run `Install Chimera Lab.command`. Manual installation: copy `Chimera Lab.vst3` into `~/Library/Audio/Plug-Ins/VST3/`. Reopen Live and rescan under Settings → Plug-Ins if needed. Load **Chimera Community → Chimera Lab** on a MIDI track.

The build is ad-hoc signed, **not notarized**. AU, Intel Mac, Windows and Linux binaries are not provided. Existing Chimera Voice installations remain separate. Read the [guide](crates/chimera-plugin/README.md) for keyboard focus, lyric modes, rollback and detailed limitations.

## Current state: 0.4 alpha

Native workspace tests and Tracktion pluginval strictness 5 passed. Live loaded and saved 0.4. Broader host testing, manual polyphonic performance checks, accessibility, microphone routing and full MPE hardware testing are still needed. The browser test command is currently blocked on the development machine by the missing WASM standard library; TypeScript checks passed.

This is real physical synthesis and active voices are expensive. On the development machine, the offline 48 kHz / 256-sample benchmark used approximately 32% of one core’s realtime budget for two held notes, 62% for four and 94% for six. These are not whole-machine CPU figures or performance guarantees. Start with two voices. The large efficiency fix is silent choir processing: approximately 28.8% down to 0.07%. No reduced-fidelity DSP mode is used. See the guide for the benchmark method and full results.

## Build the VST3

On Apple Silicon macOS, install Rust/Cargo 1.89+, Python 3 and Apple Command Line Tools:

```sh
git clone https://github.com/Franzlazarus/chimera-lab.git
cd chimera-lab
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo build --locked --release -p chimera-plugin
python3 tools/plugin/bundle.py
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --locked -p chimera-plugin
```

The bundle is written to `target/bundled/Chimera Lab.vst3`. Cargo.lock pins dependencies; the keyboard-routing patch to egui-baseview is vendored. Browser and CLI instructions are preserved in [README-upstream.md](README-upstream.md).

## Help make it better

Musicians, DSP developers, Rust developers and interface designers are welcome. Useful first contributions are reproducible host tests, clearer interaction design, profiling active voice cost, and improving the expressive mouth textures without changing the Classic patch. Cross-platform ports are also welcome.

Start with [CONTRIBUTING.md](CONTRIBUTING.md), open an issue describing your idea, or submit a focused pull request. No need to be a DSP expert to help with testing and documentation. The adapter’s development was AI-assisted; changes still need review, reproducible checks and clear claims about what was actually tested.

## Credits and licenses

Based on upstream commit `092b16ce12b927d473c884ca2dfae53ce1b995df`, with upstream history retained.

- Original CHIMERA engine and browser application: [MIT](LICENSE), Jameson Hodge and upstream contributors.
- Community plugin adapter and its distribution scripts: [GPL-3.0-or-later](crates/chimera-plugin/licenses/GPL-3.0.txt), as declared in the plugin crate. The selected VST3 bindings impose GPL requirements on the plugin distribution.
- NIH-plug: ISC; patched egui-baseview: MIT. See [third-party notices](crates/chimera-plugin/licenses/).
- CMUdict and browser fonts retain their own notices. See [licensing scope](LICENSES.md).

Please credit upstream when sharing the project, and report Lab-specific bugs here rather than in Jameson’s issue tracker.
