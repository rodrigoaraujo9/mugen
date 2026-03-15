use crate::audio::{self, AudioCommand};
use crate::config::{SAMPLE_RATE, TICK};
use crate::key::Key;
use crate::nodes::adsr::AdsrNode;
use crate::patch::{Gate, Node};
use crate::play::state::{PlayState, RuntimeState};
use device_query::{DeviceQuery, DeviceState, Keycode};
use rodio::Sink;
use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::sleep;
use std::time::Duration;
use tokio::{signal::ctrl_c, task};

enum RuntimeEvent {
    KeysChanged(HashSet<Keycode>),
    Exit,
}

#[inline]
fn sync_snapshot(tx: &tokio::sync::watch::Sender<audio::AudioSnapshot>, rt: &RuntimeState) {
    let _ = tx.send(rt.snapshot());
}

async fn play_note(play: &mut PlayState, rt: &RuntimeState, keycode: Keycode) {
    let Some(key) = Key::from_keycode(keycode) else {
        return;
    };

    let freq = key.transpose(rt.octave_offset * 12).frequency();
    let gate: Gate = Arc::new(AtomicBool::new(true));

    let sink = Sink::connect_new(play.stream.mixer());
    sink.set_volume(rt.volume);

    if rt.muted {
        sink.pause();
    }

    let src = rt.patch.create(freq);
    let src = AdsrNode::new(rt.adsr(), SAMPLE_RATE, gate.clone()).apply(src);

    sink.append(src);
    play.active_sinks
        .entry(keycode)
        .or_default()
        .push((sink, gate));
}

async fn restart_active_notes(play: &mut PlayState, rt: &RuntimeState) {
    let held: Vec<_> = rt.held_keys.iter().copied().collect();

    play.kill_all();
    for key in held {
        play_note(play, rt, key).await;
    }
}

#[inline]
fn cycle_generator(rt: &RuntimeState) {
    let next = rt.generator_kind().toggle();
    rt.generator.set_kind(next);
}

pub async fn runtime(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    focused: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = audio::get_handle().await;
    let (mut cmd_rx, snapshot_tx, held_keys_tx, initial) = audio::take_runtime_io().await;

    let mut rt = RuntimeState::new(initial);
    let mut play = PlayState::new()?;

    sync_snapshot(&snapshot_tx, &rt);

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_bg = Arc::clone(&stop_flag);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<RuntimeEvent>();

    let focused_bg = Arc::clone(&focused);
    let poll_handle = task::spawn_blocking(move || {
        let device_state = DeviceState::new();
        let mut prev = HashSet::new();
        let mut was_focused = true;

        loop {
            if stop_flag_bg.load(Ordering::Relaxed) {
                let _ = tx.send(RuntimeEvent::Exit);
                break;
            }

            sleep(Duration::from_millis(TICK));

            let is_focused = focused_bg.load(Ordering::Relaxed);

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
    });

    let ctrl_c = ctrl_c();
    tokio::pin!(ctrl_c);

    let mut last_keys: HashSet<Keycode> = HashSet::new();

    loop {
        tokio::select! {
            _ = &mut ctrl_c => break,

            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }

            msg = rx.recv() => {
                match msg {
                    Some(RuntimeEvent::KeysChanged(now)) => {
                        let toggle_b =
                            now.contains(&Keycode::B) && !last_keys.contains(&Keycode::B);

                        let musical_now: HashSet<Keycode> =
                            now.iter().copied().filter(|k| *k != Keycode::B).collect();

                        let musical_prev: HashSet<Keycode> =
                            last_keys.iter().copied().filter(|k| *k != Keycode::B).collect();

                        rt.held_keys = musical_now.clone();
                        let _ = held_keys_tx.send(rt.held_keys.clone());

                        if toggle_b {
                            cycle_generator(&rt);
                            sync_snapshot(&snapshot_tx, &rt);
                            restart_active_notes(&mut play, &rt).await;
                        }

                        for key in musical_now.difference(&musical_prev) {
                            play_note(&mut play, &rt, *key).await;
                        }

                        for key in musical_prev.difference(&musical_now) {
                            play.stop_note(*key);
                        }

                        play.cleanup_finished();
                        last_keys = now;
                    }

                    Some(RuntimeEvent::Exit) | None => break,
                }
            }

            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break; };

                match cmd {
                    AudioCommand::SetVolume(v) => {
                        rt.volume = v.clamp(0.0, 2.0);
                        play.set_all_volume(rt.volume);
                        sync_snapshot(&snapshot_tx, &rt);
                    }

                    AudioCommand::SetMuted(m) => {
                        rt.muted = m;
                        play.set_all_muted(m);
                        sync_snapshot(&snapshot_tx, &rt);
                    }

                    AudioCommand::SetGeneratorKind(kind) => {
                        rt.generator.set_kind(kind);
                        sync_snapshot(&snapshot_tx, &rt);
                        restart_active_notes(&mut play, &rt).await;
                    }

                    AudioCommand::SetAdsr(adsr) => {
                        *rt.adsr.write().unwrap() = adsr;
                        sync_snapshot(&snapshot_tx, &rt);
                        restart_active_notes(&mut play, &rt).await;
                    }

                    AudioCommand::SetLfo(params) => {
                        rt.lfo.set_all(params);
                        sync_snapshot(&snapshot_tx, &rt);
                    }

                    AudioCommand::SetLowPass(params) => {
                        rt.lowpass.set_all(params);
                        sync_snapshot(&snapshot_tx, &rt);
                    }

                    AudioCommand::SetOctave(octave) => {
                        rt.octave_offset = octave;
                        restart_active_notes(&mut play, &rt).await;
                    }
                }

                play.cleanup_finished();
            }
        }
    }

    stop_flag.store(true, Ordering::Relaxed);
    play.kill_all();
    let _ = poll_handle.await;

    Ok(())
}
