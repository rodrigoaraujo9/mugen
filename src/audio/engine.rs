//! Audio engine runtime, responsible for input polling, command handling, state updates, and playback coordination

use crate::audio::{self, AudioCommand, AudioSnapshot, AudioState};
use crate::config::TICK;
use crate::key::Key;
use crate::patch::Gate;
use crate::play::Player;
use device_query::{DeviceQuery, DeviceState, Keycode};
use rodio::Sink;
use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::sleep,
    time::Duration,
};
use tokio::{signal::ctrl_c, task};

enum RuntimeEvent {
    KeysChanged(HashSet<Keycode>),
    Exit,
}

#[inline]
fn sync_snapshot(tx: &tokio::sync::watch::Sender<AudioSnapshot>, state: &AudioState) {
    let _ = tx.send(state.snapshot());
}

async fn spawn_voice(player: &mut Player, state: &AudioState, keycode: Keycode) {
    let Some(key) = Key::from_keycode(keycode) else {
        return;
    };

    let freq = key.transpose(state.octave_offset * 12).frequency();
    let gate: Gate = Arc::new(AtomicBool::new(true));

    let sink = Sink::connect_new(player.stream.mixer());
    sink.set_volume(state.volume);

    if state.muted {
        sink.pause();
    }

    sink.append(state.patch.voice(freq, gate.clone()));
    player.voices.entry(keycode).or_default().push((sink, gate));
}

async fn rebuild_voices(player: &mut Player, state: &AudioState) {
    let held: Vec<_> = state.held_keys.iter().copied().collect();
    player.kill_all();

    for key in held {
        spawn_voice(player, state, key).await;
    }
}

#[inline]
fn cycle_generator(state: &AudioState) {
    state.patch.osc.update(|p| p.kind = p.kind.toggle());
}

pub async fn run(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    focused: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = audio::client().await;
    let (mut cmd_rx, snapshot_tx, held_keys_tx, initial) = audio::take_engine_io().await;

    let mut state = AudioState::new(initial);
    let mut player = Player::new()?;
    sync_snapshot(&snapshot_tx, &state);

    let stop_flag = Arc::new(AtomicBool::new(false));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<RuntimeEvent>();

    let poll_handle = {
        let stop_flag = Arc::clone(&stop_flag);
        let focused = Arc::clone(&focused);

        task::spawn_blocking(move || {
            let device_state = DeviceState::new();
            let mut prev = HashSet::new();
            let mut was_focused = true;

            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    let _ = tx.send(RuntimeEvent::Exit);
                    break;
                }

                sleep(Duration::from_millis(TICK));
                let is_focused = focused.load(Ordering::Relaxed);

                if !is_focused {
                    if was_focused {
                        if !prev.is_empty() {
                            let _ = tx.send(RuntimeEvent::KeysChanged(HashSet::new()));
                            prev.clear();
                        }
                        was_focused = false;
                    }
                    continue;
                }

                if !was_focused {
                    prev = device_state.get_keys().into_iter().collect();
                    was_focused = true;
                    continue;
                }

                let now: HashSet<Keycode> = device_state.get_keys().into_iter().collect();

                if now.contains(&Keycode::Escape)
                    || (now.contains(&Keycode::C) && now.contains(&Keycode::LControl))
                {
                    let _ = tx.send(RuntimeEvent::Exit);
                    break;
                }

                if now != prev {
                    let _ = tx.send(RuntimeEvent::KeysChanged(now.clone()));
                    prev = now;
                }
            }
        })
    };

    let ctrl_c = ctrl_c();
    tokio::pin!(ctrl_c);

    let mut last_keys = HashSet::new();

    loop {
        tokio::select! {
            _ = &mut ctrl_c => break,

            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }

            msg = rx.recv() => match msg {
                Some(RuntimeEvent::KeysChanged(now)) => {
                    let toggle_b =
                        now.contains(&Keycode::B) && !last_keys.contains(&Keycode::B);

                    let now: HashSet<Keycode> =
                        now.iter().copied().filter(|k| *k != Keycode::B).collect();

                    let prev: HashSet<Keycode> =
                        last_keys.iter().copied().filter(|k| *k != Keycode::B).collect();

                    state.held_keys = now.clone();
                    let _ = held_keys_tx.send(state.held_keys.clone());

                    if toggle_b {
                        cycle_generator(&state);
                        sync_snapshot(&snapshot_tx, &state);
                        rebuild_voices(&mut player, &state).await;
                    }

                    for key in now.difference(&prev) {
                        spawn_voice(&mut player, &state, *key).await;
                    }

                    for key in prev.difference(&now) {
                        player.stop_note(*key);
                    }

                    player.cleanup_finished();
                    last_keys = now;
                }
                Some(RuntimeEvent::Exit) | None => break,
            },

            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break };

                match cmd {
                    AudioCommand::SetVolume(v) => {
                        state.volume = v.clamp(0.0, 2.0);
                        player.set_all_volume(state.volume);
                    }

                    AudioCommand::SetMuted(m) => {
                        state.muted = m;
                        player.set_all_muted(m);
                    }

                    AudioCommand::SetGeneratorKind(kind) => {
                        state.patch.osc.update(|p| p.kind = kind);
                        rebuild_voices(&mut player, &state).await;
                    }

                    AudioCommand::SetAdsr(adsr) => {
                        state.patch.adsr.set(adsr);
                    }

                    AudioCommand::SetLfo(params) => {
                        state.patch.lfo.set(params);
                    }

                    AudioCommand::SetLowPass(params) => {
                        state.patch.lowpass.set(params);
                    }

                    AudioCommand::SetOctave(octave) => {
                        state.octave_offset = octave;
                        rebuild_voices(&mut player, &state).await;
                    }
                }

                sync_snapshot(&snapshot_tx, &state);
                player.cleanup_finished();
            }
        }
    }

    stop_flag.store(true, Ordering::Relaxed);
    player.kill_all();
    let _ = poll_handle.await;

    Ok(())
}
