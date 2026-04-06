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
(4, 'Kick', 6, -2, 0, 0.0, 0.03, 0.0, 0.0, 0, 10.0, 0.0, 12750.0);
