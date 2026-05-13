use std::{
    io,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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

use crate::{
    db::{Db, DbProject},
    time_formating::TimeFormating,
};

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
    timer: String,
}

enum Screen {
    Dashboard,
    TimerManager,
    NewProject,
}

impl App {
    fn new() -> Self {
        Self {
            should_exit: false,
            screen: Screen::Dashboard,
            input: Input::new(),
            db: Db::new(),
            project_list: ListState::default().with_selected(Some(0)),
            timer: String::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|frame| self.draw(frame))?;

            let tick_rate = Duration::from_millis(1000);
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if !event::poll(timeout)? {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Could not calculate current time")
                    .as_secs() as i64;
                let diff = now - self.db.current_timer();
                let diff_formatted = TimeFormating::from_seconds(diff as u64);

                self.timer = diff_formatted;
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
        let layout = Layout::vertical(vec![
            Constraint::Min(3),
            Constraint::Length(4),
            Constraint::Length(3),
        ]);
        let [main_area, timer_area, hint_area] = frame.area().layout(&layout);

        match self.screen {
            Screen::Dashboard => {
                self.draw_dashboard(frame, main_area);
                self.draw_timer(frame, timer_area);
                self.draw_hint(frame, hint_area);
            }
            Screen::TimerManager => {
                self.draw_timer_manager(frame, main_area);
                self.draw_timer(frame, timer_area);
                self.draw_hint(frame, hint_area);
            }
            Screen::NewProject => {
                self.draw_new_project(frame, main_area);
                self.draw_timer(frame, timer_area);
                self.draw_hint(frame, hint_area);
            }
        };
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match self.screen {
                Screen::Dashboard => match key_event.code {
                    KeyCode::Char('s') => self.manage_timer(),
                    KeyCode::Char('p') => self.to_screen(Screen::NewProject),
                    KeyCode::Char('r') => self.db.reset(),
                    KeyCode::Char('q') => self.exit(),
                    _ => {}
                },
                Screen::TimerManager => match key_event.code {
                    KeyCode::Esc => self.from_screen(Screen::TimerManager),
                    KeyCode::Down => self.project_list.select_next(),
                    KeyCode::Up => self.project_list.select_previous(),
                    KeyCode::Enter => self.start_timer(),
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

    fn draw_timer_manager(&mut self, frame: &mut Frame, area: Rect) {
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

    fn draw_timer(&mut self, frame: &mut Frame, area: Rect) {
        let is_running = self.db.check_is_running_timer();

        let wrapper = Block::new().borders(Borders::ALL);
        let wrapper = if is_running {
            wrapper.border_style(Color::Green)
        } else {
            wrapper.border_style(Color::Red)
        };

        let columns_layout = Layout::new(
            Direction::Horizontal,
            [Constraint::Percentage(50), Constraint::Percentage(50)],
        );
        let rows_layout = Layout::new(
            Direction::Vertical,
            [Constraint::Length(1), Constraint::Length(1)],
        );
        let [timer_area, project_area] = columns_layout.areas(wrapper.inner(area));
        let [timer_title_area, timer_area] = rows_layout.areas(timer_area);
        let [project_title_area, project_area] = rows_layout.areas(project_area);

        frame.render_widget(wrapper, area);
        frame.render_widget(Paragraph::new("Timer:").dark_gray(), timer_title_area);
        frame.render_widget(Paragraph::new(self.timer.clone()), timer_area);
        frame.render_widget(Paragraph::new("Project:").dark_gray(), project_title_area);
        frame.render_widget(
            Paragraph::new(self.current_timer_project().name),
            project_area,
        );
    }

    fn draw_hint(&mut self, frame: &mut Frame, area: Rect) {
        let wrapper = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::new().gray());

        match self.screen {
            Screen::Dashboard => {
                let timer_hint = if self.db.check_is_running_timer() {
                    " Stop timer, "
                } else {
                    " Start timer, "
                };
                let hint = Paragraph::new(Line::from_iter([
                    "<S>".bold(),
                    timer_hint.to_span(),
                    "<P>".bold(),
                    " New project, ".to_span(),
                    "<R>".bold(),
                    " Reset DB, ".to_span(),
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
            Screen::TimerManager => {
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
            Screen::TimerManager => {
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
            Screen::TimerManager => self.screen = Screen::Dashboard,
            _ => {}
        }
    }

    fn current_timer_project(&self) -> DbProject {
        let selected = self
            .project_list
            .selected()
            .expect("Some error while submiting timer with selected project");
        let projects = self.db.projects_list();
        let project = projects
            .get(selected)
            .expect("Something bad happened to selected project in list");

        project.clone()
    }

    fn submit_new_project(&mut self) {
        self.db.add_new_project(&self.input.input);
        self.input.input = String::new();
        self.input.character_index = 0;
    }

    fn manage_timer(&mut self) {
        if self.db.check_is_running_timer() {
            self.db.stop_timer();
        } else {
            self.to_screen(Screen::TimerManager);
        }
    }

    fn start_timer(&mut self) {
        self.db.start_timer(self.current_timer_project().id);
        self.screen = Screen::Dashboard;
    }
}
