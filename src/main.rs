use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use input::Input;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, HorizontalAlignment, Layout, Position, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, ToSpan},
    widgets::{Block, Gauge, List, ListState, Padding, Paragraph},
};

use crate::{
    db::{Db, DbProject},
    time_formating::TimeFormating,
};

mod db;
mod input;
mod time_formating;

const ACCENT_COLOR: u8 = 204;
const SHADOWED_COLOR: u8 = 244;
const GAUGE_COLOR: u8 = 066;
const RUNNING_TIMER_COLOR: u8 = 114;

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
    project: String,
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
            timer: String::from("-"),
            project: String::from("-"),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut last_tick = Instant::now();
        self.on_tick();

        loop {
            terminal.draw(|frame| self.draw(frame))?;

            let tick_rate = Duration::from_millis(1000);
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if !event::poll(timeout)? {
                self.on_tick();
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
        let project_count = self.db.projects_list().len() as u16;
        let empty_dashboard_height = if project_count > 0 { project_count } else { 3 };
        let dashboard_height = match self.screen {
            Screen::NewProject => project_count + 4,
            Screen::TimerManager => project_count + 1,
            Screen::Dashboard => empty_dashboard_height,
        };
        let layout = Layout::vertical(vec![
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(dashboard_height),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [
            title_area,
            title_spacer_area,
            main_area,
            main_spacer_area,
            timer_area,
            timer_spacer_area,
            hint_area,
        ] = frame.area().layout(&layout);
        let title = Line::from_iter([
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
            "! | ".to_span(),
            Span::from(env!("CARGO_PKG_VERSION")),
            " ".to_span(),
        ])
        .centered();
        let spacer = Block::new();

        frame.render_widget(title, title_area);
        frame.render_widget(spacer.clone(), title_spacer_area);
        frame.render_widget(spacer.clone(), main_spacer_area);
        frame.render_widget(spacer, timer_spacer_area);

        self.draw_timer(frame, timer_area);
        self.draw_hint(frame, hint_area);

        match self.screen {
            Screen::Dashboard => {
                self.draw_dashboard(frame, main_area);
            }
            Screen::TimerManager => {
                self.draw_timer_manager(frame, main_area);
            }
            Screen::NewProject => {
                self.draw_new_project(frame, main_area);
            }
        };
    }

    fn on_tick(&mut self) {
        let diff = TimeFormating::diff_from_now(self.db.current_timer());
        let diff_formatted = TimeFormating::from_seconds(diff as u64);

        self.timer = diff_formatted;
        self.project = self.current_timer_project().name;
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
        let is_empty = self.db.projects_list().len() == 0;

        if is_empty {
            let empty_block = Block::new()
                .padding(Padding::new(0, 0, 0, 1))
                .title("No projects")
                .title_alignment(HorizontalAlignment::Center);

            frame.render_widget(empty_block, area);
            ()
        }

        let summary = self.db.summary_by_project();
        let constraints = summary.iter().map(|_| Constraint::Length(1));
        let timers_layout = Layout::new(Direction::Vertical, constraints);
        let timers_count = summary.len();
        let timers_areas: Vec<Rect> = area.layout_vec(&timers_layout);
        let mut i = 0;
        let mut max = 1;

        while i < timers_count {
            let summary_item = summary.get(i);
            match summary_item {
                Some(item) => {
                    if item.1 > max {
                        max = item.1;
                    }
                }
                None => continue,
            }
            i = i + 1;
        }

        i = 0;

        while i < timers_count {
            let summary_item = summary.get(i);
            match summary_item {
                Some(item) => {
                    let formatted_time = TimeFormating::from_seconds(item.1 as u64);
                    let title = item.0.clone();
                    let title = format!("{title}, {formatted_time}");
                    let safe_count = if item.1 == 0 { 1 } else { item.1 };
                    let percent = 100. / (max as f64 / safe_count as f64);
                    let gauge = Gauge::default()
                        .style(Modifier::BOLD)
                        .gauge_style(Style::new().fg(Color::Indexed(GAUGE_COLOR)))
                        .label(title)
                        .percent(percent as u16);

                    frame.render_widget(gauge, timers_areas[i]);
                }
                None => continue,
            }

            i = i + 1;
        }
    }

    fn draw_new_project(&mut self, frame: &mut Frame, area: Rect) {
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
        ] = area.layout(&layout);

        let input_title = Paragraph::new("Project name:").dark_gray().bold();
        let input = Paragraph::new(String::from(&self.input.input))
            .bg(Color::DarkGray)
            .white();

        let db_items = self.db.projects_list();
        let names: Vec<String> = db_items.iter().map(|item| item.name.clone()).collect();
        let projects_list_title = Paragraph::new("Existed projects:").dark_gray().bold();
        let projects_list = List::new(names).white();

        frame.set_cursor_position(Position::new(
            input_area.x + u16::try_from(self.input.character_index).unwrap_or(0),
            input_area.y,
        ));

        frame.render_widget(input_title, input_title_area);
        frame.render_widget(input, input_area);
        frame.render_widget(projects_list_title, projects_list_title_area);
        frame.render_widget(projects_list, projects_list_area);
    }

    fn draw_timer_manager(&mut self, frame: &mut Frame, area: Rect) {
        let db_items = self.db.projects_list();
        let names: Vec<String> = db_items.iter().map(|item| item.name.clone()).collect();

        let layout = Layout::new(
            Direction::Vertical,
            [Constraint::Length(1), Constraint::Min(1)],
        );
        let [title_area, list_area] = layout.areas(area);

        let title = Paragraph::new("Select project:").fg(Color::Indexed(SHADOWED_COLOR));
        let list = List::new(names)
            .highlight_style(Style::new().bg(Color::Indexed(ACCENT_COLOR)))
            .highlight_symbol("> ");

        frame.render_widget(title, title_area);
        frame.render_stateful_widget(list, list_area, &mut self.project_list);
    }

    fn draw_timer(&mut self, frame: &mut Frame, area: Rect) {
        let is_running = self.db.check_is_running_timer();
        let timer = &self.timer;
        let project = &self.project;
        let text = if is_running {
            Line::from_iter([project.to_span(), " | ".to_span(), timer.to_span().bold()])
        } else {
            Line::from_iter(["No running timer"])
        };

        let paragraph = Paragraph::new(text);
        let paragraph = if is_running {
            paragraph.bg(Color::Indexed(RUNNING_TIMER_COLOR))
        } else {
            paragraph.bg(Color::Indexed(ACCENT_COLOR))
        };

        frame.render_widget(paragraph, area);
    }

    fn draw_hint(&mut self, frame: &mut Frame, area: Rect) {
        let color = Color::Indexed(ACCENT_COLOR);
        let secondary_color = Color::Indexed(SHADOWED_COLOR);

        let hint = match self.screen {
            Screen::Dashboard => {
                let timer_hint = if self.db.check_is_running_timer() {
                    "top timer | "
                } else {
                    "tart timer | "
                };

                Paragraph::new(Line::from_iter([
                    "S".to_span().bold().black().bg(color),
                    Span::from(timer_hint).fg(secondary_color),
                    "New ".to_span().fg(secondary_color),
                    "P".to_span().bold().black().bg(color),
                    "roject | ".to_span().fg(secondary_color),
                    "R".to_span().bold().black().bg(color),
                    "eset DB | ".to_span().fg(secondary_color),
                    "Q".bold().black().bg(color),
                    "uit ".to_span().fg(secondary_color),
                ]))
            }
            Screen::NewProject => Paragraph::new(Line::from_iter([
                "<Enter>".bold().black().bg(color),
                " Submit | ".to_span().fg(secondary_color),
                "<ESC>".bold().black().bg(color),
                " Cancel ".to_span().fg(secondary_color),
            ])),
            Screen::TimerManager => Paragraph::new(Line::from_iter([
                "<Up/Down/Enter>".bold().black().bg(color),
                " Select project | ".to_span().fg(secondary_color),
                "<ESC>".bold().black().bg(color),
                " Cancel ".to_span().fg(secondary_color),
            ])),
        };

        frame.render_widget(hint.centered(), area);
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
        let empty_project = DbProject {
            id: -1,
            name: String::from(""),
        };
        let project = projects.get(selected).unwrap_or_else(|| &empty_project);

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
        self.timer = String::from("0s");
        self.project = self.current_timer_project().name;
        self.screen = Screen::Dashboard;
    }
}
