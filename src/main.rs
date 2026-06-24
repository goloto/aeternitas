use std::{
    fs, io,
    time::{Duration, Instant},
};

use crate::modules::{
    backuper::Backuper,
    db::{Db, DbProject, DbTimer},
    input::Input,
    rich_list_state::RichListState,
    time_formatting::TimeFormating,
    utils::Utils,
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, HorizontalAlignment, Layout, Position, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, ToSpan},
    widgets::{Block, Gauge, List, ListState, Padding, Paragraph},
};

mod modules;

const ACCENT_COLOR: Color = Color::Rgb(239, 100, 97);
const RUNNING_TIMER_COLOR: Color = Color::Rgb(189, 247, 183);
const SHADOWED_COLOR: Color = Color::DarkGray;
const GAUGE_COLOR: Color = Color::DarkGray;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}

pub struct App {
    should_exit: bool,
    screen: Screen,
    input: Input,
    db: Db,
    project_list: RichListState<DbProject>,
    backup_list: ListState,
    backuper: Backuper,
    timer: Option<DbTimer>,
}

enum Screen {
    Dashboard,
    NewTimer,
    NewProject,
    Restore,
}

impl App {
    fn new() -> Self {
        Self {
            should_exit: false,
            screen: Screen::Dashboard,
            input: Input::new(),
            db: Db::new(),
            project_list: RichListState::default(),
            backup_list: ListState::default().with_selected(Some(0)),
            backuper: Backuper::new(),
            timer: None,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let app_dir = Utils::get_app_dir();
        fs::create_dir_all(&app_dir).expect("Could not create working directory");

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
        let main_area_height = match self.screen {
            Screen::NewProject => project_count + 4,
            Screen::NewTimer => project_count + 1,
            Screen::Dashboard => empty_dashboard_height,
            Screen::Restore => self.backuper.count as u16 + 1,
        };
        let layout = Layout::vertical(vec![
            // title
            Constraint::Length(1),
            // empty line
            Constraint::Length(1),
            // main area
            Constraint::Length(main_area_height),
            // empty line
            Constraint::Length(1),
            // timer
            Constraint::Length(1),
            // empty line
            Constraint::Length(1),
            // hint
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
            Screen::NewTimer => {
                self.draw_new_timer(frame, main_area);
            }
            Screen::NewProject => {
                self.draw_new_project(frame, main_area);
            }
            Screen::Restore => {
                self.draw_restore(frame, main_area);
            }
        };
    }

    fn on_tick(&mut self) {
        let timer = self.db.current_timer();

        match timer {
            Some(timer) => self.timer = Some(timer),
            None => self.timer = None,
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match self.screen {
                Screen::Dashboard => match key_event.code {
                    KeyCode::Char('s') => self.manage_timer(),
                    KeyCode::Char('p') => self.to_screen(Screen::NewProject),
                    KeyCode::Char('r') => self.to_screen(Screen::Restore),
                    KeyCode::Char('q') => self.exit(),
                    KeyCode::Char('b') => self.db.backup(),
                    _ => {}
                },
                Screen::NewTimer => match key_event.code {
                    KeyCode::Esc => self.from_screen(Screen::NewTimer),
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
                Screen::Restore => match key_event.code {
                    KeyCode::Esc => self.from_screen(Screen::Restore),
                    KeyCode::Down => self.backup_list.select_next(),
                    KeyCode::Up => self.backup_list.select_previous(),
                    KeyCode::Enter => self.restore_db(),
                    _ => {}
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
                .title("No projects yet")
                .fg(SHADOWED_COLOR)
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
                        .gauge_style(Style::new().fg(GAUGE_COLOR))
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

        let input_title = Paragraph::new("Project name:").fg(SHADOWED_COLOR).bold();
        let input = Paragraph::new(String::from(&self.input.input))
            .bg(ACCENT_COLOR)
            .white();

        let projects_list_title = Paragraph::new("Existed projects:")
            .fg(SHADOWED_COLOR)
            .bold();
        let projects_list = List::new(self.db.project_names()).white();

        frame.set_cursor_position(Position::new(
            input_area.x + u16::try_from(self.input.character_index).unwrap_or(0),
            input_area.y,
        ));

        frame.render_widget(input_title, input_title_area);
        frame.render_widget(input, input_area);
        frame.render_widget(projects_list_title, projects_list_title_area);
        frame.render_widget(projects_list, projects_list_area);
    }

    fn draw_new_timer(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::new(
            Direction::Vertical,
            [Constraint::Length(1), Constraint::Min(1)],
        );
        let [title_area, list_area] = layout.areas(area);

        let title = Paragraph::new("Select project:").fg(SHADOWED_COLOR).bold();
        let project_names: Vec<String> = self
            .project_list
            .rich_state
            .iter()
            .map(|project| project.name.clone())
            .collect();
        let list = List::new(project_names)
            .highlight_style(Style::new().bg(ACCENT_COLOR))
            .highlight_symbol("> ");

        frame.render_widget(title, title_area);
        frame.render_stateful_widget(list, list_area, &mut self.project_list.state);
    }

    fn draw_timer(&mut self, frame: &mut Frame, area: Rect) {
        let is_running = self.db.is_timer_running();
        let diff_formatted: String;
        let text = match &self.timer {
            Some(timer) => {
                let diff = TimeFormating::diff_from_now(timer.started_at);
                diff_formatted = TimeFormating::from_seconds(diff as u64);

                Line::from_iter([
                    timer.project_name.to_span(),
                    " | ".to_span(),
                    diff_formatted.to_span().bold(),
                ])
            }
            None => Line::from_iter(["No running timer"]),
        };

        let paragraph = Paragraph::new(text).centered();
        let paragraph = if is_running {
            paragraph.bg(RUNNING_TIMER_COLOR).fg(Color::Black)
        } else {
            paragraph.bg(ACCENT_COLOR)
        };

        frame.render_widget(paragraph, area);
    }

    fn draw_hint(&mut self, frame: &mut Frame, area: Rect) {
        fn get_initial<'a>(str: &'a str) -> Span<'a> {
            Span::raw(str).bold().fg(ACCENT_COLOR)
        }
        fn get_common<'a>(str: &'a str) -> Span<'a> {
            Span::raw(str).fg(SHADOWED_COLOR)
        }

        let hint = match self.screen {
            Screen::Dashboard => {
                let timer_hint = if self.db.is_timer_running() {
                    "top timer | "
                } else {
                    "tart timer | "
                };

                Paragraph::new(Line::from_iter([
                    get_initial("S"),
                    get_common(timer_hint),
                    get_common("New "),
                    get_initial("P"),
                    get_common("roject | "),
                    get_initial("B"),
                    get_common("ackup DB | "),
                    get_initial("R"),
                    get_common("estore DB | "),
                    get_initial("Q"),
                    get_common("uit "),
                ]))
            }
            Screen::NewProject => Paragraph::new(Line::from_iter([
                get_initial("<Enter>"),
                get_common(" Submit | "),
                get_initial("<ESC>"),
                get_common(" Cancel "),
            ])),
            Screen::NewTimer => Paragraph::new(Line::from_iter([
                get_initial("<Up/Down/Enter>"),
                get_common(" Select project | "),
                get_initial("<ESC>"),
                get_common(" Cancel "),
            ])),
            Screen::Restore => Paragraph::new(Line::from_iter([
                get_initial("<Up/Down/Enter>"),
                get_common(" Select backup | "),
                get_initial("<ESC>"),
                get_common(" Cancel "),
            ])),
        };

        frame.render_widget(hint.centered(), area);
    }

    fn draw_restore(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::new(
            Direction::Vertical,
            [Constraint::Length(1), Constraint::Min(1)],
        );
        let [backup_list_title_area, backup_list_area] = area.layout(&layout);

        let title = Paragraph::new("Select backup:").fg(SHADOWED_COLOR).bold();
        let list = List::new(self.backuper.list.clone())
            .highlight_style(Style::new().bg(ACCENT_COLOR))
            .highlight_symbol("> ");

        frame.render_widget(title, backup_list_title_area);
        frame.render_stateful_widget(list, backup_list_area, &mut self.backup_list);
    }

    fn submit_new_project(&mut self) {
        self.db.add_new_project(&self.input.input);
    }

    fn manage_timer(&mut self) {
        if self.db.is_timer_running() {
            self.timer = None;
            self.db.stop_timer();
        } else {
            self.project_list = RichListState::new(self.db.projects_list());
            self.to_screen(Screen::NewTimer);
        }
    }

    fn start_timer(&mut self) {
        let timer = self.project_list.selected();

        match timer {
            Some(timer) => {
                self.db.start_timer(timer.id);
                self.timer = self.db.current_timer();
                self.screen = Screen::Dashboard;
            }
            None => {}
        }
    }

    fn restore_db(&mut self) {
        self.db.restore();
    }

    fn to_screen(&mut self, screen: Screen) {
        match screen {
            Screen::NewTimer => {
                self.project_list.state.select(Some(0));
                self.screen = screen;
            }
            Screen::NewProject => {
                self.input = Input::new();
                self.screen = screen;
            }
            Screen::Restore => {
                self.backuper.recalculate_list();
                self.backup_list.select(Some(0));
                self.screen = screen;
            }
            _ => self.screen = screen,
        }
    }

    fn from_screen(&mut self, screen: Screen) {
        match screen {
            Screen::NewProject => self.screen = Screen::Dashboard,
            Screen::NewTimer => self.screen = Screen::Dashboard,
            Screen::Restore => self.screen = Screen::Dashboard,
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.should_exit = true;
    }
}
