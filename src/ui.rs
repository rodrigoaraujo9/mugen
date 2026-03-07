use std::{
    collections::HashSet,
    io,
    io::stdout,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use crossterm::{
    event::{
        self, DisableFocusChange, EnableFocusChange, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use device_query::Keycode;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Terminal,
};
use tokio::sync::{mpsc, watch};
use tokio::time::sleep;
use crate::audio_system::AudioHandle;
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmp;
use crate::generators::basic::{basic_generator, BasicKind};

#[allow(dead_code)]
mod kdr {
    use ratatui::style::Color;

    pub const BG0: Color = Color::Rgb(24, 22, 22);
    pub const BG1: Color = Color::Rgb(40, 39, 39);
    pub const BORDER: Color = Color::Rgb(98, 94, 90);

    pub const FG: Color = Color::Rgb(197, 201, 197);
    pub const MUTED: Color = Color::Rgb(158, 155, 147);

    pub const BLUE: Color = Color::Rgb(139, 164, 176);
    pub const GREEN: Color = Color::Rgb(138, 154, 123);
    pub const ORANGE: Color = Color::Rgb(182, 146, 123);
    pub const YELLOW: Color = Color::Rgb(196, 178, 138);
    pub const RED: Color = Color::Rgb(196, 116, 110);
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
enum FocusPane {
    Waveforms,
    Adsr,
    Bottom,
    Lfo,
}

impl FocusPane {
    fn next(self) -> Self {
        match self {
            FocusPane::Waveforms => FocusPane::Adsr,
            FocusPane::Adsr => FocusPane::Lfo,
            FocusPane::Lfo => FocusPane::Bottom,
            FocusPane::Bottom => FocusPane::Waveforms,
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
    fn all() -> [AdsrParam; 4] {
        [
            AdsrParam::Attack,
            AdsrParam::Decay,
            AdsrParam::Sustain,
            AdsrParam::Release,
        ]
    }

    fn label_and_hint(self) -> (&'static str, &'static str) {
        match self {
            AdsrParam::Attack => ("Attack", "(s)"),
            AdsrParam::Decay => ("Decay", "(s)"),
            AdsrParam::Sustain => ("Sustain", "(0..1)"),
            AdsrParam::Release => ("Release", "(s)"),
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
    fn all() -> [LfoParam; 3] {
        [LfoParam::Kind, LfoParam::RateHz, LfoParam::Depth]
    }

    fn label_and_hint(self) -> (&'static str, &'static str) {
        match self {
            LfoParam::Kind => ("Wave", ""),
            LfoParam::RateHz => ("Rate", "(Hz)"),
            LfoParam::Depth => ("Depth", "(0..1)"),
        }
    }
}

struct UiState {
    focus: FocusPane,

    waveforms: Vec<BasicKind>,
    waveform_idx: usize,

    adsr_param_idx: usize,
    adsr: Adsr,

    lfo_param_idx: usize,
    lfo: LfoAmp,

    patch_name: String,
    muted: bool,
    volume: f32,
    held_keys: HashSet<Keycode>,
    octave_offset: i32,
}

impl UiState {
    fn new(initial_adsr: Adsr) -> Self {
        let initial_lfo = LfoAmp::new(BasicKind::Sine, 5.0, 0.0);

        Self {
            focus: FocusPane::Waveforms,
            waveforms: vec![
                BasicKind::Sine,
                BasicKind::Saw,
                BasicKind::Square,
                BasicKind::Triangle,
                BasicKind::Noise,
            ],
            waveform_idx: 0,
            adsr_param_idx: 0,
            adsr: initial_adsr,

            lfo_param_idx: 0,
            lfo: initial_lfo,

            patch_name: "Sine".to_string(),
            muted: false,
            volume: 1.0,
            held_keys: HashSet::new(),
            octave_offset: 0,
        }
    }

    fn selected_waveform(&self) -> BasicKind {
        self.waveforms[self.waveform_idx]
    }

    fn selected_adsr_param(&self) -> AdsrParam {
        AdsrParam::all()[self.adsr_param_idx]
    }

    fn selected_lfo_param(&self) -> LfoParam {
        LfoParam::all()[self.lfo_param_idx]
    }
}

pub async fn run_ui(
    handle: AudioHandle,
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

    let (key_tx, mut key_rx) = mpsc::unbounded_channel::<KeyEvent>();

    let stop = Arc::new(AtomicBool::new(false));
    let stop_bg = stop.clone();
    let focused_bg = focused.clone();

    std::thread::spawn(move || {
        while !stop_bg.load(Ordering::Relaxed) {
            if event::poll(Duration::from_millis(50)).ok() == Some(true) {
                match event::read() {
                    Ok(Event::Key(k)) => {
                        if k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Release {
                            let _ = key_tx.send(k);
                        }
                    }
                    Ok(Event::FocusLost) => {
                        focused_bg.store(false, Ordering::Relaxed);
                    }
                    Ok(Event::FocusGained) => {
                        focused_bg.store(true, Ordering::Relaxed);
                    }
                    _ => {}
                }
            }
        }
    });

    let ui_start = std::time::Instant::now();
    let mut show_intro = true;

    let mut snap_rx = handle.subscribe();
    let mut held_keys_rx = handle.held_keys_rx.clone();
    let mut ui = UiState::new(Adsr::new(0.01, 0.10, 0.70, 0.25));

    loop {
        if show_intro && ui_start.elapsed() >= Duration::from_secs(1) {
            show_intro = false;
        }

        if show_intro {
            terminal.draw(draw_intro)?;
        } else {
            terminal.draw(|f| draw_ui(f, &ui))?;
        }

        tokio::select! {
            _ = snap_rx.changed() => {
                let s = snap_rx.borrow().clone();
                ui.patch_name = s.patch_name;
                ui.muted = s.muted;
                ui.volume = s.volume;
                ui.adsr = s.adsr;
                ui.lfo = s.lfo;
            }

            _ = held_keys_rx.changed() => {
                ui.held_keys = held_keys_rx.borrow().clone();
            }

            k = key_rx.recv() => {
                let Some(k) = k else { break; };

                if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c')) {
                    let _ = shutdown_tx.send(true);
                    break;
                }
                if matches!(k.code, KeyCode::Char('q')) {
                    let _ = shutdown_tx.send(true);
                    break;
                }

                if show_intro {
                    continue;
                }

                if matches!(k.code, KeyCode::Tab) {
                    ui.focus = ui.focus.next();
                    continue;
                }

                match ui.focus {
                    FocusPane::Waveforms => {
                        let mut changed = false;
                        match k.code {
                            KeyCode::Up => {
                                if ui.waveform_idx > 0 {
                                    ui.waveform_idx -= 1;
                                    changed = true;
                                }
                            }
                            KeyCode::Down => {
                                if ui.waveform_idx + 1 < ui.waveforms.len() {
                                    ui.waveform_idx += 1;
                                    changed = true;
                                }
                            }
                            _ => {}
                        }
                        if changed {
                            let kind = ui.selected_waveform();
                            handle.set_patch(basic_generator(kind));
                        }
                    }

                    FocusPane::Adsr => {
                        match k.code {
                            KeyCode::Up => {
                                if ui.adsr_param_idx > 0 { ui.adsr_param_idx -= 1; }
                            }
                            KeyCode::Down => {
                                if ui.adsr_param_idx + 1 < 4 { ui.adsr_param_idx += 1; }
                            }
                            KeyCode::Left => {
                                tweak_adsr(&mut ui, -1);
                                handle.set_adsr(ui.adsr);
                            }
                            KeyCode::Right => {
                                tweak_adsr(&mut ui, 1);
                                handle.set_adsr(ui.adsr);
                            }
                            _ => {}
                        }
                    }

                    FocusPane::Bottom => {
                        match k.code {
                            KeyCode::Right => {
                                if ui.octave_offset < 4 {
                                    ui.octave_offset += 1;
                                    handle.set_octave(ui.octave_offset);
                                }
                            }
                            KeyCode::Left => {
                                if ui.octave_offset > -4 {
                                    ui.octave_offset -= 1;
                                    handle.set_octave(ui.octave_offset);
                                }
                            }
                            _ => {}
                        }
                    }
                    FocusPane::Lfo => {
                        match k.code {
                            KeyCode::Up => {
                                if ui.lfo_param_idx > 0 { ui.lfo_param_idx -= 1; }
                            }
                            KeyCode::Down => {
                                if ui.lfo_param_idx + 1 < 3 { ui.lfo_param_idx += 1; }
                            }
                            KeyCode::Left => {
                                tweak_lfo(&mut ui, -1);
                                handle.set_lfoamp(ui.lfo);
                            }
                            KeyCode::Right => {
                                tweak_lfo(&mut ui, 1);
                                handle.set_lfoamp(ui.lfo);
                            }
                            _ => {}
                        }
                    }
                }
            }

            _ = sleep(Duration::from_millis(16)) => {}
        }
    }

    stop.store(true, Ordering::Relaxed);
    terminal.show_cursor()?;
    Ok(())
}

fn tweak_lfo(ui: &mut UiState, dir: i32) {
    let d = if dir < 0 { -1 } else { 1 };

    match ui.selected_lfo_param() {
        LfoParam::Kind => {
            ui.lfo.kind = next_basic_kind(ui.lfo.kind, d);
        }
        LfoParam::RateHz => {
            let step = 0.25;
            ui.lfo.rate_hz = (ui.lfo.rate_hz + (d as f32) * step).clamp(0.05, 40.0);
        }
        LfoParam::Depth => {
            let step = 0.02;
            ui.lfo.depth = (ui.lfo.depth + (d as f32) * step).clamp(0.0, 1.0);
        }
    }
}

fn next_basic_kind(k: BasicKind, dir: i32) -> BasicKind {
    let all = [
        BasicKind::Sine,
        BasicKind::Saw,
        BasicKind::Square,
        BasicKind::Triangle,
        BasicKind::Noise,
    ];
    let idx = all.iter().position(|x| *x == k).unwrap_or(0) as i32;
    let n = all.len() as i32;
    let next = (idx + dir).rem_euclid(n) as usize;
    all[next]
}

fn tweak_adsr(ui: &mut UiState, dir: i32) {
    let step = ui_selected_small_step(ui.selected_adsr_param());
    let d = if dir < 0 { -step } else { step };

    match ui.selected_adsr_param() {
        AdsrParam::Attack => ui.adsr.attack_s = (ui.adsr.attack_s + d).max(0.0),
        AdsrParam::Decay => ui.adsr.decay_s = (ui.adsr.decay_s + d).max(0.0),
        AdsrParam::Sustain => ui.adsr.sustain = (ui.adsr.sustain + d).clamp(0.0, 1.0),
        AdsrParam::Release => ui.adsr.release_s = (ui.adsr.release_s + d).max(0.0),
    }

    ui.adsr.attack_s = ui.adsr.attack_s.min(10.0);
    ui.adsr.decay_s = ui.adsr.decay_s.min(10.0);
    ui.adsr.release_s = ui.adsr.release_s.min(10.0);
}

fn ui_selected_small_step(p: AdsrParam) -> f32 {
    match p {
        AdsrParam::Sustain => 0.01,
        _ => 0.01,
    }
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
                owned.extend(std::iter::repeat(' ').take(pad));
            }
            Line::from(Span::raw(owned).bold())
        })
        .collect();

    f.render_widget(Clear, area);

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(kdr::BORDER))
        .style(Style::default().bg(kdr::BG0).fg(kdr::FG));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let total_h = art.len() as u16;
    let main_lines: Vec<Line> = lines.into_iter().take((total_h - 1) as usize).collect();
    let synthesis_line = Line::from(Span::styled(
        "s y n t h e s i s",
        Style::default().fg(kdr::ORANGE).bold(),
    ));

    let y = inner.y + (inner.height.saturating_sub(total_h)) / 2;

    let main_area = Rect {
        x: inner.x,
        y,
        width: inner.width,
        height: (total_h - 1).min(inner.height),
    };
    f.render_widget(
        Paragraph::new(main_lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(kdr::FG).bg(kdr::BG0)),
        main_area,
    );

    let synth_y = y + total_h - 1;
    if synth_y < inner.y + inner.height {
        let synth_area = Rect { x: inner.x, y: synth_y, width: inner.width, height: 1 };
        f.render_widget(
            Paragraph::new(vec![synthesis_line])
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

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(kdr::BORDER))
        .style(Style::default().bg(kdr::BG0).fg(kdr::FG));

    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0), Constraint::Length(3)])
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
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(rows[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ])
        .split(top[1]);

    draw_waveforms(f, top[0], ui);
    draw_adsr(f, right[0], ui);
    draw_lfo(f, right[1], ui);
    draw_bottom(f, rows[1], ui);
    draw_help(f, help_area, ui);
}

