//! Shared audio bus that owns the engine channels and singleton client access

use crate::audio::{Client, Command, Snapshot};
use device_query::Keycode;
use std::{
    collections::HashSet,
    error::Error,
    io::{Error as IoError, ErrorKind},
};
use tokio::sync::{Mutex, OnceCell, mpsc, watch};

pub struct Bus {
    client: Client,
    commands: Mutex<Option<mpsc::UnboundedReceiver<Command>>>,
    snapshot_tx: watch::Sender<Snapshot>,
    held_keys_tx: watch::Sender<HashSet<Keycode>>,
}

static AUDIO: OnceCell<Bus> = OnceCell::const_new();

pub async fn client() -> &'static Client {
    &AUDIO
        .get_or_init(|| async {
            let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

            let snapshot = Snapshot::default();
            let (snapshot_tx, snapshot_rx) = watch::channel(snapshot);
            let (held_keys_tx, held_keys_rx) = watch::channel(HashSet::<Keycode>::new());

            Bus {
                client: Client::new(cmd_tx, snapshot_rx, held_keys_rx),
                commands: Mutex::new(Some(cmd_rx)),
                snapshot_tx,
                held_keys_tx,
            }
        })
        .await
        .client
}

pub async fn take_runtime_channels() -> Result<
    (
        mpsc::UnboundedReceiver<Command>,
        watch::Sender<Snapshot>,
        watch::Sender<HashSet<Keycode>>,
        Snapshot,
    ),
    Box<dyn Error + Send + Sync>,
> {
    let bus = AUDIO
        .get()
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "call audio::client() first"))?;

    let mut commands = bus.commands.lock().await;
    let cmd_rx = commands
        .take()
        .ok_or_else(|| IoError::new(ErrorKind::AlreadyExists, "audio engine already taken"))?;

    let snapshot = bus.snapshot_tx.borrow().clone();

    Ok((
        cmd_rx,
        bus.snapshot_tx.clone(),
        bus.held_keys_tx.clone(),
        snapshot,
    ))
}
