use std::error::Error;

use rusqlite::Connection;

use crate::patch::oscilators::basic::Wave;

#[derive(Debug)]
pub struct Preset {
    pub id: u32,
    pub name: String,
    pub category_id: u32,
    pub octave_shift: i32,
    pub wave: Wave,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub lfo_wave: Wave,
    pub lfo_rate: f32,
    pub lfo_depth: f32,
    pub cutoff: f32,
}

pub async fn import_db() -> Result<Vec<Preset>, Box<dyn Error + Send + Sync>> {
    let conn = Connection::open("db.sqlite")?;

    let mut stmt = conn.prepare(
        "SELECT id, name, category_id, octave_shift, wave_id, attack, decay, sustain, release,
                lfo_wave_id, lfo_rate, lfo_depth
         FROM presets",
    )?;

    let presets = stmt
        .query_map([], |row| {
            Ok(Preset {
                id: row.get(0)?,
                name: row.get(1)?,
                category_id: row.get(2)?,
                octave_shift: row.get(3)?,
                wave: row.get(4)?,
                attack: row.get(5)?,
                decay: row.get(6)?,
                sustain: row.get(7)?,
                release: row.get(8)?,
                lfo_wave: row.get(9)?,
                lfo_rate: row.get(10)?,
                lfo_depth: row.get(11)?,
                cutoff: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<Preset>, _>>()?;

    Ok(presets)
}