fn draw_too_small(f: &mut ratatui::Frame, area: Rect, min_w: u16, min_h: u16) {
    f.render_widget(Clear, area);
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(kdr::BORDER))
        .style(Style::default().bg(kdr::BG0).fg(kdr::FG));
    let inner = outer.inner(area);
    f.render_widget(outer, area);
    let msg = vec![
        Line::from(Span::styled("terminal too small", Style::default().fg(kdr::ORANGE).bold())),
        Line::from(Span::styled(
            format!("need {}×{}  —  currently {}×{}", min_w, min_h, area.width, area.height),
            Style::default().fg(kdr::MUTED),
        )),
    ];
    let h = msg.len() as u16;
    let y = inner.y + inner.height.saturating_sub(h) / 2;
    let msg_area = Rect { x: inner.x, y, width: inner.width, height: h.min(inner.height) };
    f.render_widget(
        Paragraph::new(msg).alignment(Alignment::Center).style(Style::default().bg(kdr::BG0)),
        msg_area,
    );
}

fn draw_lfo(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let focused = ui.focus == FocusPane::Lfo;

    let border = if focused {
        Style::default().fg(kdr::FG)
    } else {
        Style::default().fg(kdr::BORDER)
    };

    let content_style = if focused {
        Style::default().fg(kdr::FG).bg(kdr::BG0)
    } else {
        Style::default().fg(kdr::MUTED).bg(kdr::BG0)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(tab_title("lfo", focused))
        .border_style(border)
        .style(Style::default().bg(kdr::BG0));

    let inner = block.inner(area);
    let width = inner.width as usize;

    let params = LfoParam::all();
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    if width == 0 {
        let w = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left)
            .style(content_style);
        f.render_widget(w, area);
        return;
    }

    let right_padding = 1usize;

    for (i, p) in params.iter().copied().enumerate() {
        let is_sel = i == ui.lfo_param_idx;

        let val = match p {
            LfoParam::Kind => ui.lfo.kind.name().to_string(),
            LfoParam::RateHz => format!("{:.2}", ui.lfo.rate_hz),
            LfoParam::Depth => format!("{:.2}", ui.lfo.depth),
        };

        let (label, hint) = p.label_and_hint();
        let prefix = if is_sel { "› " } else { "  " };

        let prefix_style = if is_sel {
            Style::default().fg(kdr::ORANGE).bold()
        } else {
            Style::default().fg(kdr::MUTED)
        };

        let label_style = if is_sel {
            Style::default().fg(kdr::FG).bold()
        } else {
            Style::default().fg(kdr::FG)
        };

        let hint_style = Style::default().fg(kdr::MUTED);

        let value_style = if is_sel {
            Style::default().fg(kdr::FG).bold()
        } else {
            Style::default().fg(kdr::FG)
        };

        let left_label = format!("{label} ");
        let left_len = prefix.chars().count() + left_label.chars().count() + hint.chars().count();

        let val_len = val.chars().count();
        let min_gap = 2usize;

        let usable_width = width.saturating_sub(right_padding);
        let pad_len = usable_width.saturating_sub(left_len + min_gap + val_len);
        let pad = " ".repeat(pad_len + min_gap);

        lines.push(Line::from(vec![
            Span::styled(prefix, prefix_style),
            Span::styled(left_label, label_style),
            Span::styled(hint, hint_style),
            Span::raw(pad),
            Span::styled(val, value_style),
            Span::raw(" ".repeat(right_padding)),
        ]));
    }

    let w = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left)
        .style(content_style);

    f.render_widget(w, area);
}

