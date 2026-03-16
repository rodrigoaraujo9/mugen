//! Playback module responsible for the output stream and active voice lifecycle

mod player;

pub mod key;

pub use player::{ActiveVoice, Player};
