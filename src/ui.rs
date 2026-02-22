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
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use tokio::sync::{mpsc, watch};

use crate::audio_system::AudioHandle;
use crate::fx::adsr::Adsr;
use crate::patches::basic::{basic_source, BasicKind};

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
    fn name(self) -> &'static str {
        match self {
            AdsrParam::Attack => "Attack (s)",
            AdsrParam::Decay => "Decay (s)",
            AdsrParam::Sustain => "Sustain (0..1)",
            AdsrParam::Release => "Release (s)",
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
        AdsrParam::Attack => {
            ui.adsr.attack_s = (ui.adsr.attack_s + d).max(0.0);
        }
        AdsrParam::Decay => {
            ui.adsr.decay_s = (ui.adsr.decay_s + d).max(0.0);
        }
        AdsrParam::Sustain => {
            ui.adsr.sustain = (ui.adsr.sustain + d).clamp(0.0, 1.0);
        }
        AdsrParam::Release => {
            ui.adsr.release_s = (ui.adsr.release_s + d).max(0.0);
        }
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
    let outer = Block::default().borders(Borders::ALL);
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    if inner.width < max_w as u16 {
        let msg = Paragraph::new("terminal too small")
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(msg, inner);
        return;
    }

    let widget = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });

    let total_h = art.len() as u16;
    let y = inner.y + (inner.height.saturating_sub(total_h)) / 2;

    let centered = Rect {
        x: inner.x,
        y,
        width: inner.width,
        height: total_h.min(inner.height),
    };

    f.render_widget(widget, centered);
}

fn draw_ui(f: &mut ratatui::Frame, ui: &UiState) {
    let outer = Block::default().borders(Borders::ALL).title(" mugen ");
    let area = f.area();
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(4)])
        .split(inner);

    let content_area = main[0];
    let help_area = main[1];

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

fn draw_waveforms(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let focused = ui.focus == FocusPane::Waveforms;

    let title = if focused { " waveforms * " } else { " waveforms " };
    let border = if focused {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let content_style = if focused {
        Style::default()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::raw("Current: ").dim(),
        Span::raw(ui.patch_name.clone()).bold(),
        Span::raw("  "),
        Span::raw(format!("vol {:.2}", ui.volume)).dim(),
        Span::raw("  "),
        Span::raw(if ui.muted { "muted" } else { "" }).dim(),
    ]));
    lines.push(Line::from(""));

    for (i, k) in ui.waveforms.iter().copied().enumerate() {
        let name = k.name();
        let is_sel = i == ui.waveform_idx;

        let line = if is_sel {
            Line::from(vec![Span::raw("› ").bold(), Span::raw(name).bold()])
        } else {
            Line::from(vec![Span::raw("  "), Span::raw(name)])
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

    let title = if focused { " adsr * " } else { " adsr " };
    let border = if focused {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let content_style = if focused {
        Style::default()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border);

    let params = AdsrParam::all();
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from("Edit ADSR"));
    lines.push(Line::from(""));

    for (i, p) in params.iter().copied().enumerate() {
        let is_sel = i == ui.adsr_param_idx;

        let val = match p {
            AdsrParam::Attack => format!("{:.3}", ui.adsr.attack_s),
            AdsrParam::Decay => format!("{:.3}", ui.adsr.decay_s),
            AdsrParam::Sustain => format!("{:.2}", ui.adsr.sustain),
            AdsrParam::Release => format!("{:.3}", ui.adsr.release_s),
        };

        let prefix = if is_sel { "› " } else { "  " };

        let line = if is_sel {
            Line::from(vec![
                Span::raw(prefix).bold(),
                Span::raw(format!("{:<14}", p.name())).bold(),
                Span::raw(" "),
                Span::raw(val).bold(),
            ])
        } else {
            Line::from(vec![
                Span::raw(prefix),
                Span::raw(format!("{:<14}", p.name())).dim(),
                Span::raw(" "),
                Span::raw(val),
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

fn draw_bottom(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let focused = ui.focus == FocusPane::Bottom;

    let title = if focused { " bottom * " } else { " bottom " };
    let border = if focused {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let content_style = if focused {
        Style::default()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border);

    let lines = vec![Line::from("placeholder")];

    let w = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Center)
        .style(content_style);

    f.render_widget(w, area);
}

fn draw_help(f: &mut ratatui::Frame, area: Rect, ui: &UiState) {
    let style = Style::default().fg(Color::DarkGray);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(style);

    let focus_name = match ui.focus {
        FocusPane::Waveforms => "Waveforms",
        FocusPane::Adsr => "ADSR",
        FocusPane::Bottom => "Bottom",
    };

    let l1 = Line::from(vec![
        Span::raw("Tab").bold(),
        Span::styled(" focus  ", style),
        Span::raw("↑/↓").bold(),
        Span::styled(" select  ", style),
        Span::raw("←/→").bold(),
        Span::styled(" change  ", style),
        Span::raw("q").bold(),
        Span::styled(" quit  ", style),
        Span::raw("Ctrl+C").bold(),
        Span::styled(" quit", style),
    ]);

    let l2 = Line::from(vec![
        Span::styled("Waveforms: ", style),
        Span::raw("↑/↓").bold(),
        Span::styled(" auto-apply  ", style),
        Span::styled("|  ADSR: ", style),
        Span::raw("↑/↓").bold(),
        Span::styled(" param  ", style),
        Span::raw("←/→").bold(),
        Span::styled(" adjust", style),
    ]);

    let l3 = Line::from(vec![
        Span::styled("Focus: ", style),
        Span::raw(focus_name).bold(),
        Span::styled("  |  Wave: ", style),
        Span::raw(ui.patch_name.clone()).bold(),
        Span::styled("  |  Vol ", style),
        Span::raw(format!("{:.2}", ui.volume)).bold(),
        Span::styled("  ", style),
        Span::styled(if ui.muted { "Muted" } else { "" }, style),
    ]);

    let w = Paragraph::new(vec![l1, l2, l3])
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(style);

    f.render_widget(w, area);
}