fn draw_logo(f: &mut ratatui::Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("無", Style::default().fg(kdr::FG)).bold()),
        Line::from(Span::styled("限", Style::default().fg(kdr::FG)).bold()),
    ];

    let total_h = lines.len() as u16;
    let y = area.y + area.height.saturating_sub(total_h) / 2;

    let centered = Rect {
        x: area.x,
        y,
        width: area.width,
        height: total_h.min(area.height),
    };

    let w = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::default().bg(kdr::BG0));

    f.render_widget(w, centered);
}

fn tab_title(name: &'static str, focused: bool) -> Span<'static> {
    let t = format!(" {name} ");
    if focused {
        Span::styled(t, Style::default().fg(kdr::ORANGE).bold())
    } else {
        Span::styled(t, Style::default().fg(kdr::MUTED))
    }
}

fn draw_waveforms(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let focused = ui.focus == FocusPane::Waveforms;

    let border = if focused {
        Style::default().fg(kdr::FG)
    } else {
        Style::default().fg(kdr::BORDER)
    };

    let content_style = if focused {
        Style::default().fg(kdr::FG).bg(kdr::BG0)
    } else {
        Style::default().fg(kdr::MUTED).bg(kdr::BG0)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(tab_title("waveforms", focused))
        .border_style(border)
        .style(Style::default().bg(kdr::BG0));

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (i, k) in ui.waveforms.iter().copied().enumerate() {
        let name = k.name();
        let is_sel = i == ui.waveform_idx;

        let line = if is_sel {
            Line::from(vec![
                Span::styled("› ", Style::default().fg(kdr::ORANGE).bold()),
                Span::styled(name, Style::default().fg(kdr::FG).bold()),
            ])
        } else {
            Line::from(vec![
                Span::styled("  ", Style::default().fg(kdr::MUTED)),
                Span::styled(name, Style::default().fg(kdr::FG)),
            ])
        };

        lines.push(line);
    }

    let w = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left)
        .style(content_style);

    f.render_widget(w, area);
}

fn draw_adsr(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let focused = ui.focus == FocusPane::Adsr;

    let border = if focused {
        Style::default().fg(kdr::FG)
    } else {
        Style::default().fg(kdr::BORDER)
    };

    let content_style = if focused {
        Style::default().fg(kdr::FG).bg(kdr::BG0)
    } else {
        Style::default().fg(kdr::MUTED).bg(kdr::BG0)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(tab_title("adsr", focused))
        .border_style(border)
        .style(Style::default().bg(kdr::BG0));

    let inner = block.inner(area);
    let width = inner.width as usize;

    let params = AdsrParam::all();
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));

    if width == 0 {
        let w = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left)
            .style(content_style);
        f.render_widget(w, area);
        return;
    }

    let right_padding = 1usize;

    for (i, p) in params.iter().copied().enumerate() {
        let is_sel = i == ui.adsr_param_idx;

        let val = match p {
            AdsrParam::Attack => format!("{:.3}", ui.adsr.attack_s),
            AdsrParam::Decay => format!("{:.3}", ui.adsr.decay_s),
            AdsrParam::Sustain => format!("{:.2}", ui.adsr.sustain),
            AdsrParam::Release => format!("{:.3}", ui.adsr.release_s),
        };

        let (label, hint) = p.label_and_hint();
        let prefix = if is_sel { "› " } else { "  " };

        let prefix_style = if is_sel {
            Style::default().fg(kdr::ORANGE).bold()
        } else {
            Style::default().fg(kdr::MUTED)
        };

        let label_style = if is_sel {
            Style::default().fg(kdr::FG).bold()
        } else {
            Style::default().fg(kdr::FG)
        };

        let hint_style = Style::default().fg(kdr::MUTED);

        let value_style = if is_sel {
            Style::default().fg(kdr::FG).bold()
        } else {
            Style::default().fg(kdr::FG)
        };

        let left_label = format!("{label} ");
        let left_len =
            prefix.chars().count() + left_label.chars().count() + hint.chars().count();

        let val_len = val.chars().count();
        let min_gap = 2usize;

        let usable_width = width.saturating_sub(right_padding);
        let pad_len = usable_width.saturating_sub(left_len + min_gap + val_len);
        let pad = " ".repeat(pad_len + min_gap);

        lines.push(Line::from(vec![
            Span::styled(prefix, prefix_style),
            Span::styled(left_label, label_style),
            Span::styled(hint, hint_style),
            Span::raw(pad),
            Span::styled(val, value_style),
            Span::raw(" ".repeat(right_padding)),
        ]));
    }

    let w = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left)
        .style(content_style);

    f.render_widget(w, area);
}

