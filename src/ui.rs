use std::{
    collections::HashSet,
    io,
    io::stdout,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self, DisableFocusChange, EnableFocusChange, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use device_query::Keycode;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use tokio::sync::{mpsc, watch};
use tokio::time::sleep;

use crate::audio::{Client, Snapshot};
use crate::generators::basic::Wave;
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmp;
use crate::nodes::lowpass::LowPass;

#[allow(dead_code)]
mod kdr {
    use ratatui::style::Color;

    pub const BG0: Color = Color::Rgb(24, 22, 22);
    pub const BORDER: Color = Color::Rgb(98, 94, 90);

    pub const FG: Color = Color::Rgb(197, 201, 197);
    pub const MUTED: Color = Color::Rgb(158, 155, 147);

    pub const ORANGE: Color = Color::Rgb(182, 146, 123);
    pub const YELLOW: Color = Color::Rgb(196, 178, 138);
}

struct TuiGuard;

impl Drop for TuiGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, DisableFocusChange, LeaveAlternateScreen);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pane {
    Waveforms,
    Adsr,
    Mod,
    Keyboard,
}

impl Pane {
    fn next(self) -> Self {
        match self {
            Self::Waveforms => Self::Adsr,
            Self::Adsr => Self::Mod,
            Self::Mod => Self::Keyboard,
            Self::Keyboard => Self::Waveforms,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModTab {
    Lfo,
    LowPass,
}

impl ModTab {
    fn next(self) -> Self {
        match self {
            Self::Lfo => Self::LowPass,
            Self::LowPass => Self::Lfo,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdsrParam {
    Attack,
    Decay,
    Sustain,
    Release,
}

impl AdsrParam {
    const ALL: [Self; 4] = [Self::Attack, Self::Decay, Self::Sustain, Self::Release];

    fn label_and_hint(self) -> (&'static str, &'static str) {
        match self {
            Self::Attack => ("Attack", "(s)"),
            Self::Decay => ("Decay", "(s)"),
            Self::Sustain => ("Sustain", "(0..1)"),
            Self::Release => ("Release", "(s)"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LfoParam {
    Kind,
    RateHz,
    Depth,
}

impl LfoParam {
    const ALL: [Self; 3] = [Self::Kind, Self::RateHz, Self::Depth];

    fn label_and_hint(self) -> (&'static str, &'static str) {
        match self {
            Self::Kind => ("Wave", ""),
            Self::RateHz => ("Rate", "(Hz)"),
            Self::Depth => ("Depth", "(0..1)"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LowPassParam {
    CutoffHz,
}

impl LowPassParam {
    const ALL: [Self; 1] = [Self::CutoffHz];

    fn label_and_hint(self) -> (&'static str, &'static str) {
        match self {
            Self::CutoffHz => ("Cutoff", "(Hz)"),
        }
    }
}

struct UiState {
    pane: Pane,

    waves: [Wave; 5],
    wave_idx: usize,

    adsr_param_idx: usize,
    adsr: Adsr,

    mod_tab: ModTab,

    lfo_param_idx: usize,
    lfo: LfoAmp,

    lowpass_param_idx: usize,
    lowpass: LowPass,

    patch_name: String,
    wave: Wave,
    muted: bool,
    volume: f32,
    held_keys: HashSet<Keycode>,
    octave: i32,
}

impl UiState {
    fn new(snapshot: Snapshot) -> Self {
        let waves = [
            Wave::Sine,
            Wave::Saw,
            Wave::Square,
            Wave::Triangle,
            Wave::Noise,
        ];

        let wave_idx = waves.iter().position(|w| *w == snapshot.wave).unwrap_or(0);

        Self {
            pane: Pane::Waveforms,

            waves,
            wave_idx,

            adsr_param_idx: 0,
            adsr: snapshot.adsr,

            mod_tab: ModTab::Lfo,
            lfo_param_idx: 0,
            lfo: snapshot.lfo_amp,

            lowpass_param_idx: 0,
            lowpass: snapshot.lowpass,

            patch_name: snapshot.patch_name,
            wave: snapshot.wave,
            muted: snapshot.muted,
            volume: snapshot.volume,
            held_keys: HashSet::new(),
            octave: 0,
        }
    }

    #[inline]
    fn selected_wave(&self) -> Wave {
        self.waves[self.wave_idx]
    }

    #[inline]
    fn selected_adsr_param(&self) -> AdsrParam {
        AdsrParam::ALL[self.adsr_param_idx]
    }

    #[inline]
    fn selected_lfo_param(&self) -> LfoParam {
        LfoParam::ALL[self.lfo_param_idx]
    }

    #[inline]
    fn selected_lowpass_param(&self) -> LowPassParam {
        LowPassParam::ALL[self.lowpass_param_idx]
    }

    fn sync_from_snapshot(&mut self, snapshot: Snapshot) {
        self.patch_name = snapshot.patch_name;
        self.wave = snapshot.wave;
        self.muted = snapshot.muted;
        self.volume = snapshot.volume;
        self.adsr = snapshot.adsr;
        self.lfo = snapshot.lfo_amp;
        self.lowpass = snapshot.lowpass;
        self.octave = snapshot.octave;
        self.sync_wave_idx();
    }

    fn sync_wave_idx(&mut self) {
        if let Some(i) = self.waves.iter().position(|wave| *wave == self.wave) {
            self.wave_idx = i;
        }
    }
}

pub async fn run_ui(
    client: Client,
    shutdown_tx: watch::Sender<bool>,
    focused: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = stdout();

    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableFocusChange)?;
    let _guard = TuiGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let initial = client.subscribe().borrow().clone();
    let mut ui = UiState::new(initial);

    let (key_tx, mut key_rx) = mpsc::unbounded_channel::<KeyEvent>();

    let stop = Arc::new(AtomicBool::new(false));
    spawn_input_thread(key_tx, Arc::clone(&stop), Arc::clone(&focused));

    let mut snapshot_rx = client.subscribe();
    let mut held_keys_rx = client.subscribe_held_keys();

    let intro_start = Instant::now();
    let mut show_intro = true;

    loop {
        if show_intro && intro_start.elapsed() >= Duration::from_secs(1) {
            show_intro = false;
        }

        draw_frame(&mut terminal, &ui, show_intro)?;

        tokio::select! {
            _ = snapshot_rx.changed() => {
                let snapshot = snapshot_rx.borrow().clone();
                ui.sync_from_snapshot(snapshot);
            }

            _ = held_keys_rx.changed() => {
                ui.held_keys = held_keys_rx.borrow().clone();
            }

            key = key_rx.recv() => {
                let Some(key) = key else { break; };

                if should_quit(key) {
                    let _ = shutdown_tx.send(true);
                    break;
                }

                if show_intro {
                    continue;
                }

                if matches!(key.code, KeyCode::Tab) {
                    ui.pane = ui.pane.next();
                    continue;
                }

                match ui.pane {
                    Pane::Waveforms => handle_waveforms(&mut ui, &client, key),
                    Pane::Adsr => handle_adsr(&mut ui, &client, key),
                    Pane::Mod => handle_mod(&mut ui, &client, key),
                    Pane::Keyboard => handle_keyboard(&mut ui, &client, key),
                }
            }

            _ = sleep(Duration::from_millis(16)) => {}
        }
    }

    stop.store(true, Ordering::Relaxed);
    terminal.show_cursor()?;
    Ok(())
}

fn spawn_input_thread(
    key_tx: mpsc::UnboundedSender<KeyEvent>,
    stop: Arc<AtomicBool>,
    focused: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            if event::poll(Duration::from_millis(50)).ok() != Some(true) {
                continue;
            }

            match event::read() {
                Ok(Event::Key(key))
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    let _ = key_tx.send(key);
                }
                Ok(Event::FocusLost) => {
                    focused.store(false, Ordering::Relaxed);
                }
                Ok(Event::FocusGained) => {
                    focused.store(true, Ordering::Relaxed);
                }
                _ => {}
            }
        }
    });
}

fn draw_frame(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ui: &UiState,
    show_intro: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if show_intro {
        terminal.draw(draw_intro)?;
    } else {
        terminal.draw(|f| draw_ui(f, ui))?;
    }

    Ok(())
}

fn should_quit(key: KeyEvent) -> bool {
    (key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')))
        || matches!(key.code, KeyCode::Char('q'))
}

fn handle_waveforms(ui: &mut UiState, client: &Client, key: KeyEvent) {
    let prev = ui.wave_idx;

    match key.code {
        KeyCode::Up if ui.wave_idx > 0 => ui.wave_idx -= 1,
        KeyCode::Down if ui.wave_idx + 1 < ui.waves.len() => ui.wave_idx += 1,
        _ => {}
    }

    if ui.wave_idx != prev {
        client.set_wave(ui.selected_wave());
    }
}

fn handle_adsr(ui: &mut UiState, client: &Client, key: KeyEvent) {
    match key.code {
        KeyCode::Up if ui.adsr_param_idx > 0 => ui.adsr_param_idx -= 1,
        KeyCode::Down if ui.adsr_param_idx + 1 < AdsrParam::ALL.len() => ui.adsr_param_idx += 1,
        KeyCode::Left => {
            tweak_adsr(ui, -1);
            client.set_adsr(ui.adsr);
        }
        KeyCode::Right => {
            tweak_adsr(ui, 1);
            client.set_adsr(ui.adsr);
        }
        _ => {}
    }
}

fn handle_mod(ui: &mut UiState, client: &Client, key: KeyEvent) {
    match key.code {
        KeyCode::Char(' ') => ui.mod_tab = ui.mod_tab.next(),

        KeyCode::Up => match ui.mod_tab {
            ModTab::Lfo if ui.lfo_param_idx > 0 => ui.lfo_param_idx -= 1,
            ModTab::LowPass if ui.lowpass_param_idx > 0 => ui.lowpass_param_idx -= 1,
            _ => {}
        },

        KeyCode::Down => match ui.mod_tab {
            ModTab::Lfo if ui.lfo_param_idx + 1 < LfoParam::ALL.len() => ui.lfo_param_idx += 1,
            ModTab::LowPass if ui.lowpass_param_idx + 1 < LowPassParam::ALL.len() => {
                ui.lowpass_param_idx += 1
            }
            _ => {}
        },

        KeyCode::Left => match ui.mod_tab {
            ModTab::Lfo => {
                tweak_lfo(ui, -1);
                client.set_lfo_amp(ui.lfo);
            }
            ModTab::LowPass => {
                tweak_lowpass(ui, -1);
                client.set_lowpass(ui.lowpass);
            }
        },

        KeyCode::Right => match ui.mod_tab {
            ModTab::Lfo => {
                tweak_lfo(ui, 1);
                client.set_lfo_amp(ui.lfo);
            }
            ModTab::LowPass => {
                tweak_lowpass(ui, 1);
                client.set_lowpass(ui.lowpass);
            }
        },

        _ => {}
    }
}

fn handle_keyboard(ui: &mut UiState, client: &Client, key: KeyEvent) {
    match key.code {
        KeyCode::Right if ui.octave < 4 => {
            ui.octave += 1;
            client.set_octave(ui.octave);
        }
        KeyCode::Left if ui.octave > -4 => {
            ui.octave -= 1;
            client.set_octave(ui.octave);
        }
        _ => {}
    }
}

fn tweak_adsr(ui: &mut UiState, dir: i32) {
    let step = 0.01;
    let delta = if dir < 0 { -step } else { step };

    match ui.selected_adsr_param() {
        AdsrParam::Attack => ui.adsr.attack_s = (ui.adsr.attack_s + delta).clamp(0.0, 10.0),
        AdsrParam::Decay => ui.adsr.decay_s = (ui.adsr.decay_s + delta).clamp(0.0, 10.0),
        AdsrParam::Sustain => ui.adsr.sustain = (ui.adsr.sustain + delta).clamp(0.0, 1.0),
        AdsrParam::Release => ui.adsr.release_s = (ui.adsr.release_s + delta).clamp(0.0, 10.0),
    }
}

fn tweak_lfo(ui: &mut UiState, dir: i32) {
    let dir = if dir < 0 { -1 } else { 1 };

    match ui.selected_lfo_param() {
        LfoParam::Kind => ui.lfo.wave = next_wave(ui.lfo.wave, dir),
        LfoParam::RateHz => {
            ui.lfo.rate_hz = (ui.lfo.rate_hz + dir as f32 * 0.25).clamp(0.05, 40.0);
        }
        LfoParam::Depth => {
            ui.lfo.depth = (ui.lfo.depth + dir as f32 * 0.02).clamp(0.0, 1.0);
        }
    }
}

fn tweak_lowpass(ui: &mut UiState, dir: i32) {
    let dir = if dir < 0 { -1.0 } else { 1.0 };

    match ui.selected_lowpass_param() {
        LowPassParam::CutoffHz => {
            let cutoff = ui.lowpass.cutoff_hz;

            let step = if cutoff < 100.0 {
                5.0
            } else if cutoff < 1000.0 {
                25.0
            } else if cutoff < 5000.0 {
                100.0
            } else {
                250.0
            };

            ui.lowpass.cutoff_hz = (cutoff + dir * step).clamp(20.0, 20_000.0);
        }
    }
}

fn next_wave(wave: Wave, dir: i32) -> Wave {
    const ALL: [Wave; 5] = [
        Wave::Sine,
        Wave::Saw,
        Wave::Square,
        Wave::Triangle,
        Wave::Noise,
    ];

    let idx = ALL.iter().position(|w| *w == wave).unwrap_or(0) as i32;
    ALL[(idx + dir).rem_euclid(ALL.len() as i32) as usize]
}

fn draw_intro(f: &mut ratatui::Frame) {
    let area = f.area();

    const MIN_W: u16 = 136;
    const MIN_H: u16 = 25;

    if area.width < MIN_W || area.height < MIN_H {
        draw_too_small(f, area, MIN_W, MIN_H);
        return;
    }

    let art: [&str; 23] = [
        r"          _____                    _____                    _____                    _____                    _____          ",
        r"         /\    \                  /\    \                  /\    \                  /\    \                  /\    \         ",
        r"        /::\____\                /::\____\                /::\    \                /::\    \                /::\____\        ",
        r"       /::::|   |               /:::/    /               /::::\    \              /::::\    \              /::::|   |        ",
        r"      /:::::|   |              /:::/    /               /::::::\    \            /::::::\    \            /:::::|   |        ",
        r"     /::::::|   |             /:::/    /               /:::/\:::\    \          /:::/\:::\    \          /::::::|   |        ",
        r"    /:::/|::|   |            /:::/    /               /:::/  \:::\    \        /:::/__\:::\    \        /:::/|::|   |        ",
        r"   /:::/ |::|   |           /:::/    /               /:::/    \:::\    \      /::::\   \:::\    \      /:::/ |::|   |        ",
        r"  /:::/  |::|___|______    /:::/    /      _____    /:::/    / \:::\    \    /::::::\   \:::\    \    /:::/  |::|   | _____  ",
        r" /:::/   |::::::::\    \  /:::/____/      /\    \  /:::/    /   \:::\ ___\  /:::/\:::\   \:::\    \  /:::/   |::|   |/\    \ ",
        r"/:::/    |:::::::::\____\|:::|    /      /::\____\/:::/____/  ___\:::|    |/:::/__\:::\   \:::\____\/:: /    |::|   /::\____\",
        r"\::/    / ~~~~~/:::/    /|:::|____\     /:::/    /\:::\    \ /\  /:::|____|\:::\   \:::\   \::/    /\::/    /|::|  /:::/    /",
        r" \/____/      /:::/    /  \:::\    \   /:::/    /  \:::\    /::\ \::/    /  \:::\   \:::\   \/____/  \/____/ |::| /:::/    / ",
        r"             /:::/    /    \:::\    \ /:::/    /    \:::\   \:::\ \/____/    \:::\   \:::\    \              |::|/:::/    /  ",
        r"            /:::/    /      \:::\    /:::/    /      \:::\   \:::\____\       \:::\   \:::\____\             |::::::/    /   ",
        r"           /:::/    /        \:::\__/:::/    /        \:::\  /:::/    /        \:::\   \::/    /             |:::::/    /    ",
        r"          /:::/    /          \::::::::/    /          \:::\/:::/    /          \:::\   \/____/              |::::/    /     ",
        r"         /:::/    /            \::::::/    /            \::::::/    /            \:::\    \                  /:::/    /      ",
        r"        /:::/    /              \::::/    /              \::::/    /              \:::\____\                /:::/    /       ",
        r"        \::/    /                \::/____/                \::/____/                \::/    /                \::/    /        ",
        r"         \/____/                  ~~                                                \/____/                  \/____/         ",
        r"",
        r"                                                      s y n t h e s i s",
    ];

    let max_w = art.iter().map(|s| s.chars().count()).max().unwrap_or(0);

    let lines: Vec<Line> = art
        .iter()
        .map(|s| {
            let mut owned = s.to_string();
            let pad = max_w.saturating_sub(owned.chars().count());
            if pad > 0 {
                owned.extend(std::iter::repeat_n(' ', pad));
            }
            Line::from(Span::raw(owned).bold())
        })
        .collect();

    f.render_widget(Clear, area);

    let outer = outer_block();
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let total_h = art.len() as u16;
    let y = inner.y + (inner.height.saturating_sub(total_h)) / 2;

    let main_area = Rect {
        x: inner.x,
        y,
        width: inner.width,
        height: (total_h - 1).min(inner.height),
    };

    f.render_widget(
        Paragraph::new(
            lines
                .into_iter()
                .take((total_h - 1) as usize)
                .collect::<Vec<_>>(),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(kdr::FG).bg(kdr::BG0)),
        main_area,
    );

    let synth_y = y + total_h - 1;
    if synth_y < inner.y + inner.height {
        let synth_area = Rect {
            x: inner.x,
            y: synth_y,
            width: inner.width,
            height: 1,
        };

        f.render_widget(
            Paragraph::new(vec![Line::from(Span::styled(
                "s y n t h e s i s",
                Style::default().fg(kdr::ORANGE).bold(),
            ))])
            .alignment(Alignment::Center)
            .style(Style::default().bg(kdr::BG0)),
            synth_area,
        );
    }
}

fn draw_ui(f: &mut ratatui::Frame, ui: &UiState) {
    let area = f.area();

    const MIN_W: u16 = 136;
    const MIN_H: u16 = 33;

    if area.width < MIN_W || area.height < MIN_H {
        draw_too_small(f, area, MIN_W, MIN_H);
        return;
    }

    f.render_widget(Clear, area);

    let outer = outer_block();
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(inner);

    let logo_area = main[0];
    let content_area = main[1];
    let help_area = main[2];

    draw_logo(f, logo_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(content_area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(top[1]);

    draw_waveforms(f, top[0], ui);
    draw_adsr(f, right[0], ui);
    draw_mod(f, right[1], ui);
    draw_keyboard(f, rows[1], ui);
    draw_help(f, help_area, ui);
}

fn draw_too_small(f: &mut ratatui::Frame, area: Rect, min_w: u16, min_h: u16) {
    f.render_widget(Clear, area);

    let outer = outer_block();
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let msg = vec![
        Line::from(Span::styled(
            "terminal too small",
            Style::default().fg(kdr::ORANGE).bold(),
        )),
        Line::from(Span::styled(
            format!(
                "need {}×{}  —  currently {}×{}",
                min_w, min_h, area.width, area.height
            ),
            Style::default().fg(kdr::MUTED),
        )),
    ];

    let h = msg.len() as u16;
    let y = inner.y + inner.height.saturating_sub(h) / 2;

    f.render_widget(
        Paragraph::new(msg)
            .alignment(Alignment::Center)
            .style(Style::default().bg(kdr::BG0)),
        Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: h.min(inner.height),
        },
    );
}

fn draw_logo(f: &mut ratatui::Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("無", Style::default().fg(kdr::FG)).bold()),
        Line::from(Span::styled("限", Style::default().fg(kdr::FG)).bold()),
    ];

    let total_h = lines.len() as u16;
    let y = area.y + area.height.saturating_sub(total_h) / 2;

    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(kdr::BG0)),
        Rect {
            x: area.x,
            y,
            width: area.width,
            height: total_h.min(area.height),
        },
    );
}

fn draw_waveforms(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let focused = ui.pane == Pane::Waveforms;
    let block = panel_block("waveforms", focused);

    let mut lines = vec![Line::from("")];
    for (i, wave) in ui.waves.iter().copied().enumerate() {
        lines.push(simple_select_line(i == ui.wave_idx, wave.name()));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left)
            .style(panel_style(focused)),
        area,
    );
}

fn draw_adsr(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let focused = ui.pane == Pane::Adsr;
    let block = panel_block("adsr", focused);

    let rows = AdsrParam::ALL.iter().enumerate().map(|(i, param)| {
        let value = match param {
            AdsrParam::Attack => format!("{:.3}", ui.adsr.attack_s),
            AdsrParam::Decay => format!("{:.3}", ui.adsr.decay_s),
            AdsrParam::Sustain => format!("{:.2}", ui.adsr.sustain),
            AdsrParam::Release => format!("{:.3}", ui.adsr.release_s),
        };
        let (label, hint) = param.label_and_hint();
        kv_line(
            area.width.saturating_sub(2) as usize,
            i == ui.adsr_param_idx,
            label,
            hint,
            &value,
        )
    });

    let mut lines = vec![Line::from("")];
    lines.extend(rows);

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left)
            .style(panel_style(focused)),
        area,
    );
}

fn draw_mod(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let focused = ui.pane == Pane::Mod;

    let border = if focused {
        Style::default().fg(kdr::FG)
    } else {
        Style::default().fg(kdr::BORDER)
    };

    let active = if focused {
        Style::default().fg(kdr::ORANGE).bold()
    } else {
        Style::default().fg(kdr::FG).bold()
    };

    let inactive = if focused {
        Style::default().fg(kdr::FG)
    } else {
        Style::default().fg(kdr::MUTED)
    };

    let divider = if focused {
        Style::default().fg(kdr::FG).bold()
    } else {
        Style::default().fg(kdr::MUTED).bold()
    };

    let title = match ui.mod_tab {
        ModTab::Lfo => Line::from(vec![
            Span::raw(" "),
            Span::styled("lfo", active),
            Span::styled(" ─ ", divider),
            Span::styled("lowpass", inactive),
            Span::raw(" "),
        ]),
        ModTab::LowPass => Line::from(vec![
            Span::raw(" "),
            Span::styled("lfo", inactive),
            Span::styled(" ─ ", divider),
            Span::styled("lowpass", active),
            Span::raw(" "),
        ]),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border)
        .style(Style::default().bg(kdr::BG0));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let mut lines = vec![Line::from("")];

    match ui.mod_tab {
        ModTab::Lfo => {
            for (i, param) in LfoParam::ALL.iter().enumerate() {
                let value = match param {
                    LfoParam::Kind => ui.lfo.wave.name().to_string(),
                    LfoParam::RateHz => format!("{:.2}", ui.lfo.rate_hz),
                    LfoParam::Depth => format!("{:.2}", ui.lfo.depth),
                };
                let (label, hint) = param.label_and_hint();
                lines.push(kv_line(
                    inner.width as usize,
                    i == ui.lfo_param_idx,
                    label,
                    hint,
                    &value,
                ));
            }
        }
        ModTab::LowPass => {
            for (i, param) in LowPassParam::ALL.iter().enumerate() {
                let value = match param {
                    LowPassParam::CutoffHz => format!("{:.0}", ui.lowpass.cutoff_hz),
                };
                let (label, hint) = param.label_and_hint();
                lines.push(kv_line(
                    inner.width as usize,
                    i == ui.lowpass_param_idx,
                    label,
                    hint,
                    &value,
                ));
            }
        }
    }

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left)
            .style(panel_style(focused)),
        inner,
    );
}

fn draw_keyboard(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    struct WhiteKey {
        code: Keycode,
        label: &'static str,
    }

    struct BlackKey {
        code: Keycode,
        label: &'static str,
        gap_after: usize,
    }

    let white_keys = [
        WhiteKey {
            code: Keycode::A,
            label: "a",
        },
        WhiteKey {
            code: Keycode::S,
            label: "s",
        },
        WhiteKey {
            code: Keycode::D,
            label: "d",
        },
        WhiteKey {
            code: Keycode::F,
            label: "f",
        },
        WhiteKey {
            code: Keycode::G,
            label: "g",
        },
        WhiteKey {
            code: Keycode::H,
            label: "h",
        },
        WhiteKey {
            code: Keycode::J,
            label: "j",
        },
        WhiteKey {
            code: Keycode::K,
            label: "k",
        },
        WhiteKey {
            code: Keycode::L,
            label: "l",
        },
        WhiteKey {
            code: Keycode::Semicolon,
            label: ";",
        },
        WhiteKey {
            code: Keycode::Apostrophe,
            label: "'",
        },
    ];

    let black_keys = [
        BlackKey {
            code: Keycode::W,
            label: "w",
            gap_after: 0,
        },
        BlackKey {
            code: Keycode::E,
            label: "e",
            gap_after: 1,
        },
        BlackKey {
            code: Keycode::T,
            label: "t",
            gap_after: 3,
        },
        BlackKey {
            code: Keycode::Y,
            label: "y",
            gap_after: 4,
        },
        BlackKey {
            code: Keycode::U,
            label: "u",
            gap_after: 5,
        },
        BlackKey {
            code: Keycode::O,
            label: "o",
            gap_after: 7,
        },
        BlackKey {
            code: Keycode::P,
            label: "p",
            gap_after: 8,
        },
    ];

    let bounds = area;
    if bounds.width < 18 || bounds.height < 6 {
        return;
    }

    let focused = ui.pane == Pane::Keyboard;
    let is_pressed = |code: &Keycode| ui.held_keys.contains(code);

    let total_w = bounds.width as usize;
    let total_h = bounds.height as usize;
    let white_count = white_keys.len();

    let white_w = (total_w / white_count).max(4);
    let used_w = white_w * white_count;
    let x0 = bounds.x as usize + (total_w.saturating_sub(used_w)) / 2;

    let white_bg = if focused { kdr::FG } else { kdr::BORDER };
    let white_fill = Style::default().bg(white_bg).fg(kdr::BG0);
    let orange_fill = Style::default().bg(kdr::ORANGE).fg(kdr::BG0);
    let sep_style = Style::default().fg(kdr::BG0).bg(white_bg);
    let black_fill = Style::default().bg(kdr::BG0).fg(kdr::FG);
    let black_pressed = Style::default().bg(kdr::ORANGE).fg(kdr::BG0);

    let buf = f.buffer_mut();

    fill_rect(
        buf,
        bounds,
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
        Style::default().bg(kdr::BG0),
    );

    for (i, key) in white_keys.iter().enumerate() {
        let x = (x0 + i * white_w) as u16;
        fill_rect(
            buf,
            bounds,
            x,
            bounds.y,
            white_w as u16,
            bounds.height,
            if is_pressed(&key.code) {
                orange_fill
            } else {
                white_fill
            },
        );
    }

    for i in 0..(white_count - 1) {
        let x = (x0 + (i + 1) * white_w - 1) as u16;
        vline(buf, bounds, x, bounds.y, bounds.height, '│', sep_style);
    }

    for i in 0..(white_count - 1) {
        if is_pressed(&white_keys[i].code) || is_pressed(&white_keys[i + 1].code) {
            let x = (x0 + (i + 1) * white_w - 1) as u16;
            fill_rect(buf, bounds, x, bounds.y, 1, bounds.height, orange_fill);
        }
    }

    let label_y = bounds.y + bounds.height - 1;
    for (i, key) in white_keys.iter().enumerate() {
        let x = (x0 + i * white_w) as u16;
        let lx = x + white_w as u16 / 2;

        if lx >= bounds.x && lx < bounds.x + bounds.width {
            let pressed = is_pressed(&key.code);
            let style = if pressed {
                Style::default().fg(kdr::BG0).bg(kdr::ORANGE).bold()
            } else {
                Style::default().fg(kdr::BG0).bg(white_bg)
            };

            buf[(lx, label_y)]
                .set_char(key.label.chars().next().unwrap_or(' '))
                .set_style(style);
        }
    }

    let black_h = ((total_h * 60) / 100).max(2);
    let black_w = ((white_w * 55) / 100).max(2);

    for key in &black_keys {
        let center_x = x0 + (key.gap_after + 1) * white_w;
        let bx = center_x.saturating_sub(black_w / 2);

        let x = bx as u16;
        let y = bounds.y;
        let w = black_w as u16;
        let h = black_h as u16;
        let pressed = is_pressed(&key.code);

        fill_rect(
            buf,
            bounds,
            x,
            y,
            w,
            h,
            if pressed { black_pressed } else { black_fill },
        );

        let lx = x + w / 2;
        let ly = y + h - 1;
        if lx >= bounds.x
            && lx < bounds.x + bounds.width
            && ly >= bounds.y
            && ly < bounds.y + bounds.height
        {
            let style = if pressed {
                Style::default().fg(kdr::BG0).bg(kdr::ORANGE).bold()
            } else {
                Style::default().fg(kdr::FG).bg(kdr::BG0)
            };

            buf[(lx, ly)]
                .set_char(key.label.chars().next().unwrap_or(' '))
                .set_style(style);
        }
    }
}

fn draw_help(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let focus_name = match ui.pane {
        Pane::Waveforms => "Waveforms",
        Pane::Adsr => "ADSR",
        Pane::Mod => match ui.mod_tab {
            ModTab::Lfo => "LFO",
            ModTab::LowPass => "LowPass",
        },
        Pane::Keyboard => "Keyboard",
    };

    let key_style = Style::default().fg(kdr::ORANGE).bold();
    let dim = Style::default().fg(kdr::MUTED);
    let strong = Style::default().fg(kdr::FG).bold();

    let line1 = Line::from(vec![
        Span::styled("Tab", key_style),
        Span::styled(" focus  ", dim),
        Span::styled("Space", key_style),
        Span::styled(" toggle mod  ", dim),
        Span::styled("↑/↓", key_style),
        Span::styled(" select  ", dim),
        Span::styled("←/→", key_style),
        Span::styled(" change  ", dim),
        Span::styled("q", key_style),
        Span::styled(" quit  ", dim),
        Span::styled("Ctrl+C", key_style),
        Span::styled(" quit", dim),
    ]);

    let line2 = Line::from(vec![
        Span::styled("Focus: ", dim),
        Span::styled(focus_name, strong),
        Span::styled("  |  Wave: ", dim),
        Span::styled(ui.patch_name.clone(), strong),
        Span::styled("  |  LP ", dim),
        Span::styled(format!("{:.0}Hz", ui.lowpass.cutoff_hz), strong),
        Span::styled("  |  Vol ", dim),
        Span::styled(format!("{:.2}", ui.volume), strong),
        Span::styled(
            if ui.muted { " Muted" } else { "" },
            Style::default().fg(kdr::ORANGE).bold(),
        ),
        Span::styled("  |  Oct ", dim),
        Span::styled(
            format!("{:+}", ui.octave),
            if ui.octave == 0 {
                strong
            } else {
                Style::default().fg(kdr::YELLOW).bold()
            },
        ),
    ]);

    f.render_widget(
        Paragraph::new(vec![Line::from(""), line1, line2])
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(kdr::BG0)),
        area,
    );
}

fn outer_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(kdr::BORDER))
        .style(Style::default().bg(kdr::BG0).fg(kdr::FG))
}

