use device_query::{DeviceQuery, DeviceState, Keycode};
use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use rodio::stream::{OutputStream, OutputStreamBuilder};
use rodio::Sink;

use tokio::{signal::ctrl_c, task};

use crate::config::TICK;
use crate::key::Key;
use crate::patches::basic::{basic_source, BasicKind};
use crate::audio_system;

pub struct PlayState {
    pub stream: OutputStream,
    pub active_sinks: HashMap<Keycode, Sink>,
}

impl PlayState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        Ok(Self { stream, active_sinks: HashMap::new() })
    }

    fn stop_note(&mut self, keycode: Keycode) {
        if let Some(sink) = self.active_sinks.remove(&keycode) {
            sink.stop();
        }
    }

    fn stop_all(&mut self) {
        for (_, sink) in self.active_sinks.drain() {
            sink.stop();
        }
    }

    fn set_all_volume(&mut self, v: f32) {
        for sink in self.active_sinks.values_mut() {
            sink.set_volume(v);
        }
    }

    fn set_all_muted(&mut self, muted: bool) {
        if muted {
            for sink in self.active_sinks.values_mut() { sink.pause(); }
        } else {
            for sink in self.active_sinks.values_mut() { sink.play(); }
        }
    }
}

#[derive(Clone, Copy)]
struct RuntimeState {
    volume: f32,
    muted: bool,
    kind: BasicKind,
}

fn publish_snapshot(tx: &tokio::sync::watch::Sender<audio_system::AudioSnapshot>, rt: RuntimeState) {
    let _ = tx.send(audio_system::AudioSnapshot {
        volume: rt.volume,
        muted: rt.muted,
        kind: rt.kind,
    });
}

async fn play_note(play_state: &mut PlayState, rt: &RuntimeState, keycode: Keycode) {
    if play_state.active_sinks.contains_key(&keycode) {
        return;
    }

    let Some(key) = Key::from_keycode(keycode) else { return; };
    let freq = key.frequency();

    let sink = Sink::connect_new(&play_state.stream.mixer());
    sink.set_volume(rt.volume);
    if rt.muted { sink.pause(); }

    let src = basic_source(rt.kind);
    sink.append(src.create_source(freq));

    play_state.active_sinks.insert(keycode, sink);
}

async fn restart_active_notes(play_state: &mut PlayState, rt: &RuntimeState) {
    let keys: Vec<Keycode> = play_state.active_sinks.keys().copied().collect();
    for k in keys {
        play_state.stop_note(k);
        play_note(play_state, rt, k).await;
    }
}

pub async fn run_audio(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    focused: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _handle = audio_system::get_handle().await.clone();
    let (mut cmd_rx, snapshot_tx, initial) = audio_system::take_runtime_channels().await;

    let mut rt = RuntimeState {
        volume: initial.volume,
        muted: initial.muted,
        kind: initial.kind,
    };

    let mut play_state = PlayState::new()?;
    publish_snapshot(&snapshot_tx, rt);

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_bg = stop_flag.clone();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Option<(HashSet<Keycode>, HashSet<Keycode>, bool)>>();

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

            std::thread::sleep(Duration::from_millis(TICK));

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
                if *shutdown.borrow() { break; }
            }

            msg = rx.recv() => {
                match msg {
                    Some(Some((now, prev, toggle_b))) => {
                        if toggle_b {
                            rt.kind = rt.kind.next();
                            publish_snapshot(&snapshot_tx, rt);
                            restart_active_notes(&mut play_state, &rt).await;
                        }

                        for k in now.difference(&prev) {
                            if *k == Keycode::B { continue; }
                            play_note(&mut play_state, &rt, *k).await;
                        }

                        for k in prev.difference(&now) {
                            if *k == Keycode::B { continue; }
                            play_state.stop_note(*k);
                        }
                    }
                    Some(None) | None => break,
                }
            }

            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break; };

                match cmd {
                    audio_system::AudioCommand::SetVolume(v) => {
                        rt.volume = v.clamp(0.0, 2.0);
                        play_state.set_all_volume(rt.volume);
                        publish_snapshot(&snapshot_tx, rt);
                    }
                    audio_system::AudioCommand::SetMuted(m) => {
                        rt.muted = m;
                        play_state.set_all_muted(rt.muted);
                        publish_snapshot(&snapshot_tx, rt);
                    }
                    audio_system::AudioCommand::ToggleSource => {
                        rt.kind = rt.kind.next();
                        publish_snapshot(&snapshot_tx, rt);
                        restart_active_notes(&mut play_state, &rt).await;
                    }
                    audio_system::AudioCommand::SetSource(kind) => {
                        rt.kind = kind;
                        publish_snapshot(&snapshot_tx, rt);
                        restart_active_notes(&mut play_state, &rt).await;
                    }
                }
            }
        }
    }

    stop_flag.store(true, Ordering::Relaxed);
    play_state.stop_all();
    let _ = poll_handle.await;
    Ok(())
}
