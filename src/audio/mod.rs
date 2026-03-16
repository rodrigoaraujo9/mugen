//! Public audio module -> commands, client API, runtime state, bus wiring, and engine loop

mod bus;
mod client;
mod command;
mod engine;
mod snapshot;
mod state;

pub use bus::{AudioBus, client, take_engine_io};
pub use client::AudioClient;
pub use command::AudioCommand;
pub use engine::run;
pub use snapshot::AudioSnapshot;
pub use state::AudioState;
