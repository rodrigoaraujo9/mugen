# Mugen

I am building a **terminal-based synthesizer** in Rust.

You can play it live from your computer keyboard, switch waveforms as you go, and layer notes like a real instrument. The goal is not a DAW — it’s a **playable synth** that feels immediate and hackable.

Right now it focuses on:

- real-time sound generation
- polyphonic playing
- switching sound character while notes are held
- adsr manipulation (hardcoded to amp)

## Available waveforms

- **Sine**
- **Saw**
- **Square**
- **Triangle**
- **Noise**

You can rotate between them while playing.

## How to play

- Use the keyboard (A–L row + W/E/T/Y/U/O/P) like a small piano
- Hold multiple keys to play chords
- Press **B** to change the waveform
- Press **Q** or **Ctrl+C** to quit

## Screenshot

![capture](images/capture1.png)

---

## How it works (very simply)

- **Generator** → produces sound (sine, saw, etc.)
- **Node** → changes sound (filters, effects, modulation)
- **PatchSource** → generator + chain of nodes
- The synth just plays the current patch for each key you press

---

## Next steps (planned)

### Short term

- Audio capture and oscilator/audio wave visualization
- Use **Lua** for config / scripting allowing user to have his own presets
- Make patches and fx configurable and dynamically loadable (third-party friendly)
- Add essential fx (adsr, phase distortion, cutoff, etc.)

### Synth improvements

- One sink per voice with a **dynamic mixer** (instead of many sinks) ??
- Proper **envelope controls**  
  (attack, decay, sustain, release)
- Unison, detune, LFO, glide/portamento
- Mono / poly modes
- Effects (reverb, delay, distortion, flanger, phaser)

### UI

- Ratatui UI with:
    - keyboard drawn at the bottom
    - knobs/sliders for synth parameters
    - support for real keyboard + MIDI input
    - visualizers (waveform, volume, spectrum)

---

## Long-term idea

A **CLI synth you can jam with friends**.

- Logic Pro compatibility
- Encrypted P2P sessions
- Everyone hears what everyone plays (like a band)
- Each player can bind sounds to keys
- Record the whole session to WAV
- Choose a key and only allow to play notes that belong to it

### Architecture idea

- Audio thread (real-time, high priority)
- Network thread (async, P2P)
- Lock-free ring buffers between them
- Low-latency streaming (<50ms target)

Session owner mixes audio and distributes it back to peers.
