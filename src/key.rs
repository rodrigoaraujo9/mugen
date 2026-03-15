use crate::config::{A4_SEMITONES, BASE_FREQ, KEYBOARD_BASE_OCTAVE, SEMITONES_PER_OCTAVE};
use device_query::Keycode;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Note {
    C = 0,
    Db = 1,
    D = 2,
    Eb = 3,
    E = 4,
    F = 5,
    Gb = 6,
    G = 7,
    Ab = 8,
    A = 9,
    Bb = 10,
    B = 11,
}

impl Note {
    pub const ALL: [Note; 12] = [
        Note::C,
        Note::Db,
        Note::D,
        Note::Eb,
        Note::E,
        Note::F,
        Note::Gb,
        Note::G,
        Note::Ab,
        Note::A,
        Note::Bb,
        Note::B,
    ];

    #[inline]
    pub const fn semitone(self) -> i32 {
        self as i32
    }

    #[inline]
    pub const fn from_semitone(semitone: i32) -> Self {
        Self::ALL[semitone.rem_euclid(SEMITONES_PER_OCTAVE) as usize]
    }

    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            Note::C => "C",
            Note::Db => "Db",
            Note::D => "D",
            Note::Eb => "Eb",
            Note::E => "E",
            Note::F => "F",
            Note::Gb => "Gb",
            Note::G => "G",
            Note::Ab => "Ab",
            Note::A => "A",
            Note::Bb => "Bb",
            Note::B => "B",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub note: Note,
    pub octave: i32,
}

impl Key {
    #[inline]
    pub const fn new(note: Note, octave: i32) -> Self {
        Self { note, octave }
    }

    #[inline]
    pub const fn absolute_semitone(self) -> i32 {
        self.octave * SEMITONES_PER_OCTAVE + self.note.semitone()
    }

    #[inline]
    pub fn frequency(self) -> f32 {
        let semitone_diff = self.absolute_semitone() - A4_SEMITONES;
        BASE_FREQ * 2.0f32.powf(semitone_diff as f32 / 12.0)
    }

    #[inline]
    pub const fn transpose(self, semitones: i32) -> Self {
        let absolute = self.absolute_semitone() + semitones;
        let octave = absolute.div_euclid(SEMITONES_PER_OCTAVE);
        let note = Note::from_semitone(absolute);
        Self { note, octave }
    }

    pub fn from_keycode(keycode: Keycode) -> Option<Self> {
        let base = KEYBOARD_BASE_OCTAVE;

        Some(match keycode {
            Keycode::A => Self::new(Note::C, base),
            Keycode::W => Self::new(Note::Db, base),
            Keycode::S => Self::new(Note::D, base),
            Keycode::E => Self::new(Note::Eb, base),
            Keycode::D => Self::new(Note::E, base),
            Keycode::F => Self::new(Note::F, base),
            Keycode::T => Self::new(Note::Gb, base),
            Keycode::G => Self::new(Note::G, base),
            Keycode::Y => Self::new(Note::Ab, base),
            Keycode::H => Self::new(Note::A, base),
            Keycode::U => Self::new(Note::Bb, base),
            Keycode::J => Self::new(Note::B, base),
            Keycode::K => Self::new(Note::C, base + 1),
            Keycode::O => Self::new(Note::Db, base + 1),
            Keycode::L => Self::new(Note::D, base + 1),
            Keycode::P => Self::new(Note::Eb, base + 1),
            Keycode::Semicolon => Self::new(Note::E, base + 1),
            Keycode::Apostrophe => Self::new(Note::F, base + 1),
            _ => return None,
        })
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.note.name(), self.octave)
    }
}
