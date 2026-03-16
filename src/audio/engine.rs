//! Audio engine runtime: polls input, handles commands, updates state, and controls playback

use crate::audio::{self, Command, Snapshot, State};
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

enum Event {
    KeysChanged(HashSet<Keycode>),
    Exit,
}

#[inline]
fn publish_snapshot(tx: &tokio::sync::watch::Sender<Snapshot>, state: &State) {
    let _ = tx.send(state.snapshot());
}

async fn start_note(player: &mut Player, state: &State, keycode: Keycode) {
    let Some(key) = Key::from_keycode(keycode) else {
        return;
    };

    let freq = key.transpose(state.octave * 12).frequency();
    let gate: Gate = Arc::new(AtomicBool::new(true));

    let sink = Sink::connect_new(player.stream.mixer());
    sink.set_volume(state.volume);

    if state.muted {
        sink.pause();
    }

    sink.append(state.patch.build_voice(freq, gate.clone()));
    player.add_voice(keycode, sink, gate);
}

async fn restart_held_notes(player: &mut Player, state: &State) {
    let held: Vec<_> = state.held_keys.iter().copied().collect();

    player.kill_all();

    for key in held {
        start_note(player, state, key).await;
    }
}

#[inline]
fn toggle_wave(state: &State) {
    state.toggle_wave();
}

pub async fn run(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    focused: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = audio::client().await;
    let (mut cmd_rx, snapshot_tx, held_keys_tx, initial) = audio::take_engine_channels().await;

    let mut state = State::from_snapshot(initial);
    let mut player = Player::new()?;
    publish_snapshot(&snapshot_tx, &state);

    let stop_flag = Arc::new(AtomicBool::new(false));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

    let poll_handle = {
        let stop_flag = Arc::clone(&stop_flag);
        let focused = Arc::clone(&focused);

        task::spawn_blocking(move || {
            let device_state = DeviceState::new();
            let mut last_keys = HashSet::new();
            let mut was_focused = true;

            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    let _ = tx.send(Event::Exit);
                    break;
                }

                sleep(Duration::from_millis(TICK));
                let is_focused = focused.load(Ordering::Relaxed);

                if !is_focused {
                    if was_focused && !last_keys.is_empty() {
                        let _ = tx.send(Event::KeysChanged(HashSet::new()));
                        last_keys.clear();
                    }
                    was_focused = false;
                    continue;
                }

                if !was_focused {
                    last_keys = device_state.get_keys().into_iter().collect();
                    was_focused = true;
                    continue;
                }

                let now: HashSet<Keycode> = device_state.get_keys().into_iter().collect();

                if now.contains(&Keycode::Escape)
                    || (now.contains(&Keycode::C) && now.contains(&Keycode::LControl))
                {
                    let _ = tx.send(Event::Exit);
                    break;
                }

                if now != last_keys {
                    let _ = tx.send(Event::KeysChanged(now.clone()));
                    last_keys = now;
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
                Some(Event::KeysChanged(now)) => {
                    let toggle_b =
                        now.contains(&Keycode::B) && !last_keys.contains(&Keycode::B);

                    let now: HashSet<Keycode> =
                        now.iter().copied().filter(|k| *k != Keycode::B).collect();

                    let prev: HashSet<Keycode> =
                        last_keys.iter().copied().filter(|k| *k != Keycode::B).collect();

                    state.held_keys = now.clone();
                    let _ = held_keys_tx.send(state.held_keys.clone());

                    if toggle_b {
                        toggle_wave(&state);
                        publish_snapshot(&snapshot_tx, &state);
                        restart_held_notes(&mut player, &state).await;
                    }

                    for key in now.difference(&prev) {
                        start_note(&mut player, &state, *key).await;
                    }

                    for key in prev.difference(&now) {
                        player.stop_note(*key);
                    }

                    player.clear_finished();
                    last_keys = now;
                }
                Some(Event::Exit) | None => break,
            },

            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break };

                match cmd {
                    Command::SetVolume(v) => {
                        state.volume = v.clamp(0.0, 2.0);
                        player.set_volume(state.volume);
                    }

                    Command::SetMuted(m) => {
                        state.muted = m;
                        player.set_muted(m);
                    }

                    Command::SetWave(wave) => {
                        state.set_wave(wave);
                        restart_held_notes(&mut player, &state).await;
                    }

                    Command::SetAdsr(adsr) => {
                        state.set_adsr(adsr);
                    }

                    Command::SetGain(gain) => {
                        state.set_gain(gain);
                    }

                    Command::SetLfoAmp(lfo_amp) => {
                        state.set_lfo_amp(lfo_amp);
                    }

                    Command::SetLowPass(lowpass) => {
                        state.set_lowpass(lowpass);
                    }

                    Command::SetOctave(octave) => {
                        state.octave = octave;
                        restart_held_notes(&mut player, &state).await;
                    }
                }

                publish_snapshot(&snapshot_tx, &state);
                player.clear_finished();
            }
        }
    }

    stop_flag.store(true, Ordering::Relaxed);
    player.kill_all();
    let _ = poll_handle.await;

    Ok(())
}
