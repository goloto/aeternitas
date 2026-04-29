use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use input::Input;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Position},
    style::{Style, Stylize},
    text::{Line, Span, ToSpan},
    widgets::{Block, Borders, Paragraph},
};

mod input;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}

pub struct App {
    should_exit: bool,
    input_mode: InputMode,
    input: Input,
}

enum InputMode {
    Normal,
    Editing,
}

impl App {
    const fn new() -> Self {
        Self {
            should_exit: false,
            input_mode: InputMode::Normal,
            input: Input::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
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
                .centered(),
            )
            .borders(Borders::ALL)
            .border_style(Style::new().cyan());

        match self.input_mode {
            InputMode::Editing => {
                let input_block = Paragraph::new(String::from(&self.input.input)).block(
                    Block::new()
                        .title_top(Line::from(" Project name ".bold()).centered())
                        .title_bottom(
                            Line::from_iter([" Exit editing ".yellow(), "<ESC>".bold().cyan()])
                                .centered(),
                        )
                        .borders(Borders::ALL)
                        .border_style(Style::new().yellow()),
                );

                let layout = Layout::vertical(vec![Constraint::Min(3), Constraint::Length(3)]);
                let [main_area, input_area] = frame.area().layout(&layout);

                frame.set_cursor_position(Position::new(
                    input_area.x + u16::try_from(self.input.character_index).unwrap_or(0) + 1,
                    input_area.y + 1,
                ));
                frame.render_widget(main_block, main_area);
                frame.render_widget(input_block, input_area);
            }
            InputMode::Normal => {
                let layout = Layout::vertical(vec![Constraint::Min(3)]);
                let hint = Line::from_iter([
                    " Add new project ".to_span(),
                    "<P>".yellow().bold(),
                    " Exit ".to_span(),
                    "<Q>".yellow().bold(),
                ])
                .centered();

                let [main_area] = frame.area().layout(&layout);

                frame.render_widget(main_block.title_bottom(hint), main_area);
            }
        };
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.input_mode {
            InputMode::Editing => match key_event.code {
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                KeyCode::Enter => self.submit_input(),
                _ => self.input.handle_key_event(key_event),
            },
            _ => match key_event.code {
                KeyCode::Char('n') => self.start_tracking(),
                KeyCode::Char('p') => self.new_input(),
                KeyCode::Char('q') => self.exit(),
                _ => {}
            },
        }
    }

    fn exit(&mut self) {
        self.should_exit = true;
    }

    fn start_tracking(&mut self) {
        todo!();
    }

    fn new_input(&mut self) {
        self.input = Input::new();
        self.input_mode = InputMode::Editing;
    }

    fn submit_input(&mut self) {
        self.input_mode = InputMode::Normal;
    }
}
