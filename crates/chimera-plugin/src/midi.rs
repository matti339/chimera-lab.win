use nih_plug::prelude::NoteEvent;

#[derive(Clone, Copy, Default)]
struct Key {
    down: bool,
    sounding: bool,
    order: u64,
    velocity: f32,
}
pub struct Notes {
    keys: [[Key; 128]; 16],
    sustain: [bool; 16],
    pub bend: [f32; 16],
    serial: u64,
    ranked: [Option<(u8, u8, f32)>; 3],
    current: Option<(u8, u8, f32)>,
}
impl Default for Notes {
    fn default() -> Self {
        Self {
            keys: [[Key::default(); 128]; 16],
            sustain: [false; 16],
            bend: [0.0; 16],
            serial: 0,
            ranked: [None; 3],
            current: None,
        }
    }
}
impl Notes {
    pub fn active(&self) -> Option<(u8, u8, f32)> {
        self.current
    }
    pub fn ranked(&self, rank: usize) -> Option<(u8, u8, f32)> {
        self.ranked.get(rank).copied().flatten()
    }
    pub fn sounding(&self, channel: u8, note: u8) -> bool {
        self.keys[channel as usize][note as usize].sounding
    }
    fn refresh(&mut self) {
        let mut best = [(0_u64, 0_u8, 0_u8, 0_f32); 3];
        for (c, keys) in self.keys.iter().enumerate() {
            for (n, k) in keys.iter().enumerate() {
                if k.sounding {
                    for i in 0..3 {
                        if k.order > best[i].0 {
                            for j in (i + 1..3).rev() {
                                best[j] = best[j - 1];
                            }
                            best[i] = (k.order, c as u8, n as u8, k.velocity);
                            break;
                        }
                    }
                }
            }
        }
        self.ranked = best.map(|x| (x.0 > 0).then_some((x.1, x.2, x.3)));
        self.current = self.ranked[0];
    }
    pub fn handle(&mut self, event: NoteEvent<()>) {
        let Some(channel) = event.channel().map(usize::from).filter(|c| *c < 16) else {
            return;
        };
        match event {
            NoteEvent::NoteOn { note, velocity, .. } if note < 128 && velocity > 0.0 => {
                self.serial = self.serial.saturating_add(1);
                self.keys[channel][note as usize] = Key {
                    down: true,
                    sounding: true,
                    order: self.serial,
                    velocity: velocity.clamp(0.0, 1.0),
                };
            }
            NoteEvent::NoteOff { note, .. } | NoteEvent::NoteOn { note, .. } if note < 128 => {
                let key = &mut self.keys[channel][note as usize];
                key.down = false;
                key.sounding &= self.sustain[channel];
            }
            NoteEvent::Choke { note, .. } if note < 128 => {
                self.keys[channel][note as usize] = Key::default()
            }
            NoteEvent::MidiPitchBend { value, .. } => {
                self.bend[channel] = (value.clamp(0.0, 1.0) - 0.5) * 2.0
            }
            NoteEvent::MidiCC { cc: 64, value, .. } => {
                self.sustain[channel] = value >= 0.5;
                if !self.sustain[channel] {
                    for key in &mut self.keys[channel] {
                        if !key.down {
                            key.sounding = false;
                        }
                    }
                }
            }
            NoteEvent::MidiCC { cc: 120 | 123, .. } => {
                self.keys[channel] = [Key::default(); 128];
                self.sustain[channel] = false;
            }
            NoteEvent::MidiCC { cc: 121, .. } => {
                self.sustain[channel] = false;
                self.bend[channel] = 0.0;
                for key in &mut self.keys[channel] {
                    if !key.down {
                        key.sounding = false;
                    }
                }
            }
            _ => return,
        }
        self.refresh();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn on(channel: u8, note: u8) -> NoteEvent<()> {
        NoteEvent::NoteOn {
            timing: 0,
            voice_id: None,
            channel,
            note,
            velocity: 0.8,
        }
    }
    fn off(channel: u8, note: u8) -> NoteEvent<()> {
        NoteEvent::NoteOff {
            timing: 0,
            voice_id: None,
            channel,
            note,
            velocity: 0.0,
        }
    }
    #[test]
    fn last_note_reverts_and_channels_are_independent() {
        let mut n = Notes::default();
        n.handle(on(0, 60));
        n.handle(on(1, 60));
        n.handle(off(0, 60));
        assert_eq!(n.active(), Some((1, 60, 0.8)));
        n.handle(on(1, 64));
        n.handle(off(1, 64));
        assert_eq!(n.active(), Some((1, 60, 0.8)));
        n.handle(off(1, 60));
        assert_eq!(n.active(), None);
    }
    #[test]
    fn sustain_and_all_notes_off_do_not_stick() {
        let mut n = Notes::default();
        n.handle(on(0, 60));
        n.handle(NoteEvent::MidiCC {
            timing: 0,
            channel: 0,
            cc: 64,
            value: 1.0,
        });
        n.handle(off(0, 60));
        assert!(n.active().is_some());
        n.handle(NoteEvent::MidiCC {
            timing: 0,
            channel: 0,
            cc: 64,
            value: 0.0,
        });
        assert!(n.active().is_none());
        n.handle(on(0, 67));
        n.handle(NoteEvent::MidiCC {
            timing: 0,
            channel: 0,
            cc: 123,
            value: 0.0,
        });
        assert!(n.active().is_none());
    }
    #[test]
    fn zero_velocity_releases_and_bend_centres() {
        let mut n = Notes::default();
        n.handle(on(0, 60));
        n.handle(NoteEvent::NoteOn {
            timing: 0,
            voice_id: None,
            channel: 0,
            note: 60,
            velocity: 0.0,
        });
        assert!(n.active().is_none());
        n.handle(NoteEvent::MidiPitchBend {
            timing: 0,
            channel: 0,
            value: 0.5,
        });
        assert_eq!(n.bend[0], 0.0);
    }
}
