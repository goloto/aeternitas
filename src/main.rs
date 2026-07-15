use std::{
    fs, io,
    time::{Duration, Instant},
};

use crate::{
    modules::{
        animation::Animation,
        backuper::Backuper,
        db::{Db, DbProject, DbSummary, DbTimer},
        input::Input,
        rich_list_state::RichListState,
        time_formatting::TimeFormating,
        utils::Utils,
    },
    resources::{
        color_palette::{ACCENT_COLOR, BOLD_TEXT_COLOR, REGULAR_TEXT_COLOR, RUNNING_TIMER_COLOR},
        timer_animation::create_timer_animation,
    },
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{
        Constraint, Direction,
        Flex::{self},
        HorizontalAlignment, Layout, Position, Rect,
        Spacing::Space,
    },
    style::{Color, Style, Stylize},
    text::{Line, Span, ToSpan},
    widgets::{Block, List, Padding, Paragraph},
};

mod modules;
mod resources;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}

pub struct App<'a> {
    should_exit: bool,
    screen: Screen,
    input: Input,
    db: Db,
    project_list: RichListState<DbProject>,
    backup_list: RichListState<String>,
    backuper: Backuper,
    timer: Option<DbTimer>,
    tick: bool,
    timer_animation: Animation<Line<'a>>,
    overall_summary: Option<Vec<DbSummary>>,
    current_week_summary: Option<Vec<DbSummary>>,
    last_week_summary: Option<Vec<DbSummary>>,
}

enum Screen {
    Dashboard,
    NewTimer,
    NewProject,
    Restore,
}

