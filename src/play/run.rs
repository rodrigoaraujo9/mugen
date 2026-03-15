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
    rt.basic_generator.set_kind(next);
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
    let stop_flag_bg = stop_flag.clone();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<
        Option<(HashSet<Keycode>, HashSet<Keycode>, bool)>,
    >();

    let focused_bg = focused.clone();
    let poll_handle = task::spawn_blocking(move || {
        let device_state = DeviceState::new();
        let mut prev = HashSet::new();
        let mut was_focused = true;

        loop {
            if stop_flag_bg.load(Ordering::Relaxed) {
                let _ = tx.send(None);
                break;
            }

            sleep(Duration::from_millis(TICK));

            let is_focused = focused_bg.load(Ordering::Relaxed);

            if !is_focused {
                if was_focused {
                    if !prev.is_empty() {
                        let _ = tx.send(Some((HashSet::new(), prev.clone(), false)));
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
                let _ = tx.send(None);
                break;
            }

            if now != prev {
                let toggle_b = now.contains(&Keycode::B) && !prev.contains(&Keycode::B);
                let _ = tx.send(Some((now.clone(), prev.clone(), toggle_b)));
                prev = now;
            }
        }
    });

    let ctrl_c = ctrl_c();
    tokio::pin!(ctrl_c);

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
                    Some(Some((now, prev, toggle_b))) => {
                        rt.held_keys = now.iter().copied().filter(|k| *k != Keycode::B).collect();
                        let _ = held_keys_tx.send(rt.held_keys.clone());

                        if toggle_b {
                            cycle_generator(&rt);
                            sync_snapshot(&snapshot_tx, &rt);
                            restart_active_notes(&mut play, &rt).await;
                        }

                        for key in now.difference(&prev) {
                            if *key != Keycode::B {
                                play_note(&mut play, &rt, *key).await;
                            }
                        }

                        for key in prev.difference(&now) {
                            if *key != Keycode::B {
                                play.stop_note(*key);
                            }
                        }

                        play.cleanup_finished();
                    }

                    Some(None) | None => break,
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
                        rt.basic_generator.set_kind(kind);
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
