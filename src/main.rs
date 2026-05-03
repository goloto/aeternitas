use std::{
    io,
    time::{Duration, Instant},
};

use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use input::Input;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Position},
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
    input_mode: InputMode,
    input: Input,
    current_time: String,
    project_start: Instant,
}

enum InputMode {
    Normal,
    Editing,
}

impl App {
    fn new() -> Self {
        Self {
            should_exit: false,
            input_mode: InputMode::Normal,
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

    fn draw(&self, frame: &mut Frame) {
        let main_block = Block::new()
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

        match self.input_mode {
            InputMode::Editing => {
                let input_block = Paragraph::new(String::from(&self.input.input)).block(
                    Block::new()
                        .title_top(Line::from(" What's project name? ".bold()).left_aligned())
                        .title_bottom(
                            Line::from_iter([" Exit editing ".yellow(), "<ESC>".bold().cyan()])
                                .right_aligned(),
                        )
                        .borders(Borders::ALL)
                        .border_style(Style::new().yellow()),
                );

                let layout = Layout::vertical(vec![
                    Constraint::Min(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ]);
                let timer_layout = Layout::horizontal(vec![
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ]);

                let [main_area, timer_area, input_area] = frame.area().layout(&layout);
                let [time_area, task_area] = timer_area.layout(&timer_layout);

                frame.set_cursor_position(Position::new(
                    input_area.x + u16::try_from(self.input.character_index).unwrap_or(0) + 1,
                    input_area.y + 1,
                ));

                frame.render_widget(main_block, main_area);
                frame.render_widget(timer_block, timer_area);
                frame.render_widget(current_time_block, time_area);
                frame.render_widget(current_task_block, task_area);
                frame.render_widget(input_block, input_area);
            }
            InputMode::Normal => {
                let layout = Layout::vertical(vec![Constraint::Min(3), Constraint::Length(3)]);
                let timer_layout = Layout::horizontal(vec![
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ]);
                let hint = Line::from_iter([
                    " Start timer ".to_span(),
                    "<N>".bold(),
                    " Stop timer ".to_span(),
                    "<S>".bold(),
                    " Add new project ".to_span(),
                    "<P>".bold(),
                    " Exit ".to_span(),
                    "<Q>".bold(),
                ])
                .yellow()
                .right_aligned();

                let [main_area, timer_area] = frame.area().layout(&layout);
                let [time_area, task_area] = timer_area.layout(&timer_layout);

                frame.render_widget(main_block.title_bottom(hint), main_area);
                frame.render_widget(timer_block, timer_area);
                frame.render_widget(current_time_block, time_area);
                frame.render_widget(current_task_block, task_area);
            }
        };
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match self.input_mode {
                    InputMode::Editing => match key_event.code {
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        KeyCode::Enter => self.submit_input(),
                        _ => self.input.handle_key_event(key_event),
                    },
                    _ => match key_event.code {
                        KeyCode::Char('n') => self.start_tracking(),
                        KeyCode::Char('s') => self.stop_tracking(),
                        KeyCode::Char('p') => self.new_input(),
                        KeyCode::Char('q') => self.exit(),
                        _ => {}
                    },
                }
            }
            _ => {}
        };
        Ok(())
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

    fn new_input(&mut self) {
        self.input = Input::new();
        self.input_mode = InputMode::Editing;
    }

    fn submit_input(&mut self) {
        self.input_mode = InputMode::Normal;
    }
}
