insert into presets
    (id, name, category_id,
        octave_shift, wave_id,
        attack, decay, sustain,
        release, lfo_wave_id,
        lfo_rate, lfo_depth,
    cutoff) values
(0, 'Heavy Bass', 0, -3, 1, 0.05, 0.03, 0.42, 0.0, 2, 10.0, 1.0, 20000.0),
(1, 'Broken Bells', 3, +3, 3, 0.0, 0.2, 0.0, 0.54, 4, 10.0, 0.6, 20000.0),
(2, 'Church Organ', 4, -1, 0, 0.5, 0.5, 0.4, 1.0, 0, 2.75, 0.1, 20000.0),
(3, 'Brown Noise', 7, -1, 4, 1.0, 0.0, 1.0, 1.0, 0, 10.0, 0.0, 526.0),
(4, 'Kick', 6, -2, 0, 0.0, 0.03, 0.0, 0.0, 0, 10.0, 0.0, 12750.0);
