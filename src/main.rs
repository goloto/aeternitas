use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use input::Input;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, ToSpan},
    widgets::{Block, Borders, List, ListState, Paragraph},
};

use crate::db::Db;

mod db;
mod input;
mod time_formating;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}

pub struct App {
    should_exit: bool,
    screen: Screen,
    input: Input,
    db: Db,
    project_list: ListState,
}

enum Screen {
    Dashboard,
    NewTimer,
    NewProject,
}

impl App {
    fn new() -> Self {
        Self {
            should_exit: false,
            screen: Screen::Dashboard,
            input: Input::new(),
            db: Db::new(),
            project_list: ListState::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|frame| self.draw(frame))?;

            let tick_rate = Duration::from_millis(1000);
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if !event::poll(timeout)? {
                // self.current_time = Local::now().to_string();
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
        // let current_time_block =
        //     Line::from_iter(["Now: ".to_span(), Span::from(&self.current_time)]);
        // let current_task_block = Line::from_iter([
        //     "After launch: ".to_span(),
        //     Span::from(TimeFormating::from_seconds(
        //         self.project_start.elapsed().as_secs(),
        //     )),
        // ]);

        // let timer_block = Block::new()
        //     .title_top(Line::from("Current timer").left_aligned())
        //     .borders(Borders::ALL)
        //     .border_style(Style::new().gray());

        let layout = Layout::vertical(vec![Constraint::Min(3), Constraint::Length(3)]);
        let [main_area, hint_area] = frame.area().layout(&layout);

        match self.screen {
            Screen::Dashboard => {
                self.draw_dashboard(frame, main_area);
                self.draw_hint(frame, hint_area);
            }
            Screen::NewTimer => {
                self.draw_new_timer(frame, main_area);
                self.draw_hint(frame, hint_area);
            }
            Screen::NewProject => {
                self.draw_new_project(frame, main_area);
                self.draw_hint(frame, hint_area);
            }
        };
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match self.screen {
                Screen::Dashboard => match key_event.code {
                    KeyCode::Char('n') => self.to_screen(Screen::NewTimer),
                    KeyCode::Char('p') => self.to_screen(Screen::NewProject),
                    KeyCode::Char('q') => self.exit(),
                    _ => {}
                },
                Screen::NewTimer => match key_event.code {
                    KeyCode::Esc => self.from_screen(Screen::NewTimer),
                    KeyCode::Char('j') | KeyCode::Down => self.project_list.select_next(),
                    KeyCode::Char('k') | KeyCode::Up => self.project_list.select_previous(),
                    _ => {}
                },
                Screen::NewProject => match key_event.code {
                    KeyCode::Esc => self.from_screen(Screen::NewProject),
                    KeyCode::Enter => self.submit_new_project(),
                    _ => self.input.handle_key_event(key_event),
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

    fn draw_new_project(&mut self, frame: &mut Frame, area: Rect) {
        let wrapper = Block::new()
            .title_top(Line::from_iter([" New project ".to_span()]).left_aligned())
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());

        let layout = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ],
        );
        let [
            input_title_area,
            input_area,
            _empty_line,
            projects_list_title_area,
            projects_list_area,
        ] = wrapper.inner(area).layout(&layout);

        let input_title = Paragraph::new("Type name here:").dark_gray().bold();
        let input = Paragraph::new(String::from(&self.input.input));

        let db_items = self.db.projects_list();
        let names: Vec<String> = db_items.iter().map(|item| item.name.clone()).collect();
        let projects_list_title = Paragraph::new("Existed projects:").dark_gray().bold();
        let projects_list = List::new(names).style(Color::White);

        frame.set_cursor_position(Position::new(
            input_area.x + u16::try_from(self.input.character_index).unwrap_or(0),
            input_area.y,
        ));

        frame.render_widget(wrapper, area);
        frame.render_widget(input_title, input_title_area);
        frame.render_widget(input, input_area);
        frame.render_widget(projects_list_title, projects_list_title_area);
        frame.render_widget(projects_list, projects_list_area);
    }

    fn draw_new_timer(&mut self, frame: &mut Frame, area: Rect) {
        let db_items = self.db.projects_list();
        let names: Vec<String> = db_items.iter().map(|item| item.name.clone()).collect();

        let wrapper = Block::new()
            .title_top(Line::from_iter([" New timer ".to_span()]).left_aligned())
            .borders(Borders::ALL)
            .border_style(Style::new().yellow());
        let layout = Layout::new(
            Direction::Vertical,
            [Constraint::Length(1), Constraint::Min(1)],
        );
        let [title_area, list_area] = layout.areas(wrapper.inner(area));

        let title = Paragraph::new("Select project:").bold().dark_gray();
        let list = List::new(names)
            .style(Color::White)
            .highlight_style(Modifier::REVERSED)
            .highlight_symbol("> ");

        frame.render_widget(wrapper, area);
        frame.render_widget(title, title_area);
        frame.render_stateful_widget(list, list_area, &mut self.project_list);
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
            Screen::NewProject => {
                let hint = Paragraph::new(Line::from_iter([
                    "<Enter>".bold(),
                    " Submit, ".to_span(),
                    "<ESC>".bold(),
                    " Cancel ".to_span(),
                ]));

                frame.render_widget(hint.block(wrapper), area);
            }
            Screen::NewTimer => {
                let hint = Paragraph::new(Line::from_iter([
                    "<Up/Down/Enter>".bold(),
                    " Select project, ".to_span(),
                    "<ESC>".bold(),
                    " Cancel ".to_span(),
                ]));

                frame.render_widget(hint.block(wrapper), area);
            }
        }
    }

    fn exit(&mut self) {
        self.should_exit = true;
    }

    fn to_screen(&mut self, screen: Screen) {
        match screen {
            Screen::NewTimer => {
                self.project_list.select(Some(0));
                self.screen = screen;
            }
            Screen::NewProject => {
                self.input = Input::new();
                self.screen = screen;
            }
            _ => self.screen = screen,
        }
    }

    fn from_screen(&mut self, screen: Screen) {
        match screen {
            Screen::NewProject => self.screen = Screen::Dashboard,
            Screen::NewTimer => self.screen = Screen::Dashboard,
            _ => {}
        }
    }

    fn submit_new_project(&mut self) {
        self.db.add_new_project(&self.input.input);
        self.input.input = String::new();
        self.input.character_index = 0;
    }
}
