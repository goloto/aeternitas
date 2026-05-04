use std::{
    io,
    time::{Duration, Instant},
};

use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use input::Input;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, ToSpan},
    widgets::{Block, Borders, Paragraph},
};
use time_formating::TimeFormating;

mod input;
mod time_formating;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}

pub struct App {
    should_exit: bool,
    screen: Screen,
    input: Input,
    current_time: String,
    project_start: Instant,
}

enum Screen {
    Editing,
    Dashboard,
}

impl App {
    fn new() -> Self {
        Self {
            should_exit: false,
            screen: Screen::Dashboard,
            input: Input::new(),
            current_time: String::new(),
            project_start: Instant::now(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|frame| self.draw(frame))?;

            let tick_rate = Duration::from_millis(1000);
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if !event::poll(timeout)? {
                self.current_time = Local::now().to_string();
                last_tick = Instant::now();
                continue;
            }

            self.handle_events()?;

            if self.should_exit {
                break;
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let current_time_block =
            Line::from_iter(["Now: ".to_span(), Span::from(&self.current_time)]);
        let current_task_block = Line::from_iter([
            "After launch: ".to_span(),
            Span::from(TimeFormating::from_seconds(
                self.project_start.elapsed().as_secs(),
            )),
        ]);

        let timer_block = Block::new()
            .title_top(Line::from("Current timer").left_aligned())
            .borders(Borders::ALL)
            .border_style(Style::new().gray());

        let layout = Layout::vertical(vec![Constraint::Min(3), Constraint::Length(3)]);
        let [main_area, hint_area] = frame.area().layout(&layout);

        match self.screen {
            Screen::Dashboard => {
                self.draw_dashboard(frame, main_area);
                self.draw_hint(frame, hint_area);
            }
            Screen::Editing => {
                self.draw_hint(frame, hint_area);
            }
        };
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match self.screen {
                Screen::Editing => match key_event.code {
                    KeyCode::Esc => self.to_screen(Screen::Dashboard),
                    KeyCode::Enter => self.submit_input(),
                    _ => self.input.handle_key_event(key_event),
                },
                Screen::Dashboard => match key_event.code {
                    KeyCode::Char('n') => self.start_tracking(),
                    KeyCode::Char('s') => self.stop_tracking(),
                    KeyCode::Char('p') => self.to_screen(Screen::Editing),
                    KeyCode::Char('q') => self.exit(),
                    _ => {}
                },
            },
            _ => {}
        };
        Ok(())
    }

    fn draw_dashboard(&mut self, frame: &mut Frame, area: Rect) {
        let dashboard = Block::new()
            .title_top(
                Line::from_iter([
                    " Welcome to ".to_span(),
                    "A".light_red().bold(),
                    "e".red().bold(),
                    "t".light_magenta().bold(),
                    "e".magenta().bold(),
                    "r".light_yellow().bold(),
                    "n".yellow().bold(),
                    "i".light_green().bold(),
                    "t".green().bold(),
                    "a".light_blue().bold(),
                    "s".blue().bold(),
                    "! / ".to_span(),
                    Span::from(env!("CARGO_PKG_VERSION")),
                    " ".to_span(),
                ])
                .left_aligned(),
            )
            .borders(Borders::ALL)
            .border_style(Style::new().cyan());

        frame.render_widget(dashboard, area);
    }

    fn draw_hint(&mut self, frame: &mut Frame, area: Rect) {
        let wrapper = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::new().gray());

        match self.screen {
            Screen::Dashboard => {
                let hint = Paragraph::new(Line::from_iter([
                    "<N>".bold(),
                    " New timer, ".to_span(),
                    "<P>".bold(),
                    " New project, ".to_span(),
                    "<Q>".bold(),
                    " Exit ".to_span(),
                ]));

                frame.render_widget(hint.block(wrapper), area);
            }
            Screen::Editing => {
                let hint = Paragraph::new(Line::from_iter([
                    "<ESC>".bold(),
                    " Cancel editing ".to_span(),
                ]));

                frame.render_widget(hint.block(wrapper), area);
            }
        }

    }

    fn exit(&mut self) {
        self.should_exit = true;
    }

    fn start_tracking(&mut self) {
        self.project_start = Instant::now();
    }

    fn stop_tracking(&mut self) {
        let elapsed = self.project_start.elapsed().as_secs();

        println!("elapsed: {elapsed}")
    }

    fn to_screen(&mut self, screen: Screen) {
        self.screen = screen;
    }

    fn new_input(&mut self) {
        self.input = Input::new();
    }

    fn submit_input(&mut self) {
        todo!()
    }
}
