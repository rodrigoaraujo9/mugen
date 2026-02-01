use fundsp::math::pow;

pub const BASE_FREQ: f32 = 440.0; //base frequency in Hz-> A4

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn from_semitone(semitone: u32) -> Option<Self> {
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
            _ => None,
        }
    }

    pub fn semitone(&self) -> u32 {
        *self as u32
    }
}

pub struct Key {
    note: Note,  // 1 for C or 9 for Ab for example
    octave: i32, // corresponding mapping on keyboard (ex: 4 -> Ab4)
}

impl Key {
    pub fn new(note: Note, octave: i32) -> Self {
        Self { note, octave }
    }

    pub fn get_note(&self) -> Note {
        self.note
    }

    pub fn get_octave(&self) -> i32 {
        self.octave
    }

    pub fn frequency(&self) -> f32 {
        // use ref A4 (which is at octave 4, semitone 9) -> standart
        let a4_semitones = 4 * 12 + 9; // A4 is at semitone 57
        let this_semitones = self.octave * 12 + self.note.semitone() as i32;
        let diff = this_semitones - a4_semitones;
        BASE_FREQ * pow(2.0, diff as f32 / 12.0)
    }

    pub fn from_keycode(key: char) -> Option<Self> {
        match key.to_ascii_lowercase() {
            'a' => Some(Key::new(Note::C, 4)),
            's' => Some(Key::new(Note::D, 4)),
            'd' => Some(Key::new(Note::E, 4)),
            'f' => Some(Key::new(Note::F, 4)),
            'g' => Some(Key::new(Note::G, 4)),
            'h' => Some(Key::new(Note::A, 4)),
            'j' => Some(Key::new(Note::B, 4)),
            'k' => Some(Key::new(Note::C, 5)),
            'l' => Some(Key::new(Note::D, 5)),
            ';' => Some(Key::new(Note::E, 5)),
            '\'' => Some(Key::new(Note::F, 5)),
            'w' => Some(Key::new(Note::Db, 4)),
            'e' => Some(Key::new(Note::Eb, 4)),
            't' => Some(Key::new(Note::Gb, 4)),
            'y' => Some(Key::new(Note::Ab, 4)),
            'u' => Some(Key::new(Note::Bb, 4)),
            'o' => Some(Key::new(Note::Db, 5)),
            'p' => Some(Key::new(Note::Eb, 5)),
            _ => None,
        }
    }
}
