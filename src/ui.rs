use std::io;
use std::io::stdout;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use crossterm::{
    event::{
        self, DisableFocusChange, EnableFocusChange, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Terminal,
};
use tokio::sync::{mpsc, watch};

use crate::audio_system::AudioHandle;
use crate::fx::adsr::Adsr;
use crate::patches::basic::{basic_source, BasicKind};

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
}

impl FocusPane {
    fn next(self) -> Self {
        match self {
            FocusPane::Waveforms => FocusPane::Adsr,
            FocusPane::Adsr => FocusPane::Bottom,
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

struct UiState {
    focus: FocusPane,
    waveforms: Vec<BasicKind>,
    waveform_idx: usize,
    adsr_param_idx: usize,
    adsr: Adsr,
    patch_name: String,
    muted: bool,
    volume: f32,
}

impl UiState {
    fn new(initial_adsr: Adsr) -> Self {
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
            patch_name: "Sine".to_string(),
            muted: false,
            volume: 1.0,
        }
    }

    fn selected_waveform(&self) -> BasicKind {
        self.waveforms[self.waveform_idx]
    }

    fn selected_adsr_param(&self) -> AdsrParam {
        AdsrParam::all()[self.adsr_param_idx]
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
                        if k.kind == KeyEventKind::Press {
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
                            handle.set_patch(basic_source(kind));
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

                    FocusPane::Bottom => {}
                }
            }

            _ = tokio::time::sleep(Duration::from_millis(16)) => {}
        }
    }

    stop.store(true, Ordering::Relaxed);
    terminal.show_cursor()?;
    Ok(())
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

    let area = f.area();
    f.render_widget(Clear, area);

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(kdr::BORDER))
        .style(Style::default().bg(kdr::BG0).fg(kdr::FG));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    // if inner.width < max_w as u16 {
    //     let msg = Paragraph::new("terminal too small")
    //         .alignment(Alignment::Center)
    //         .wrap(Wrap { trim: false })
    //         .style(Style::default().fg(kdr::FG).bg(kdr::BG0));
    //     f.render_widget(msg, inner);
    //     return;
    // }

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
    f.render_widget(Clear, area);

    let outer = Block::default()
        .borders(Borders::ALL)
        // .title(Span::styled(" mugen ", Style::default().fg(kdr::ORANGE).bold()))
        .border_style(Style::default().fg(kdr::BORDER))
        .style(Style::default().bg(kdr::BG0).fg(kdr::FG));

    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0), Constraint::Length(4)])
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

    draw_waveforms(f, top[0], ui);
    draw_adsr(f, top[1], ui);
    draw_bottom(f, rows[1], ui);
    draw_help(f, help_area, ui);
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

    lines.push(Line::from(Span::styled(
        "Edit ADSR",
        Style::default().fg(kdr::MUTED),
    )));
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
    let focused = ui.focus == FocusPane::Bottom;

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
        .title(tab_title("MIDI", focused))
        .border_style(border)
        .style(Style::default().bg(kdr::BG0));

    let lines = vec![Line::from(Span::styled(
        "placeholder",
        Style::default().fg(kdr::MUTED),
    ))];

    let w = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Center)
        .style(content_style);

    f.render_widget(w, area);
}

fn draw_help(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(kdr::BORDER))
        .style(Style::default().bg(kdr::BG0));

    let focus_name = match ui.focus {
        FocusPane::Waveforms => "Waveforms",
        FocusPane::Adsr => "ADSR",
        FocusPane::Bottom => "Bottom",
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

    let l2 = Line::from(vec![
        Span::styled("Waveforms: ", dim_style),
        Span::styled("↑/↓", key_style),
        Span::styled(" auto-apply  ", dim_style),
        Span::styled("|  ADSR: ", dim_style),
        Span::styled("↑/↓", key_style),
        Span::styled(" param  ", dim_style),
        Span::styled("←/→", key_style),
        Span::styled(" adjust", dim_style),
    ]);

    let l3 = Line::from(vec![
        Span::styled("Focus: ", dim_style),
        Span::styled(focus_name, strong),
        Span::styled("  |  Wave: ", dim_style),
        Span::styled(ui.patch_name.clone(), strong),
        Span::styled("  |  Vol ", dim_style),
        Span::styled(format!("{:.2}", ui.volume), strong),
        Span::styled("  ", dim_style),
        Span::styled(
            if ui.muted { "Muted" } else { "" },
            Style::default().fg(kdr::RED).bold(),
        ),
    ]);

    let w = Paragraph::new(vec![l1, l2, l3])
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::default().bg(kdr::BG0));

    f.render_widget(w, area);
}
