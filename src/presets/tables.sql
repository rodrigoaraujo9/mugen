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
(1, 'Bass'),
(2, 'Plucks'),
(3, 'Leads'),
(4, 'Bells'),
(5, 'Keys'),
(6, 'Pads'),
(7, 'Drums'),
(8, 'Soundscapes'),
(9, 'Effects');

insert into waves (id, name) values
(0, 'Sine'),
(1, 'Saw'),
(2, 'Square'),
(3, 'Triangle'),
(4, 'Noise');
