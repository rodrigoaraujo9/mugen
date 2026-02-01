use device_query::Keycode;
use crate::config::{BASE_FREQ, A4_SEMITONES, SEMITONES_PER_OCTAVE, KEYBOARD_BASE_OCTAVE};

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
    #[inline]
    pub const fn semitone(self) -> i32 {
        self as i32
    }

    pub const fn from_semitone(semitone: u32) -> Option<Self> {
        match semitone % 12 {
            0 => Some(Note::C),
            1 => Some(Note::Db),
            2 => Some(Note::D),
            3 => Some(Note::Eb),
            4 => Some(Note::E),
            5 => Some(Note::F),
            6 => Some(Note::Gb),
            7 => Some(Note::G),
            8 => Some(Note::Ab),
            9 => Some(Note::A),
            10 => Some(Note::Bb),
            11 => Some(Note::B),
            _ => unreachable!(),
        }
    }

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
    note: Note,
    octave: i32,
}

impl Key {
    #[inline]
    pub const fn new(note: Note, octave: i32) -> Self {
        Self { note, octave }
    }

    #[inline]
    pub const fn note(self) -> Note {
        self.note
    }

    #[inline]
    pub const fn octave(self) -> i32 {
        self.octave
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

    pub const fn transpose(self, semitones: i32) -> Self {
        let new_absolute = self.absolute_semitone() + semitones;
        let new_octave = new_absolute.div_euclid(SEMITONES_PER_OCTAVE);
        let new_note_value = new_absolute.rem_euclid(SEMITONES_PER_OCTAVE);

        let new_note = match new_note_value {
            0 => Note::C,
            1 => Note::Db,
            2 => Note::D,
            3 => Note::Eb,
            4 => Note::E,
            5 => Note::F,
            6 => Note::Gb,
            7 => Note::G,
            8 => Note::Ab,
            9 => Note::A,
            10 => Note::Bb,
            11 => Note::B,
            _ => unreachable!(),
        };

        Key::new(new_note, new_octave)
    }

    pub fn from_keycode(key: Keycode) -> Option<Self> {
        let base = KEYBOARD_BASE_OCTAVE;
        match key {
            Keycode::A => Some(Key::new(Note::C, base)),
            Keycode::S => Some(Key::new(Note::D, base)),
            Keycode::D => Some(Key::new(Note::E, base)),
            Keycode::F => Some(Key::new(Note::F, base)),
            Keycode::G => Some(Key::new(Note::G, base)),
            Keycode::H => Some(Key::new(Note::A, base)),
            Keycode::J => Some(Key::new(Note::B, base)),
            Keycode::K => Some(Key::new(Note::C, base + 1)),
            Keycode::L => Some(Key::new(Note::D, base + 1)),
            Keycode::Semicolon => Some(Key::new(Note::E, base + 1)),
            Keycode::Apostrophe => Some(Key::new(Note::F, base + 1)),
            Keycode::W => Some(Key::new(Note::Db, base)),
            Keycode::E => Some(Key::new(Note::Eb, base)),
            Keycode::T => Some(Key::new(Note::Gb, base)),
            Keycode::Y => Some(Key::new(Note::Ab, base)),
            Keycode::U => Some(Key::new(Note::Bb, base)),
            Keycode::O => Some(Key::new(Note::Db, base + 1)),
            Keycode::P => Some(Key::new(Note::Eb, base + 1)),
            _ => None,
        }
    }

    pub fn to_string(self) -> String {
        format!("{}{}", self.note.name(), self.octave)
    }
}