impl<'a> App<'a> {
    fn new() -> Self {
        Self {
            should_exit: false,
            screen: Screen::Dashboard,
            input: Input::new(),
            db: Db::new(),
            project_list: RichListState::default(),
            backup_list: RichListState::default(),
            backuper: Backuper::new(),
            timer: None,
            tick: false,
            timer_animation: Animation::new(create_timer_animation()),
            overall_summary: None,
            current_week_summary: None,
            last_week_summary: None,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let app_dir = Utils::get_app_dir();
        fs::create_dir_all(&app_dir).expect("Could not create working directory");
        let backup_dir = Utils::get_backups_dir();
        fs::create_dir_all(&backup_dir).expect("Could not create backup directory");

        let mut last_second_tick = Instant::now();
        let mut last_minute_tick = Instant::now();
        self.on_tick();
        self.update_statistics();

        loop {
            terminal.draw(|frame| self.draw(frame))?;

            let tick_rate = Duration::from_secs(1);
            let timeout = tick_rate.saturating_sub(last_second_tick.elapsed());

            if !event::poll(timeout)? {
                self.on_tick();
                last_second_tick = Instant::now();

                if last_minute_tick.elapsed() >= Duration::from_mins(1) {
                    self.update_statistics();
                    last_minute_tick = Instant::now();
                }

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
        let empty_dashboard_height = if project_count > 0 {
            // + title + empty line
            project_count + 2
        } else {
            // empty line + title + empty line
            3
        };
        let main_area_height = match self.screen {
            Screen::NewProject => project_count + 4,
            Screen::NewTimer => project_count + 1,
            Screen::Dashboard => empty_dashboard_height,
            Screen::Restore => self.backuper.count as u16 + 1,
        };
        let layout = Layout::vertical(vec![
            // title
            Constraint::Length(1),
            // main area
            Constraint::Length(main_area_height),
            // timer
            Constraint::Length(1),
            // hint
            Constraint::Length(1),
        ])
        .spacing(Space(1));
        let [title_area, main_area, timer_area, hint_area] = frame.area().layout(&layout);
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
        frame.render_widget(title, title_area);

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
        self.tick = !self.tick;
        let timer = self.db.current_timer();

        match timer {
            Some(timer) => self.timer = Some(timer),
            None => self.timer = None,
        }

        if self.db.is_timer_running() {
            self.timer_animation.next();
        }
    }

    fn update_statistics(&mut self) {
        self.overall_summary = Some(self.db.summary_overall());
        self.current_week_summary = Some(self.db.summary_current_week());
        self.last_week_summary = Some(self.db.summary_last_week());
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

    fn draw_dashboard(&self, frame: &mut Frame, area: Rect) {
        let is_empty = self.db.projects_list().len() == 0;

        if is_empty {
            let empty_block = Block::new()
                .padding(Padding::new(0, 0, 0, 1))
                .title("No finished timers yet")
                .fg(REGULAR_TEXT_COLOR)
                .title_alignment(HorizontalAlignment::Center);

            frame.render_widget(empty_block, area);
            ()
        }

        let horizontal_layout = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ],
        )
        .spacing(Space(3));
        let vertical_layout = Layout::new(
            Direction::Vertical,
            [Constraint::Length(1), Constraint::Min(1)],
        )
        .spacing(Space(1));
        let [overall_area, last_week_area, current_week_area] = horizontal_layout.areas(area);
        let [overall_title_area, overall_area] = vertical_layout.areas(overall_area);
        let [last_week_title_area, last_week_area] = vertical_layout.areas(last_week_area);
        let [current_week_title_area, current_week_area] = vertical_layout.areas(current_week_area);

        frame.render_widget(Paragraph::new("Overall").centered(), overall_title_area);
        frame.render_widget(Paragraph::new("Last week").centered(), last_week_title_area);
        frame.render_widget(
            Paragraph::new("Current week").centered(),
            current_week_title_area,
        );

        let overall_summary = match &self.overall_summary {
            Some(summary) => summary,
            None => panic!("Haven't find any overall summary"),
        };
        let last_week_summary = match &self.last_week_summary {
            Some(summary) => summary,
            None => panic!("Haven't find any previous week summary"),
        };
        let current_week_summary = match &self.current_week_summary {
            Some(summary) => summary,
            None => panic!("Haven't find any curremt week summary"),
        };

        self.draw_summary(overall_summary, overall_area, frame);
        self.draw_summary(last_week_summary, last_week_area, frame);
        self.draw_summary(current_week_summary, current_week_area, frame);
    }

    fn draw_summary(&self, summary: &Vec<DbSummary>, area: Rect, frame: &mut Frame) {
        let mut sorted_summary = summary.clone();
        sorted_summary.sort_by_key(|item| -item.count);

        let constraints: Vec<Constraint> = sorted_summary
            .iter()
            .map(|_| Constraint::Length(1))
            .collect();
        let timers_layout = Layout::new(Direction::Vertical, constraints);
        let timers_count = sorted_summary.len();
        let timers_areas: Vec<Rect> = area.layout_vec(&timers_layout);
        let mut i = 0;
        let mut max = 1;

        while i < timers_count {
            let summary_item = sorted_summary.get(i);
            match summary_item {
                Some(item) => {
                    if item.count > max {
                        max = item.count;
                    }
                }
                None => continue,
            }
            i = i + 1;
        }

        i = 0;

        while i < timers_count {
            let summary_item = sorted_summary.get(i);
            match summary_item {
                Some(item) => {
                    let timer_wrapper_layout = Layout::new(
                        Direction::Horizontal,
                        [Constraint::Min(1), Constraint::Min(1)],
                    )
                    .flex(Flex::SpaceBetween);
                    let [project_name_area, time_area] =
                        timer_wrapper_layout.areas(timers_areas[i]);

                    let formatted_time = TimeFormating::from_seconds_short(item.count as u64);
                    let project_name = item.project_name.clone();

                    let symbol = if self.tick {
                        Span::from(">").fg(ACCENT_COLOR)
                    } else {
                        Span::from(">").fg(REGULAR_TEXT_COLOR)
                    };
                    let project_paragraph = Span::from(project_name).fg(REGULAR_TEXT_COLOR);
                    let project_paragraph = match &self.timer {
                        Some(timer) => {
                            if timer.project_id == item.project_id {
                                Line::from_iter([symbol, " ".to_span(), project_paragraph])
                            } else {
                                Line::from_iter([project_paragraph])
                            }
                        }
                        None => Line::from_iter([project_paragraph]),
                    };

                    frame.render_widget(project_paragraph, project_name_area);
                    frame.render_widget(
                        Paragraph::new(formatted_time)
                            .bold()
                            .fg(BOLD_TEXT_COLOR)
                            .right_aligned(),
                        time_area,
                    );
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

        let input_title = Paragraph::new("Project name:").fg(BOLD_TEXT_COLOR).bold();
        let input = Paragraph::new(String::from(&self.input.input))
            .bg(ACCENT_COLOR)
            .white();

        let projects_list_title = Paragraph::new("Existed projects:")
            .fg(BOLD_TEXT_COLOR)
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

        let title = Paragraph::new("Select project:").fg(BOLD_TEXT_COLOR).bold();
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
                    diff_formatted.to_span().bold().fg(BOLD_TEXT_COLOR),
                ])
            }
            None => Line::from_iter(["No running timer"]),
        };

        let animation: Line = if is_running {
            self.timer_animation.current().clone()
        } else {
            let empty_block = " ".to_span().bg(ACCENT_COLOR);

            Line::from_iter([
                empty_block.clone(),
                empty_block.clone(),
                empty_block.clone(),
                empty_block.clone(),
                empty_block.clone(),
                empty_block.clone(),
                empty_block.clone(),
                empty_block.clone(),
            ])
        };
        let animation_length = animation.iter().len() as u16;
        let layout = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Length(animation_length),
                Constraint::Min(1),
                Constraint::Length(animation_length),
            ],
        );
        let [animation_area_1, timer_area, animation_area_2] = layout.areas(area);

