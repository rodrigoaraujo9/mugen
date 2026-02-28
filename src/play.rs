use crate::audio_patch::Node;
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

use crate::config::{TICK, SAMPLE_RATE, ADSR_ATTACK_S, ADSR_DECAY_S, ADSR_SUSTAIN, ADSR_RELEASE_S};
use crate::key::Key;
use crate::patches::basic::{basic_source, BasicKind};
use crate::fx::adsr::{Adsr, AdsrNode, Gate};
use crate::audio_system;
use crate::audio_patch::AudioSource;

pub type ActiveNote = (Sink, Gate);

pub struct PlayState {
    pub stream: OutputStream,
    pub active_sinks: HashMap<Keycode, Vec<ActiveNote>>,
}

impl PlayState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        Ok(Self { stream, active_sinks: HashMap::new() })
    }

    fn stop_note(&mut self, keycode: Keycode) {
        if let Some(voices) = self.active_sinks.get_mut(&keycode) {
            for (_sink, gate) in voices.iter_mut() {
                gate.store(false, Ordering::Relaxed);
            }
        }
    }

    fn kill_note(&mut self, keycode: Keycode) {
        if let Some(mut voices) = self.active_sinks.remove(&keycode) {
            for (sink, gate) in voices.drain(..) {
                gate.store(false, Ordering::Relaxed);
                sink.stop();
            }
        }
    }

    fn stop_all(&mut self) {
        for (_k, voices) in self.active_sinks.iter_mut() {
            for (_sink, gate) in voices.iter_mut() {
                gate.store(false, Ordering::Relaxed);
            }
        }
    }

    fn kill_all(&mut self) {
        for (_k, mut voices) in self.active_sinks.drain() {
            for (sink, gate) in voices.drain(..) {
                gate.store(false, Ordering::Relaxed);
                sink.stop();
            }
        }
    }

    fn cleanup_finished(&mut self) {
        self.active_sinks.retain(|_, voices| {
            voices.retain(|(sink, _)| !sink.empty());
            !voices.is_empty()
        });
    }

    fn set_all_volume(&mut self, v: f32) {
        for (_k, voices) in self.active_sinks.iter_mut() {
            for (sink, _gate) in voices.iter_mut() {
                sink.set_volume(v);
            }
        }
    }

    fn set_all_muted(&mut self, muted: bool) {
        for (_k, voices) in self.active_sinks.iter_mut() {
            for (sink, _gate) in voices.iter_mut() {
                if muted { sink.pause(); } else { sink.play(); }
            }
        }
    }
}

struct RuntimeState {
    volume: f32,
    muted: bool,
    adsr: Adsr,
    current_patch: Box<dyn AudioSource>,
    avaliable_patches: Vec<Box<dyn AudioSource>>,
    toggle_index: usize,
    held_keys: HashSet<Keycode>,
    octave_offset: i32,
}

fn publish_snapshot(tx: &tokio::sync::watch::Sender<audio_system::AudioSnapshot>, rt: &RuntimeState) {
    let _ = tx.send(audio_system::AudioSnapshot {
        volume: rt.volume,
        muted: rt.muted,
        patch_name: rt.current_patch.name().to_string(),
    });
}

async fn play_note(play_state: &mut PlayState, rt: &RuntimeState, keycode: Keycode) {
    let Some(key) = Key::from_keycode(keycode) else { return; };
    let freq = key.transpose(rt.octave_offset * 12).frequency();

    let gate: Gate = Arc::new(AtomicBool::new(true));

    let sink = Sink::connect_new(&play_state.stream.mixer());
    sink.set_volume(rt.volume);
    if rt.muted { sink.pause(); }

    let raw_src = rt.current_patch.create_source(freq);
    let adsr_node = AdsrNode::new(rt.adsr, SAMPLE_RATE, gate.clone());
    let src = adsr_node.apply(raw_src);
    sink.append(src);

    play_state.active_sinks.entry(keycode).or_default().push((sink, gate));
}

async fn restart_active_notes(play_state: &mut PlayState, rt: &RuntimeState) {
    play_state.kill_all();
    for &k in rt.held_keys.iter() {
        play_note(play_state, rt, k).await;
    }
}

fn cycle_patch(rt: &mut RuntimeState) {
    if rt.avaliable_patches.is_empty() {
        return;
    }
    rt.toggle_index = (rt.toggle_index + 1) % rt.avaliable_patches.len();
    rt.current_patch = basic_source(match rt.toggle_index {
        0 => BasicKind::Sine,
        1 => BasicKind::Saw,
        2 => BasicKind::Square,
        3 => BasicKind::Triangle,
        4 => BasicKind::Noise,
        _ => BasicKind::Sine,
    });
}

pub async fn run_audio(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    focused: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _handle = audio_system::get_handle().await.clone();
    let (mut cmd_rx, snapshot_tx, held_keys_tx, initial) = audio_system::take_runtime_channels().await;

    let mut rt = RuntimeState {
        volume: initial.volume,
        muted: initial.muted,
        adsr: Adsr::new(ADSR_ATTACK_S, ADSR_DECAY_S, ADSR_SUSTAIN, ADSR_RELEASE_S),
        current_patch: basic_source(BasicKind::Sine),
        avaliable_patches: vec![
            basic_source(BasicKind::Sine),
            basic_source(BasicKind::Saw),
            basic_source(BasicKind::Square),
            basic_source(BasicKind::Triangle),
            basic_source(BasicKind::Noise),
        ],
        toggle_index: 0,
        held_keys: HashSet::new(),
        octave_offset: 0,
    };

    let mut play_state = PlayState::new()?;
    publish_snapshot(&snapshot_tx, &rt);

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
                        rt.held_keys = now.iter().copied().filter(|k| *k != Keycode::B).collect();

                        let _ = held_keys_tx.send(rt.held_keys.clone());

                        if toggle_b {
                            cycle_patch(&mut rt);
                            publish_snapshot(&snapshot_tx, &rt);
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

                        play_state.cleanup_finished();
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
                        publish_snapshot(&snapshot_tx, &rt);
                    }
                    audio_system::AudioCommand::SetMuted(m) => {
                        rt.muted = m;
                        play_state.set_all_muted(rt.muted);
                        publish_snapshot(&snapshot_tx, &rt);
                    }
                    audio_system::AudioCommand::TogglePatch(patches) => {
                        if !patches.is_empty() {
                            rt.avaliable_patches = patches;
                            rt.toggle_index = 0;
                            rt.current_patch = basic_source(BasicKind::Sine);
                            publish_snapshot(&snapshot_tx, &rt);
                            restart_active_notes(&mut play_state, &rt).await;
                        }
                    }
                    audio_system::AudioCommand::SetPatch(patch) => {
                        rt.current_patch = patch;
                        publish_snapshot(&snapshot_tx, &rt);
                        restart_active_notes(&mut play_state, &rt).await;
                    }
                    audio_system::AudioCommand::SetAdsr(adsr) => {
                        rt.adsr = adsr;
                        publish_snapshot(&snapshot_tx, &rt);
                        restart_active_notes(&mut play_state, &rt).await;
                    }
                    audio_system::AudioCommand::SetOctave(o) => {
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