fn draw_bottom(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    use ratatui::buffer::Buffer;

    fn fill_rect(buf: &mut Buffer, bounds: Rect, x: u16, y: u16, w: u16, h: u16, st: Style) {
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
                buf[(xx, yy)].set_char(' ').set_style(st);
            }
        }
    }

    fn vline(buf: &mut Buffer, bounds: Rect, x: u16, y: u16, h: u16, ch: char, st: Style) {
        if h == 0 {
            return;
        }
        if x < bounds.x || x >= bounds.x + bounds.width {
            return;
        }
        let y2 = y.saturating_add(h);
        let ymin = y.max(bounds.y);
        let ymax = y2.min(bounds.y + bounds.height);

        for yy in ymin..ymax {
            buf[(x, yy)].set_char(ch).set_style(st);
        }
    }

    let focused = ui.focus == FocusPane::Bottom;

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
        WhiteKey { code: Keycode::A, label: "a" },
        WhiteKey { code: Keycode::S, label: "s" },
        WhiteKey { code: Keycode::D, label: "d" },
        WhiteKey { code: Keycode::F, label: "f" },
        WhiteKey { code: Keycode::G, label: "g" },
        WhiteKey { code: Keycode::H, label: "h" },
        WhiteKey { code: Keycode::J, label: "j" },
        WhiteKey { code: Keycode::K, label: "k" },
        WhiteKey { code: Keycode::L, label: "l" },
        WhiteKey { code: Keycode::Semicolon, label: ";" },
        WhiteKey { code: Keycode::Apostrophe, label: "'" },
    ];

    let black_keys = [
        BlackKey { code: Keycode::W, label: "w", gap_after: 0 },
        BlackKey { code: Keycode::E, label: "e", gap_after: 1 },
        BlackKey { code: Keycode::T, label: "t", gap_after: 3 },
        BlackKey { code: Keycode::Y, label: "y", gap_after: 4 },
        BlackKey { code: Keycode::U, label: "u", gap_after: 5 },
        BlackKey { code: Keycode::O, label: "o", gap_after: 7 },
        BlackKey { code: Keycode::P, label: "p", gap_after: 8 },
    ];

    let is_pressed = |code: &Keycode| ui.held_keys.contains(code);

    let bounds = area;
    if bounds.width < 18 || bounds.height < 6 {
        return;
    }

    let n_white = white_keys.len();
    let total_w = bounds.width as usize;
    let total_h = bounds.height as usize;

    let white_w = (total_w / n_white).max(4);
    let used_w = white_w * n_white;
    let x0 = bounds.x as usize + (total_w.saturating_sub(used_w)) / 2;

    let white_h = total_h;
    let black_h = ((white_h * 60) / 100).max(2);
    let black_w = ((white_w * 55) / 100).max(2);

    let buf = f.buffer_mut();

    let bg = Style::default().bg(kdr::BG0);
    fill_rect(buf, bounds, bounds.x, bounds.y, bounds.width, bounds.height, bg);

    let white_bg = if focused { kdr::FG } else { kdr::BORDER };
    let white_fill = Style::default().bg(white_bg).fg(kdr::BG0);

    let orange_fill = Style::default().bg(kdr::ORANGE).fg(kdr::BG0);

    let sep_style = Style::default().fg(kdr::BG0).bg(white_bg);

    let black_fill = Style::default().bg(kdr::BG0).fg(kdr::FG);
    let black_pressed = Style::default().bg(kdr::ORANGE).fg(kdr::BG0);

    for (i, wk) in white_keys.iter().enumerate() {
        let x = (x0 + i * white_w) as u16;
        let y = bounds.y;
        let w = white_w as u16;
        let h = white_h as u16;

        let st = if is_pressed(&wk.code) { orange_fill } else { white_fill };
        fill_rect(buf, bounds, x, y, w, h, st);
    }

    for i in 0..(n_white - 1) {
        let x = (x0 + (i + 1) * white_w - 1) as u16;
        vline(buf, bounds, x, bounds.y, bounds.height, '│', sep_style);
    }

    for (i, wk) in white_keys.iter().enumerate() {
        if !is_pressed(&wk.code) {
            continue;
        }
        let x = (x0 + i * white_w) as u16;
        fill_rect(buf, bounds, x, bounds.y, white_w as u16, bounds.height, orange_fill);
    }
    for i in 0..(n_white - 1) {
        let left_p = is_pressed(&white_keys[i].code);
        let right_p = is_pressed(&white_keys[i + 1].code);
        if left_p || right_p {
            let x = (x0 + (i + 1) * white_w - 1) as u16;
            fill_rect(buf, bounds, x, bounds.y, 1, bounds.height, orange_fill);
        }
    }

    let label_y = bounds.y + bounds.height - 1;
    for (i, wk) in white_keys.iter().enumerate() {
        let x = (x0 + i * white_w) as u16;
        let w = white_w as u16;
        let lx = x + (w / 2);

        if lx >= bounds.x && lx < bounds.x + bounds.width {
            let pressed = is_pressed(&wk.code);
            let st = if pressed {
                Style::default().fg(kdr::BG0).bg(kdr::ORANGE).bold()
            } else {
                Style::default().fg(kdr::BG0).bg(white_bg)
            };
            buf[(lx, label_y)]
                .set_char(wk.label.chars().next().unwrap_or(' '))
                .set_style(st);
        }
    }
    for bk in black_keys.iter() {
        let pressed = is_pressed(&bk.code);

        let center_x = x0 + (bk.gap_after + 1) * white_w;
        let bx = center_x.saturating_sub(black_w / 2);

        let x = bx as u16;
        let y = bounds.y;
        let w = black_w as u16;
        let h = black_h as u16;

        fill_rect(buf, bounds, x, y, w, h, if pressed { black_pressed } else { black_fill });

        let lx = x + (w / 2);
        let ly = y + h - 1;
        if lx >= bounds.x && lx < bounds.x + bounds.width && ly >= bounds.y && ly < bounds.y + bounds.height
        {
            let st = if pressed {
                Style::default().fg(kdr::BG0).bg(kdr::ORANGE).bold()
            } else {
                Style::default().fg(kdr::FG).bg(kdr::BG0)
            };
            buf[(lx, ly)]
                .set_char(bk.label.chars().next().unwrap_or(' '))
                .set_style(st);
        }
    }
}