        let timer = Paragraph::new(text).centered();
        let timer = if is_running {
            timer.bg(RUNNING_TIMER_COLOR).fg(Color::Black)
        } else {
            timer.bg(ACCENT_COLOR)
        };
        let animation: Line = if is_running {
            self.timer_animation.current().clone()
        } else {
            Line::from_iter([
                " ".to_span().bg(ACCENT_COLOR),
                " ".to_span().bg(ACCENT_COLOR),
                " ".to_span().bg(ACCENT_COLOR),
                " ".to_span().bg(ACCENT_COLOR),
                " ".to_span().bg(ACCENT_COLOR),
                " ".to_span().bg(ACCENT_COLOR),
                " ".to_span().bg(ACCENT_COLOR),
                " ".to_span().bg(ACCENT_COLOR),
            ])
        };

        frame.render_widget(&animation, animation_area_1);
        frame.render_widget(timer, timer_area);
        frame.render_widget(&animation, animation_area_2);
    }

    fn draw_hint(&mut self, frame: &mut Frame, area: Rect) {
        fn get_initial<'a>(str: &'a str) -> Span<'a> {
            Span::raw(str).bold().fg(ACCENT_COLOR)
        }
        fn get_common<'a>(str: &'a str) -> Span<'a> {
            Span::raw(str).fg(REGULAR_TEXT_COLOR)
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

        let title = Paragraph::new("Select backup:").fg(BOLD_TEXT_COLOR).bold();
        let list = List::new(self.backuper.list.clone())
            .highlight_style(Style::new().bg(ACCENT_COLOR))
            .highlight_symbol("> ");

        frame.render_widget(title, backup_list_title_area);
        frame.render_stateful_widget(list, backup_list_area, &mut self.backup_list.state);
    }

    fn submit_new_project(&mut self) {
        self.db.add_new_project(&self.input.input);
    }

    fn manage_timer(&mut self) {
        if self.db.is_timer_running() {
            self.timer = None;
            self.db.stop_timer();
            self.timer_animation.reset();
            self.update_statistics();
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
        let selected_backup = self.backup_list.selected();

        match selected_backup {
            Some(backup) => {
                self.db.restore(backup);
                self.screen = Screen::Dashboard;
                self.update_statistics();
                self.timer = self.db.current_timer();
            }
            None => {}
        }
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
                self.backup_list = RichListState::new(self.backuper.list.clone());
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
