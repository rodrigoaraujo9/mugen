PRAGMA foreign_keys = ON;

drop table if exists presets;
drop table if exists categories;
drop table if exists waves;

create table categories (
    id integer primary key,
    name text not null unique collate nocase
) strict;

create table waves (
    id integer primary key,
    name text not null unique collate nocase
) strict;

create table presets (
    id integer primary key,
    name text not null unique collate nocase,

    category_id integer not null
        references categories(id)
        on update cascade
        on delete restrict,

    octave_shift integer not null default 0
        check (octave_shift between -4 and 4),

    wave_id integer not null default 0
        references waves(id)
        on update cascade
        on delete restrict,

    attack real not null default 0.0
        check (attack >= 0.0),

    decay real not null default 0.0
        check (decay >= 0.0),

    sustain real not null default 1.0
        check (sustain between 0.0 and 1.0),

    release real not null default 0.0
        check (release >= 0.0),

    lfo_wave_id integer not null default 0
        references waves(id)
        on update cascade
        on delete restrict,

    lfo_rate real not null default 10.0
        check (lfo_rate >= 0.0),

    lfo_depth real not null default 0.0
        check (lfo_depth between 0.0 and 1.0),

    cutoff real not null default 20000.0
        check (cutoff between 0.0 and 20000.0)
) strict;

create index idx_presets_category_id on presets(category_id);
create index idx_presets_wave_id on presets(wave_id);
create index idx_presets_lfo_wave_id on presets(lfo_wave_id);

insert into categories (id, name) values
(0, 'Bass'),
(1, 'Plucks'),
(2, 'Leads'),
(3, 'Bells'),
(4, 'Keys'),
(5, 'Pads'),
(6, 'Drums'),
(7, 'Soundscapes'),
(8, 'Effects');

