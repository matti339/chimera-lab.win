# Contributing to Chimera Lab

Thanks for helping turn a fascinating physical voice engine into a useful instrument. This is an independent community fork of Jameson Hodge’s CHIMERA. Lab-specific issues and pull requests belong here; keep upstream credit and licenses intact.

## Useful places to start

- **Host testing:** fresh install, rescan, opening/closing the editor, automation, saving/reopening a Set, keyboard focus and MIDI sustain. Try a 2- or 4-note choir with independent releases. Report the exact host, macOS, chip, sample rate and buffer size.
- **Interface and accessibility:** identify confusing controls and propose a concrete before/after interaction. Keyboard navigation, text focus, scaling and contrast matter as much as appearance.
- **Performance:** profile active voice cost separately from idle cost and editor drawing. Include the same patch, MIDI input, machine, sample rate and buffer size before/after. Two and four voices are the practical starting points.
- **Expressive textures:** better control over clicks, pops, constrictions and transitions. Keep the neutral Classic sound unchanged unless a change is explicitly proposed and accepted.
- **Ports:** Windows, Linux, Intel Mac and additional plugin formats need build and host validation; they are not supported releases yet.

Open an issue to discuss a larger change before doing substantial work. Small bug fixes and documentation pull requests are welcome directly. Please avoid including private projects, copyrighted songs, credentials or personal paths in reports. A tiny synthetic MIDI reproduction is ideal.

## Development

Read [AGENTS.md](AGENTS.md) and the [engine contract](docs/rust-engine.md). Plugin sources are in `crates/chimera-plugin/src/`; packaging is in `tools/plugin/`. Build instructions and limitations are in the root README and plugin guide.

The audio callback must not allocate, block, perform file/network I/O or depend on buffer size. Preallocate voice state; keep UI and filesystem work off the realtime thread. Preserve parameter IDs and the Lab VST3 class ID so existing projects continue to load. Do not reuse Chimera Voice’s identity.

Run the appropriate checks:

```sh
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test --locked --workspace
bun run typecheck
bun run test
```

Browser tests need Bun and Rust’s `wasm32-unknown-unknown` target. Report an unavailable check explicitly instead of treating it as passed. For CPU comparisons:

```sh
DEVELOPER_DIR=/Library/Developer/CommandLineTools CHIMERA_CPU_PROFILE=1 cargo test --locked --release -p chimera-plugin cpu_profile_when_requested -- --nocapture
```

Run this on the parent and changed revision under comparable load. Describe whether a figure is callback budget, core CPU or total system CPU. Plugin builds should also pass pluginval and a real host smoke test. Never re-bless audio goldens just to make a failure disappear; discuss intentional sound changes first.

Keep pull requests focused: what was wrong, what changed, reproduction, validation and remaining limitations. Include a regression test for a fixed bug where practical. For UI work, include an actual screenshot rather than a mockup presented as working software.

AI-assisted contributions are welcome when clearly disclosed and understood by the contributor. Do not submit unreviewed code dumps or claim tests that were not run. Contributions follow the existing license of the component being changed; no copyright assignment is requested.
