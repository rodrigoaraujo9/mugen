use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    widgets::{Cell, Chart, Dataset, Row, Table},
    Terminal,
};
use std::time::Duration;

use crate::audio_capture::Matrix;

use crate::display::{
    oscilloscope::Oscilloscope, update_value_f, update_value_i, Dimension, GraphConfig,
};

pub struct VisualizerApp {
    graph: GraphConfig,
    oscilloscope: Oscilloscope,
    fps: FpsCounter,
}

struct FpsCounter {
    frames: usize,
    framerate: usize,
    last_update: std::time::Instant,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            frames: 0,
            framerate: 0,
            last_update: std::time::Instant::now(),
        }
    }

    fn tick(&mut self) {
        self.frames += 1;
        if self.last_update.elapsed().as_secs() >= 1 {
            self.framerate = self.frames;
            self.frames = 0;
            self.last_update = std::time::Instant::now();
        }
    }

    fn get(&self) -> usize {
        self.framerate
    }
}

impl VisualizerApp {
    pub fn new() -> Self {
        let graph = GraphConfig {
            axis_color: Color::DarkGray,
            labels_color: Color::Cyan,
            palette: vec![Color::Red, Color::Yellow, Color::Green, Color::Magenta],
            scale: 1.0,
            width: 2048,
            samples: 2048,
            sampling_rate: 48_000,
            references: true,
            show_ui: true,
            scatter: false,
            pause: false,
            marker_type: Marker::Braille,
        };

        Self {
            graph,
            oscilloscope: Oscilloscope::default(),
            fps: FpsCounter::new(),
        }
    }

    pub fn draw<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        audio_data: Option<Matrix<f64>>,
    ) -> std::io::Result<()> {
        self.fps.tick();

        terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0)])
                    .split(f.area());

                let mut area = chunks[0];

                let mut datasets = Vec::new();
                if let Some(data) = audio_data {
                    if self.graph.references {
                        datasets.extend(self.oscilloscope.references(&self.graph));
                    }
                    datasets.extend(self.oscilloscope.process(&self.graph, &data));
                }

                if self.graph.show_ui {
                    f.render_widget(
                        self.make_header(),
                        Rect {
                            x: area.x,
                            y: area.y,
                            width: area.width,
                            height: 1,
                        },
                    );
                    area.y += 1;
                    area.height = area.height.saturating_sub(1);
                }

                let chart = Chart::new(datasets.iter().map(|ds| Dataset::from(ds)).collect())
                    .x_axis(self.oscilloscope.axis(&self.graph, Dimension::X))
                    .y_axis(self.oscilloscope.axis(&self.graph, Dimension::Y));

                f.render_widget(chart, area);
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    pub fn handle_events(&mut self) -> std::io::Result<bool> {
        if event::poll(Duration::from_millis(0))? {
            let ev = event::read()?;
            return self.process_event(ev);
        }
        Ok(false)
    }

    fn process_event(&mut self, event: Event) -> std::io::Result<bool> {
        let mut quit = false;

        if let Event::Key(key) = event {
            if key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Char('c') | KeyCode::Char('q') | KeyCode::Char('w') => {
                        return Ok(true)
                    }
                    _ => {}
                }
            }

            let magnitude = match key.modifiers {
                KeyModifiers::SHIFT => 10.0,
                KeyModifiers::CONTROL => 5.0,
                KeyModifiers::ALT => 0.2,
                _ => 1.0,
            };

            match key.code {
                KeyCode::Up => update_value_f(&mut self.graph.scale, 0.01, magnitude, 0.0..10.0),
                KeyCode::Down => {
                    update_value_f(&mut self.graph.scale, -0.01, magnitude, 0.0..10.0)
                }
                KeyCode::Right => update_value_i(
                    &mut self.graph.samples,
                    true,
                    25,
                    magnitude,
                    0..self.graph.width * 2,
                ),
                KeyCode::Left => update_value_i(
                    &mut self.graph.samples,
                    false,
                    25,
                    magnitude,
                    0..self.graph.width * 2,
                ),
                KeyCode::Char(' ') => self.graph.pause = !self.graph.pause,
                KeyCode::Char('s') => self.graph.scatter = !self.graph.scatter,
                KeyCode::Char('h') => self.graph.show_ui = !self.graph.show_ui,
                KeyCode::Char('r') => self.graph.references = !self.graph.references,
                KeyCode::Char('q') => quit = true,
                KeyCode::Esc => {
                    self.graph.samples = self.graph.width;
                    self.graph.scale = 1.0;
                    self.oscilloscope.reset();
                }
                _ => {}
            }

            self.oscilloscope.handle(event);
        }

        Ok(quit)
    }

    fn make_header(&self) -> Table {
        let fps = self.fps.get();
        let scope_header = self.oscilloscope.header(&self.graph);

        Table::new(
            vec![Row::new(vec![
                Cell::from("oscillo::tjam").style(
                    Style::default()
                        .fg(*self.graph.palette.first().unwrap_or(&Color::White))
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(scope_header),
                Cell::from(format!("-{:.2}x+", self.graph.scale)),
                Cell::from(format!("{}/{} spf", self.graph.samples, self.graph.width)),
                Cell::from(format!("{}fps", fps)),
                Cell::from(if self.graph.scatter { "***" } else { "---" }),
                Cell::from(if self.graph.pause { "||" } else { "|>" }),
            ])],
            vec![
                Constraint::Percentage(35),
                Constraint::Percentage(25),
                Constraint::Percentage(7),
                Constraint::Percentage(13),
                Constraint::Percentage(6),
                Constraint::Percentage(6),
                Constraint::Percentage(6),
            ],
        )
        .style(Style::default().fg(self.graph.labels_color))
    }
}

impl Default for VisualizerApp {
    fn default() -> Self {
        Self::new()
    }
}