insert into waves (id, name) values
(0, 'Sine'),
(1, 'Saw'),
(2, 'Square'),
(3, 'Triangle'),
(4, 'Noise');

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
(4, 'Kick', 6, -2, 0, 0.0, 0.03, 0.0, 0.0, 0, 10.0, 0.0, 12750.0),
(5,  'Sub Floor',            0, -4, 0, 0.02, 0.08, 0.85, 0.18, 0,  1.20, 0.04, 1800.0),
(6,  'Rubber Bass',          0, -3, 2, 0.01, 0.10, 0.55, 0.12, 0,  4.50, 0.10, 3200.0),
(7,  'Acid Bite',            0, -2, 1, 0.00, 0.07, 0.48, 0.08, 2,  7.20, 0.22, 2400.0),
(8,  'Round Mono',           0, -2, 3, 0.01, 0.12, 0.70, 0.14, 0,  2.40, 0.05, 4500.0),
(9,  'Dirty Square Bass',    0, -3, 2, 0.00, 0.05, 0.40, 0.06, 4,  9.00, 0.18, 2100.0),
(10, 'Velvet Pluck',         1,  0, 3, 0.00, 0.14, 0.00, 0.12, 0,  5.50, 0.08, 12000.0),
(11, 'Glass Pluck',          1,  1, 0, 0.00, 0.10, 0.00, 0.20, 3,  6.00, 0.12, 16000.0),
(12, 'Wooden Thumb',         1, -1, 2, 0.00, 0.09, 0.00, 0.06, 0,  3.20, 0.04, 6800.0),
(13, 'Muted Pop',            1,  0, 1, 0.00, 0.05, 0.00, 0.07, 2, 10.00, 0.10, 5200.0),
(14, 'Crystal Plink',        1,  2, 3, 0.00, 0.18, 0.00, 0.22, 0,  8.40, 0.16, 18000.0),
(15, 'Soft Lead',            2,  0, 0, 0.03, 0.15, 0.70, 0.28, 0,  4.00, 0.05, 15000.0),
(16, 'Saw Hero',             2,  1, 1, 0.01, 0.10, 0.78, 0.24, 3,  5.00, 0.08, 14000.0),
(17, 'Square Solo',          2,  1, 2, 0.00, 0.07, 0.68, 0.18, 0,  5.50, 0.06, 9000.0),
(18, 'Thin Whistle',         2,  2, 3, 0.00, 0.05, 0.60, 0.15, 0,  6.80, 0.10, 17000.0),
(19, 'Noise Lead',           2,  1, 4, 0.00, 0.03, 0.50, 0.10, 0, 11.00, 0.30, 4200.0),
(20, 'Toy Bell',             3,  2, 3, 0.00, 0.25, 0.00, 0.35, 0,  7.50, 0.14, 17000.0),
(21, 'FM-ish Bell',          3,  1, 0, 0.00, 0.35, 0.00, 0.50, 3, 12.00, 0.20, 20000.0),
(22, 'Tiny Chime',           3,  3, 3, 0.00, 0.18, 0.00, 0.22, 0,  9.80, 0.10, 19000.0),
(23, 'Frozen Bell',          3,  2, 2, 0.01, 0.40, 0.05, 0.80, 0,  3.00, 0.08, 15000.0),
(24, 'Clocktone',            3,  2, 1, 0.00, 0.22, 0.00, 0.18, 4, 13.00, 0.12, 17500.0),
(25, 'Cathedral',            4, -1, 0, 0.40, 0.60, 0.55, 1.20, 0,  2.20, 0.05, 20000.0),
(26, 'Tape Organ',           4,  0, 1, 0.10, 0.40, 0.50, 0.80, 3,  1.80, 0.07, 9000.0),
(27, 'Reed Organ',           4, -1, 2, 0.05, 0.30, 0.45, 0.60, 0,  2.50, 0.05, 7000.0),
(28, 'Air Keys',             4,  1, 3, 0.03, 0.25, 0.50, 0.40, 0,  4.50, 0.04, 12000.0),
(29, 'Dusty Piano',          4,  0, 0, 0.00, 0.40, 0.10, 0.35, 0,  5.00, 0.06, 8000.0),
(30, 'Warm Pad',             5,  0, 0, 0.80, 0.70, 0.78, 1.60, 0,  0.80, 0.04, 20000.0),
(31, 'Analog Cloud',         5,  0, 1, 0.60, 0.80, 0.72, 1.40, 3,  1.20, 0.08, 14000.0),
(32, 'Night Pad',            5, -1, 2, 0.70, 0.50, 0.68, 1.80, 0,  0.60, 0.05, 7500.0),
(33, 'Ice Choir',            5,  1, 3, 0.90, 0.90, 0.82, 2.10, 0,  0.45, 0.10, 17000.0),
(34, 'Noise Wash',           5,  0, 4, 1.20, 0.40, 0.90, 2.50, 0,  0.30, 0.06, 1800.0),
(35, 'Deep Kick',            6, -3, 0, 0.00, 0.05, 0.00, 0.00, 0, 10.00, 0.00, 9000.0),
(36, 'Soft Kick',            6, -2, 0, 0.00, 0.08, 0.00, 0.00, 0,  8.00, 0.00, 12000.0),
(37, 'Snare Noise',          6,  0, 4, 0.00, 0.12, 0.00, 0.02, 0, 10.00, 0.00, 4800.0),
(38, 'Hi Hat',               6,  3, 4, 0.00, 0.03, 0.00, 0.00, 0, 10.00, 0.00, 16000.0),
(39, 'Tom Drum',             6, -1, 0, 0.00, 0.10, 0.00, 0.04, 0,  6.50, 0.00, 10500.0),
(40, 'Wind Bed',             7,  0, 4, 1.50, 0.30, 0.88, 2.80, 0,  0.20, 0.03, 900.0),
(41, 'Sea Foam',             7,  1, 4, 1.20, 0.40, 0.84, 2.20, 3,  0.35, 0.08, 1400.0),
(42, 'Radio Dust',           7,  0, 4, 0.30, 0.20, 0.75, 1.50, 2,  4.00, 0.12, 2200.0),
(43, 'Frozen Air',           7,  2, 3, 0.80, 0.60, 0.90, 2.00, 0,  0.50, 0.16, 13000.0),
(44, 'Dark Space',           7, -2, 1, 1.00, 0.90, 0.92, 2.70, 0,  0.18, 0.04, 2600.0),
(45, 'Laser Zap',            8,  2, 1, 0.00, 0.06, 0.10, 0.05, 3, 14.00, 0.30, 18000.0),
(46, 'UI Click',             8,  3, 2, 0.00, 0.01, 0.00, 0.00, 0, 10.00, 0.00, 12000.0),
(47, 'Sweep Rise',           8,  1, 4, 0.20, 0.80, 0.85, 0.60, 0,  0.90, 0.22, 6000.0),
(48, 'Sweep Fall',           8, -1, 4, 0.00, 0.60, 0.60, 0.20, 0,  1.40, 0.18, 3000.0),
(49, 'Glitch Tone',          8,  2, 2, 0.00, 0.04, 0.20, 0.03, 4, 16.00, 0.35, 9000.0),
(50, 'Soft Sub',             0, -4, 0, 0.03, 0.10, 0.92, 0.20, 0,  0.80, 0.02, 1200.0),
(51, 'Pick Bass',            0, -2, 1, 0.00, 0.06, 0.38, 0.09, 0,  5.00, 0.07, 3600.0),
(52, 'Hollow Pluck',         1,  0, 2, 0.00, 0.11, 0.00, 0.10, 3,  7.20, 0.11, 8400.0),
(53, 'Bright Lead',          2,  2, 1, 0.00, 0.09, 0.74, 0.20, 0,  6.40, 0.07, 18000.0),
(54, 'Mallet Bell',          3,  1, 3, 0.00, 0.28, 0.00, 0.32, 0,  8.10, 0.09, 16500.0),
(55, 'Stage EP',             4,  0, 0, 0.00, 0.30, 0.18, 0.25, 0,  4.20, 0.03, 10000.0),
(56, 'Sunset Pad',           5,  1, 1, 0.90, 0.70, 0.76, 1.90, 0,  0.55, 0.06, 11000.0),
(57, 'Clap Noise',           6,  2, 4, 0.00, 0.05, 0.00, 0.01, 0, 10.00, 0.00, 7500.0),
(58, 'Rain Texture',         7,  0, 4, 0.80, 0.20, 0.86, 1.70, 0,  0.28, 0.05, 2400.0),
(59, 'Arcade Beep',          8,  3, 2, 0.00, 0.02, 0.35, 0.03, 0, 10.00, 0.00, 14000.0);
