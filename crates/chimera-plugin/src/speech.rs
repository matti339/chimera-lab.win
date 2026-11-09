//! Rich text planning is performed off the audio thread. Playback copies only bounded,
//! precompiled gesture buffers; neither parsing nor allocation occurs in process().
use chimera::{
    anatomy::human::BodyMix,
    control::score::{ControlKey, Gesture, Label},
    phonetics::{
        english::EnglishG2P,
        sing::{NoteOnOptions, SongOptions, SungNote, plan_note_off, plan_note_on, plan_song},
        syllables::lyrics_to_syllables,
    },
};
use std::{io::Read, sync::OnceLock};
pub const MAX_SYLLABLES: usize = 32;
pub const MAX_GESTURES: usize = 4000;
pub const MAX_NOTE: usize = 192;
pub const NOTE_TAG: f64 = 100.0;

pub struct Bank {
    pub phrase: Vec<Gesture>,
    pub on: [Vec<Gesture>; MAX_SYLLABLES],
    pub off: [Vec<Gesture>; MAX_SYLLABLES],
    pub labels: [Label; MAX_SYLLABLES],
    pub len: usize,
    pub revision: u64,
    pub beats: f64,
    pub boundaries: [f64; 33],
    pub held: [bool; 32],
    pub rests: [bool; 32],
}
impl Bank {
    pub fn new(realtime: bool) -> Self {
        Self {
            phrase: Vec::with_capacity(if realtime { MAX_GESTURES } else { 0 }),
            on: std::array::from_fn(|_| Vec::with_capacity(if realtime { MAX_NOTE } else { 0 })),
            off: std::array::from_fn(|_| Vec::with_capacity(if realtime { MAX_NOTE } else { 0 })),
            labels: [Label::default(); MAX_SYLLABLES],
            len: 0,
            revision: 0,
            beats: 0.,
            boundaries: [0.; 33],
            held: [false; 32],
            rests: [false; 32],
        }
    }
    pub fn copy_from(&mut self, other: &Self) {
        self.phrase.clear();
        self.phrase.extend_from_slice(&other.phrase);
        for i in 0..MAX_SYLLABLES {
            self.on[i].clear();
            self.on[i].extend_from_slice(&other.on[i]);
            self.off[i].clear();
            self.off[i].extend_from_slice(&other.off[i]);
        }
        self.len = other.len;
        self.labels = other.labels;
        self.revision = other.revision;
        self.beats = other.beats;
        self.boundaries = other.boundaries;
        self.held = other.held;
        self.rests = other.rests;
    }
}
fn dictionary() -> &'static EnglishG2P {
    static DICT: OnceLock<EnglishG2P> = OnceLock::new();
    DICT.get_or_init(|| {
        let bytes = include_bytes!("../../../public/data/cmudict.txt.gz");
        let mut text = String::new();
        flate2::read::GzDecoder::new(&bytes[..])
            .read_to_string(&mut text)
            .expect("bundled dictionary");
        let mut dict = EnglishG2P::new();
        dict.load_dict(&text);
        dict
    })
}
fn remove_pitch(gestures: &mut [Gesture]) {
    for g in gestures {
        g.targets.remove(ControlKey::F0);
        g.targets.remove(ControlKey::VIBRATO);
    }
}
#[cfg(test)]
pub fn compile(text: &str) -> Result<Bank, String> {
    compile_for(
        text,
        BodyMix::MALE,
        &super::lab::Session::default(),
        false,
        120.,
        1.,
        0.,
        1.,
    )
}
pub fn compile_for(
    text: &str,
    body: BodyMix,
    session: &super::lab::Session,
    natural: bool,
    bpm: f64,
    step: f64,
    swing: f64,
    emphasis: f64,
) -> Result<Bank, String> {
    if text.chars().count() > 240 {
        return Err("Keep a phrase to 240 characters or fewer.".into());
    }
    let mut syllables = lyrics_to_syllables(text, dictionary());
    for (i, syl) in syllables.iter_mut().enumerate().take(32) {
        if let Some(word) = session
            .pronunciation
            .get(i)
            .filter(|s| !s.trim().is_empty())
        {
            if let Some(mut replacement) =
                lyrics_to_syllables(word, dictionary()).into_iter().next()
            {
                replacement.text = syl.text.clone();
                *syl = replacement;
            }
        }
    }
    if syllables.is_empty() {
        return Err("Type at least one English word.".into());
    }
    if syllables.len() > MAX_SYLLABLES {
        return Err(format!(
            "This phrase has {} syllables; the limit is 32.",
            syllables.len()
        ));
    }
    let mut bank = Bank::new(false);
    bank.len = syllables.len();
    for i in 0..bank.len {
        bank.boundaries[i + 1] = bank.boundaries[i]
            + if natural {
                session.durations[i].clamp(0.25, 8.) as f64
                    * (1. + if i % 2 == 0 { swing } else { -swing })
            } else {
                1.
            };
        bank.held[i] = syllables[i].melisma == Some(true);
        bank.rests[i] = session.rests[i];
    }
    bank.beats = bank.boundaries[bank.len];
    for (i, syl) in syllables.iter().enumerate() {
        bank.labels[i] = Label::new(&syl.text);
        bank.on[i] = plan_note_on(
            syl,
            60.0,
            &NoteOnOptions {
                body,
                vibrato: 0.0,
                tag: NOTE_TAG,
                legato_from: None,
            },
        );
        bank.off[i] = plan_note_off(syl, 60.0, body);
        remove_pitch(&mut bank.on[i]);
        remove_pitch(&mut bank.off[i]);
        if bank.on[i].len() > MAX_NOTE || bank.off[i].len() > MAX_NOTE {
            return Err("That syllable is too complex; try splitting it with a hyphen.".into());
        }
    }
    let notes: Vec<_> = syllables
        .clone()
        .into_iter()
        .map(|syl| SungNote {
            midi: 60.0,
            beats: 1.0,
            syl,
        })
        .collect();
    if natural {
        let seconds = 60. / bpm * step;
        let mut begin = 0;
        while begin < bank.len {
            if bank.rests[begin] {
                begin += 1;
                continue;
            }
            let mut end = begin + 1;
            while end < bank.len && !bank.rests[end] {
                end += 1;
            }
            let group: Vec<_> = (begin..end)
                .map(|i| SungNote {
                    midi: 60.,
                    beats: (bank.boundaries[i + 1] - bank.boundaries[i]) * step,
                    syl: syllables[i].clone(),
                })
                .collect();
            let plan = plan_song(
                &group,
                &SongOptions {
                    body,
                    bpm,
                    t0: bank.boundaries[begin] * seconds + 0.12_f64.min(seconds * 0.2),
                    vibrato: 0.,
                },
            );
            bank.phrase.extend(plan.gestures);
            begin = end;
        }
        for g in &mut bank.phrase {
            g.t0 = g.t0.max(0.) / seconds;
            g.t1 = g.t1.max(0.) / seconds;
            g.t1 = g.t1.min(bank.beats);
            g.ramp_in /= seconds;
            g.ramp_out /= seconds;
            g.release_hold = g.release_hold.map(|v| v / seconds);
            g.tag = None;
            if g.strength >= 20. {
                g.strength *= emphasis;
            }
        }
        bank.phrase.retain(|g| g.t1 > g.t0);
    } else {
        bank.phrase = plan_song(
            &notes,
            &SongOptions {
                body,
                bpm: 60.0,
                t0: 0.2,
                vibrato: 0.0,
            },
        )
        .gestures;
    }
    if bank.phrase.len() > MAX_GESTURES {
        return Err("Phrase too complex; use fewer words.".into());
    }
    // Normalize the complete utterance (including onset and breath release) to N beats.
    // The audio thread scales these beat-domain gestures by the live BPM and selected division.
    if !natural {
        let start = bank
            .phrase
            .iter()
            .map(|g| g.t0)
            .fold(f64::INFINITY, f64::min);
        let end = bank.phrase.iter().map(Gesture::end).fold(0.0, f64::max);
        let scale = bank.len as f64 / (end - start).max(0.01);
        for g in &mut bank.phrase {
            g.t0 = (g.t0 - start) * scale;
            g.t1 = (g.t1 - start) * scale;
            g.ramp_in *= scale;
            g.ramp_out *= scale;
            g.release_hold = g.release_hold.map(|v| v * scale);
            g.tag = None;
        }
    }
    remove_pitch(&mut bank.phrase);
    Ok(bank)
}
/// Scale a beat-domain phrase into seconds. Preallocated destination is essential.
pub fn scale_phrase(bank: &Bank, seconds_per_step: f64, out: &mut Vec<Gesture>) {
    out.clear();
    for g in &bank.phrase {
        let mut g = *g;
        g.t0 *= seconds_per_step;
        g.t1 *= seconds_per_step;
        g.ramp_in *= seconds_per_step;
        g.ramp_out *= seconds_per_step;
        g.release_hold = g.release_hold.map(|v| v * seconds_per_step);
        out.push(g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dictionary_and_limits_are_explicit() {
        let bank = compile("hello world").unwrap();
        assert_eq!(bank.len, 3);
        assert_eq!(bank.labels[0].as_str(), "hel");
        assert!(compile("").is_err());
        assert!(compile(&"a ".repeat(33)).is_err());
        assert!(
            bank.on
                .iter()
                .flatten()
                .all(|g| !g.targets.has(ControlKey::F0))
        );
    }
    #[test]
    fn phrase_duration_tracks_bpm_without_drifting() {
        let bank = compile("hello world").unwrap();
        for bpm in [60.0, 120.0, 173.0] {
            let mut out = Vec::with_capacity(MAX_GESTURES);
            scale_phrase(&bank, 60.0 / bpm, &mut out);
            let end = out.iter().map(Gesture::end).fold(0.0, f64::max);
            assert!((end - bank.len as f64 * 60.0 / bpm).abs() < 1e-10);
            assert!(out.iter().all(|g| g.t0 >= 0.0));
        }
    }
}
