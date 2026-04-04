create table presets (
    id smallint primary key not null unique,
    name text not null unique,
    octave_shift smallint check(octave_shift >= -4 and octave_shift <= 4), -- (-4..4)
    wave smallint default 0 check(wave >= 0 and wave <= 4), -- 0-Sine | 1-Saw | 2-Square | 3-Triangle | 4-Noise
    attack real default 0.0 check(attack >= 0.0), -- seconds
    decay real default 0.0 check(decay >= 0.0), -- seconds
    sustain real default 1.0 check(sustain >= 0.0 and sustain <= 1.0), -- (0..1)
    release real default 0.0 check(release >= 0.0), -- seconds
    lfo_wave smallint default 0 check(wave >= 0 and wave <= 4), -- 0-Sine | 1-Saw | 2-Square | 3-Triangle | 4-Noise
    lfo_rate real default 10.0 check (lfo_rate >= 0.0), -- Hz
    lfo_depth real default 0.0 check(lfo_depth >= 0.0 and lfo_depth <= 1.0), -- (0..1)
    cuttoff real default 20000.0 check(cuttoff >= 0.0 and cuttoff <= 20000.0) -- Hz
);