fn draw_help(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {

    let focus_name = match ui.focus {
        FocusPane::Waveforms => "Waveforms",
        FocusPane::Adsr => "ADSR",
        FocusPane::Lfo => "LFO",
        FocusPane::Bottom => "Keyboard",
    };

    let key_style = Style::default().fg(kdr::ORANGE).bold();
    let dim_style = Style::default().fg(kdr::MUTED);
    let strong = Style::default().fg(kdr::FG).bold();

    let l1 = Line::from(vec![
        Span::styled("Tab", key_style),
        Span::styled(" focus  ", dim_style),
        Span::styled("↑/↓", key_style),
        Span::styled(" select  ", dim_style),
        Span::styled("←/→", key_style),
        Span::styled(" change  ", dim_style),
        Span::styled("q", key_style),
        Span::styled(" quit  ", dim_style),
        Span::styled("Ctrl+C", key_style),
        Span::styled(" quit", dim_style),
    ]);

    let l3 = Line::from(vec![
        Span::styled("Focus: ", dim_style),
        Span::styled(focus_name, strong),
        Span::styled("  |  Wave: ", dim_style),
        Span::styled(ui.patch_name.clone(), strong),
        Span::styled("  |  Vol ", dim_style),
        Span::styled(format!("{:.2}", ui.volume), strong),
        Span::styled(
            if ui.muted { "Muted" } else { "" },
            Style::default().fg(kdr::ORANGE).bold(),
        ),
        Span::styled("  |  Oct ", dim_style),
        Span::styled(
            format!("{:+}", ui.octave_offset),
            if ui.octave_offset == 0 { strong } else { Style::default().fg(kdr::YELLOW).bold() }
        ),
    ]);

    let w = Paragraph::new(vec![Line::from(""), l1, l3])        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::default().bg(kdr::BG0));

    f.render_widget(w, area);
}
