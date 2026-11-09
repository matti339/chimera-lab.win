//! Preset files and bounded GUI undo. Never called on the audio thread.
use super::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
#[derive(Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub format: u32,
    pub name: String,
    pub values: BTreeMap<String, f32>,
    pub lyrics: String,
    pub session: lab::Session,
}
impl Snapshot {
    pub fn capture(p: &Controls, name: &str) -> Self {
        Self {
            format: 1,
            name: name.into(),
            values: p
                .param_map()
                .into_iter()
                .map(|(id, ptr, _)| (id, unsafe { ptr.unmodulated_normalized_value() }))
                .collect(),
            lyrics: p.lyrics.lock().unwrap().clone(),
            session: p.lab.session.lock().unwrap().clone(),
        }
    }
    pub fn apply(&self, p: &Controls, setter: &ParamSetter, shared: &Shared) {
        for (id, ptr, _) in p.param_map() {
            // Older preset files predate polyphony; loading one must restore the classic mode.
            let value = self.values.get(&id).copied().or_else(|| match id.as_str() {
                "lab_poly" => Some(0.),
                "lab_voice_limit" => Some(0.5),
                _ => None,
            });
            if let Some(v) = value.filter(|v| v.is_finite()) {
                unsafe {
                    setter.raw_context.raw_begin_set_parameter(ptr);
                    setter
                        .raw_context
                        .raw_set_parameter_normalized(ptr, v.clamp(0., 1.));
                    setter.raw_context.raw_end_set_parameter(ptr);
                }
            }
        }
        *p.lyrics.lock().unwrap() = self.lyrics.chars().take(240).collect();
        let mut session = self.session.clone();
        session.sanitize();
        *p.lab.session.lock().unwrap() = session;
        shared.session_revision.fetch_add(1, Ordering::Release);
    }
    pub fn fingerprint(&self) -> String {
        serde_json::to_string(&(self.values.clone(), &self.lyrics, &self.session))
            .unwrap_or_default()
    }
}
pub fn folder() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Library/Application Support/Chimera Lab/Presets")
}
fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .take(64)
        .collect();
    if s.trim().is_empty() {
        "My voice".into()
    } else {
        s.trim().into()
    }
}
pub fn save(s: &Snapshot) -> Result<String, String> {
    let dir = folder();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", slug(&s.name)));
    let tmp = path.with_extension("tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(tmp, &path).map_err(|e| e.to_string())?;
    Ok(format!("Saved {}", s.name))
}
pub fn list() -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(folder())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let p = entry.path();
            (p.extension().and_then(|x| x.to_str()) == Some("json"))
                .then(|| p.file_stem().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    names.sort();
    names
}
pub fn load(name: &str) -> Result<Snapshot, String> {
    let path = folder().join(format!("{}.json", slug(name)));
    if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 1_000_000 {
        return Err("Preset is too large".into());
    }
    let s: Snapshot = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if s.format != 1 {
        return Err("Unsupported preset version".into());
    }
    Ok(s)
}
pub const FACTORY: &[&str] = &[
    "Classic 0.2",
    "Slime chamber",
    "Tongue clicks",
    "Lip suction",
    "Inward breath",
    "Throat creature",
    "Talking bass",
    "Soft choir",
    "Moving vowels",
    "Poly choir · 2 voices",
    "Poly choir · 4 voices",
];
pub fn factory(index: usize) -> Snapshot {
    let p = Controls::default();
    let mut s = Snapshot::capture(&p, FACTORY[index]);
    s.session.preset_name = s.name.clone();
    let mut change = |id: &str, value: f32| {
        if let Some((_, ptr, _)) = p.param_map().into_iter().find(|(key, _, _)| key == id) {
            s.values
                .insert(id.into(), unsafe { ptr.preview_normalized(value) });
        }
    };
    match index {
        1 => {
            change("lab_wet", 0.85);
            change("lab_noise_level", 0.7);
            change("lab_sequencer", 1.);
            change("lab_noise_tone", 0.16);
            change("lab_noise_decay", 0.3);
            s.session.steps = [7, 0, 8, 0, 0, 6, 0, 8, 7, 0, 0, 8, 0, 6, 8, 0];
        }
        2 => {
            change("lab_noise_level", 0.8);
            change("lab_sequencer", 1.);
            change("lab_noise_tone", 0.7);
            s.session.steps = [7, 0, 0, 7, 0, 7, 0, 0, 7, 0, 7, 0, 0, 0, 7, 0];
        }
        3 => {
            change("lab_noise_level", 0.8);
            change("lab_wet", 0.55);
            change("lab_sequencer", 1.);
            s.session.steps = [8, 0, 0, 0, 8, 0, 8, 0, 0, 0, 8, 0, 8, 0, 0, 0];
        }
        4 => {
            change("lab_noise_level", 0.7);
            change("lab_wet", 0.4);
            change("lab_sequencer", 1.);
            s.session.steps = [6, 0, 0, 0, 0, 0, 6, 0, 0, 6, 0, 0, 6, 0, 0, 0];
        }
        5 => {
            change("lab_fry", 0.65);
            change("lab_rough", 0.6);
            change("lab_ventricular", 0.65);
            change("lab_wet", 0.3);
            change("lab_movement", 0.35);
            change("lab_size", 0.6);
            change("vowel", 3.4);
        }
        6 => {
            change("lab_movement", 0.75);
            change("lab_size", 0.7);
            change("lab_fry", 0.2);
            change("vowel", 2.);
            change("lab_expression", 0.75);
        }
        7 => {
            change("lab_choir", 0.65);
            change("lab_spread", 0.8);
            change("lab_air", 0.3);
            change("lab_vibrato_delay", 0.5);
            change("lab_expression", 0.7);
        }
        8 => {
            change("lab_movement", 0.8);
            change("lab_wet", 0.15);
            change("vowel", 1.5);
        }
        9 | 10 => {
            change("lab_poly", 1.);
            change("lab_voice_limit", if index == 9 { 2. } else { 4. });
            change("lab_spread", 0.7);
            change("lab_air", 0.15);
            change("lab_expression", 0.5);
        }
        _ => {}
    }
    s
}
