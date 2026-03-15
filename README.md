<br />
<div align="center">
  <h3 align="center">mugen</h3>
  <p align="center">
    A terminal-based synthesizer in Rust.
  </p>
</div>

<div align="center">
  <img src="./assets/demo.gif" alt="Demo">
</div>

## About

This is a passion project to learn more about how synthesizers used in music composition and production work (mathematically) as well as dive into real-time systems in Rust. Synthesisers do a lot of compute and require that there is no noticeble latency from when keys are pressed or released to when the sound is played or stops. Therefore I am finding this quite a nice challenge!

You can play it live from your computer keyboard, layer notes, switch waveforms while holding notes, and tweak parameters in real time without rebuilding the modulation chain.

Right now it focuses on:

- real-time sound generation
- polyphonic playback
- live waveform switching while notes are held
- dynamic patch architecture with interchangeable generators and nodes
- per-note ADSR amplitude envelopes
- live LFO control
- live low-pass filter control
- real-time parameter changes while audio is running
- real-time display of held keys

## Available waveforms

- **Sine**
- **Saw**
- **Square**
- **Triangle**
- **Noise**

## How to play

- Use the keyboard (`A–L` row + `W/E/T/Y/U/O/P`) like a small piano
- Hold multiple keys to play chords
- Use **Tab** to switch panels
- Use **arrow keys** to select and change parameters
- Use **Space** to switch between **LFO** and **LowPass** in the modulation panel
- Press **Q** or **Ctrl+C** to quit

## Architecture

- **Generator** → produces the raw source signal
- **Node** → transforms the signal (modulation, filtering, effects)
- **Patch** → owns one generator and a dynamic chain of nodes
- Generator and node parameters are shared live so the UI can update them while notes are playing
- ADSR is applied per note, after the patch chain

## Screenshots

![Intro](assets/intro.png)
![UI](assets/ui.png)
