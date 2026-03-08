use device_query::Keycode;
use std::fmt;
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

pub const fn note_semitone(note: Note) -> i32 {
    note as i32
}

pub const fn note_from_semitone(semitone: u32) -> Option<Note> {
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

pub const fn note_name(note: Note) -> &'static str {
    match note {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub note: Note,
    pub octave: i32,
}

pub const fn create_key(note: Note, octave: i32) -> Key {
    Key { note, octave }
}

pub const fn key_absolute_semitone(key: Key) -> i32 {
    key.octave * SEMITONES_PER_OCTAVE + note_semitone(key.note)
}

pub fn key_frequency(key: Key) -> f32 {
    let semitone_diff = key_absolute_semitone(key) - A4_SEMITONES;
    BASE_FREQ * 2.0f32.powf(semitone_diff as f32 / 12.0)
}

pub const fn key_transpose(key: Key, semitones: i32) -> Key {
    let new_absolute = key_absolute_semitone(key) + semitones;
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

    create_key(new_note, new_octave)
}

pub fn key_from_keycode(keycode: Keycode) -> Option<Key> {
    let base = KEYBOARD_BASE_OCTAVE;
    match keycode {
        Keycode::A => Some(create_key(Note::C, base)),
        Keycode::S => Some(create_key(Note::D, base)),
        Keycode::D => Some(create_key(Note::E, base)),
        Keycode::F => Some(create_key(Note::F, base)),
        Keycode::G => Some(create_key(Note::G, base)),
        Keycode::H => Some(create_key(Note::A, base)),
        Keycode::J => Some(create_key(Note::B, base)),
        Keycode::K => Some(create_key(Note::C, base + 1)),
        Keycode::L => Some(create_key(Note::D, base + 1)),
        Keycode::Semicolon => Some(create_key(Note::E, base + 1)),
        Keycode::Apostrophe => Some(create_key(Note::F, base + 1)),
        Keycode::W => Some(create_key(Note::Db, base)),
        Keycode::E => Some(create_key(Note::Eb, base)),
        Keycode::T => Some(create_key(Note::Gb, base)),
        Keycode::Y => Some(create_key(Note::Ab, base)),
        Keycode::U => Some(create_key(Note::Bb, base)),
        Keycode::O => Some(create_key(Note::Db, base + 1)),
        Keycode::P => Some(create_key(Note::Eb, base + 1)),
        _ => None,
    }
}

pub fn key_to_string(key: Key) -> String {
    format!("{}{}", note_name(key.note), key.octave)
}

impl Key {
    #[inline]
    pub const fn new(note: Note, octave: i32) -> Self {
        create_key(note, octave)
    }

    #[inline]
    pub const fn absolute_semitone(self) -> i32 {
        key_absolute_semitone(self)
    }

    #[inline]
    pub fn frequency(self) -> f32 {
        key_frequency(self)
    }

    pub const fn transpose(self, semitones: i32) -> Self {
        key_transpose(self, semitones)
    }

    pub fn from_keycode(keycode: Keycode) -> Option<Self> {
        key_from_keycode(keycode)
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", key_to_string(*self))
    }
}
