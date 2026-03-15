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

fn publish_snapshot(tx: &tokio::sync::watch::Sender<audio::AudioSnapshot>, rt: &RuntimeState) {
    let _ = tx.send(audio::AudioSnapshot {
        volume: rt.volume,
        muted: rt.muted,
        patch_name: rt.patch_name(),
        adsr: rt.adsr_value(),
        lfo: rt.lfo.params(),
        lowpass: rt.lowpass.params(),
    });
}

async fn play_note(play_state: &mut PlayState, rt: &RuntimeState, keycode: Keycode) {
    let Some(key) = Key::from_keycode(keycode) else {
        return;
    };

    let freq = key.transpose(rt.octave_offset * 12).frequency();
    let gate: Gate = Arc::new(AtomicBool::new(true));
    let sink = Sink::connect_new(play_state.stream.mixer());

    sink.set_volume(rt.volume);
    if rt.muted {
        sink.pause();
    }

    let raw_src = rt.patch.create(freq);
    let adsr = rt.adsr_value();
    let src = AdsrNode::new(adsr, SAMPLE_RATE, gate.clone()).apply(raw_src);

    sink.append(src);
    play_state
        .active_sinks
        .entry(keycode)
        .or_default()
        .push((sink, gate));
}

async fn restart_active_notes(play_state: &mut PlayState, rt: &RuntimeState) {
    play_state.kill_all();
    for &k in &rt.held_keys {
        play_note(play_state, rt, k).await;
    }
}

fn cycle_patch(rt: &RuntimeState) {
    let next = rt.basic_generator.params().kind.toggle();
    rt.basic_generator.set_kind(next);
}

pub async fn runtime(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    focused: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _handle = audio::get_handle().await.clone();
    let (mut cmd_rx, snapshot_tx, held_keys_tx, initial) = audio::take_runtime_channels().await;

    let mut rt = RuntimeState::new(initial);

    let mut play_state = PlayState::new()?;
    publish_snapshot(&snapshot_tx, &rt);

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_bg = stop_flag.clone();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<
        Option<(HashSet<Keycode>, HashSet<Keycode>, bool)>,
    >();

    let focused_bg = focused.clone();
    let poll_handle = task::spawn_blocking(move || {
        let device_state = DeviceState::new();
        let mut prev: HashSet<Keycode> = HashSet::new();
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
                        let empty: HashSet<Keycode> = HashSet::new();
                        let _ = tx.send(Some((empty, prev.clone(), false)));
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
                            cycle_patch(&rt);
                            publish_snapshot(&snapshot_tx, &rt);
                            restart_active_notes(&mut play_state, &rt).await;
                        }

                        for k in now.difference(&prev) {
                            if *k == Keycode::B {
                                continue;
                            }
                            play_note(&mut play_state, &rt, *k).await;
                        }

                        for k in prev.difference(&now) {
                            if *k == Keycode::B {
                                continue;
                            }
                            play_state.stop_note(*k);
                        }

                        play_state.cleanup_finished();
                    }
                    Some(None) | None => break,
                }
            }
            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break; };

                match cmd {
                    AudioCommand::SetVolume(v) => {
                        rt.volume = v.clamp(0.0, 2.0);
                        play_state.set_all_volume(rt.volume);
                        publish_snapshot(&snapshot_tx, &rt);
                    }
                    AudioCommand::SetMuted(m) => {
                        rt.muted = m;
                        play_state.set_all_muted(rt.muted);
                        publish_snapshot(&snapshot_tx, &rt);
                    }
                    AudioCommand::SetGeneratorKind(kind) => {
                        rt.basic_generator.set_kind(kind);
                        publish_snapshot(&snapshot_tx, &rt);
                        restart_active_notes(&mut play_state, &rt).await;
                    }
                    AudioCommand::SetAdsr(adsr) => {
                        *rt.adsr.write().unwrap() = adsr;
                        publish_snapshot(&snapshot_tx, &rt);
                        restart_active_notes(&mut play_state, &rt).await;
                    }
                    AudioCommand::SetLfo(params) => {
                        rt.lfo.set_all(params);
                        publish_snapshot(&snapshot_tx, &rt);
                    }
                    AudioCommand::SetLowPass(params) => {
                        rt.lowpass.set_all(params);
                        publish_snapshot(&snapshot_tx, &rt);
                    }
                    AudioCommand::SetOctave(o) => {
                        rt.octave_offset = o;
                        restart_active_notes(&mut play_state, &rt).await;
                    }
                }

                play_state.cleanup_finished();
            }
        }
    }

    stop_flag.store(true, Ordering::Relaxed);
    play_state.kill_all();
    let _ = poll_handle.await;

    Ok(())
}
