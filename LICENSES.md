# Licensing scope

The root `LICENSE` preserves Jameson Hodge’s MIT license for the original CHIMERA engine and browser application, including the engine additions in this fork.

The community adapter in `crates/chimera-plugin/src/`, its Classic regression fixture and the scripts in `tools/plugin/` are licensed under GPL-3.0-or-later. The full GPL text is in `crates/chimera-plugin/licenses/GPL-3.0.txt`. The plugin links the selected GPL-licensed vst3-sys bindings, so the compiled plugin is distributed under GPL-3.0-or-later with corresponding source and build instructions.

The vendored `egui-baseview` directory retains its MIT license. NIH-plug retains ISC. Their notices, the engine MIT notice, the VST3 bindings notice and CMUdict notice are collected in `crates/chimera-plugin/licenses/`. Original browser assets retain the notices in `public/fonts/` and `public/data/CMUDICT-LICENSE.txt`.

New Lab documentation is available under MIT. Third-party material is not relicensed by this statement. Preserve notices when redistributing, and include or provide the corresponding source for the exact plugin binary you distribute. Cargo.lock and the pinned Git dependency revisions are part of that source.