fn panel_block(title: &'static str, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(tab_title(title, focused))
        .border_style(if focused {
            Style::default().fg(kdr::FG)
        } else {
            Style::default().fg(kdr::BORDER)
        })
        .style(Style::default().bg(kdr::BG0))
}

fn panel_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(kdr::FG).bg(kdr::BG0)
    } else {
        Style::default().fg(kdr::MUTED).bg(kdr::BG0)
    }
}

fn tab_title(name: &'static str, focused: bool) -> Span<'static> {
    let title = format!(" {name} ");
    if focused {
        Span::styled(title, Style::default().fg(kdr::ORANGE).bold())
    } else {
        Span::styled(title, Style::default().fg(kdr::MUTED))
    }
}

fn simple_select_line(selected: bool, text: &str) -> Line<'static> {
    if selected {
        Line::from(vec![
            Span::styled("› ", Style::default().fg(kdr::ORANGE).bold()),
            Span::styled(text.to_string(), Style::default().fg(kdr::FG).bold()),
        ])
    } else {
        Line::from(vec![
            Span::styled("  ", Style::default().fg(kdr::MUTED)),
            Span::styled(text.to_string(), Style::default().fg(kdr::FG)),
        ])
    }
}

fn kv_line(width: usize, selected: bool, label: &str, hint: &str, value: &str) -> Line<'static> {
    let prefix = if selected { "› " } else { "  " };
    let left_label = format!("{label} ");
    let left_len = prefix.chars().count() + left_label.chars().count() + hint.chars().count();
    let value_len = value.chars().count();
    let right_padding = 1usize;
    let min_gap = 2usize;

    let usable = width.saturating_sub(right_padding);
    let pad_len = usable.saturating_sub(left_len + min_gap + value_len);
    let pad = " ".repeat(pad_len + min_gap);

    let prefix_style = if selected {
        Style::default().fg(kdr::ORANGE).bold()
    } else {
        Style::default().fg(kdr::MUTED)
    };

    let label_style = if selected {
        Style::default().fg(kdr::FG).bold()
    } else {
        Style::default().fg(kdr::FG)
    };

    let value_style = if selected {
        Style::default().fg(kdr::FG).bold()
    } else {
        Style::default().fg(kdr::FG)
    };

    Line::from(vec![
        Span::styled(prefix, prefix_style),
        Span::styled(left_label, label_style),
        Span::styled(hint.to_string(), Style::default().fg(kdr::MUTED)),
        Span::raw(pad),
        Span::styled(value.to_string(), value_style),
        Span::raw(" ".repeat(right_padding)),
    ])
}

fn fill_rect(
    buf: &mut ratatui::buffer::Buffer,
    bounds: Rect,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    style: Style,
) {
    if w == 0 || h == 0 {
        return;
    }

    let x2 = x.saturating_add(w);
    let y2 = y.saturating_add(h);

    let xmin = x.max(bounds.x);
    let ymin = y.max(bounds.y);
    let xmax = x2.min(bounds.x + bounds.width);
    let ymax = y2.min(bounds.y + bounds.height);

    for yy in ymin..ymax {
        for xx in xmin..xmax {
            buf[(xx, yy)].set_char(' ').set_style(style);
        }
    }
}

fn vline(
    buf: &mut ratatui::buffer::Buffer,
    bounds: Rect,
    x: u16,
    y: u16,
    h: u16,
    ch: char,
    style: Style,
) {
    if h == 0 || x < bounds.x || x >= bounds.x + bounds.width {
        return;
    }

    let y2 = y.saturating_add(h);
    let ymin = y.max(bounds.y);
    let ymax = y2.min(bounds.y + bounds.height);

    for yy in ymin..ymax {
        buf[(x, yy)].set_char(ch).set_style(style);
    }
}
