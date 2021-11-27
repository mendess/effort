mod app;
mod combo_buffer;
mod selected_vec;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::{
    env::args,
    io::{self, Stdout},
};
use time::{format_description::FormatItem, macros::format_description, Duration};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};

use crate::{
    app::{App, Selected},
    selected_vec::SelectedVec,
};

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn main() -> anyhow::Result<()> {
    let path = match args().nth(1) {
        Some(filename) => filename,
        None => {
            println!("Usage: {} [FILE]", args().next().unwrap());
            std::process::exit(1)
        }
    };
    let activities = app::load_activities(&path)?;
    let mut terminal = setup_terminal()?;
    let mut app = App::new(path, activities);
    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    let mut combo_buffer = combo_buffer::ComboBuffer::default();
    let mut error = None;
    loop {
        terminal.draw(|f| ui(f, app, &mut error))?;
        error = None;
        if let Event::Key(key) = event::read()? {
            if let KeyCode::Char('q') = key.code {
                if !app.editing() {
                    return Ok(());
                }
            }
            match app.new_activity() {
                Some(new) => {
                    if new.editing {
                        match key.code {
                            KeyCode::Char(c) => new.selected_buf().push(c),
                            KeyCode::Backspace => {
                                new.selected_buf().pop();
                            }
                            KeyCode::Tab => new.select_next(),
                            KeyCode::BackTab => new.select_prev(),
                            KeyCode::Esc => new.editing = false,
                            KeyCode::Enter => {
                                if let Err(msg) = app.submit_activity() {
                                    error = Some(msg)
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('i') => new.editing = true,
                            KeyCode::Char('k') => new.select_next(),
                            KeyCode::Char('j') => new.select_prev(),
                            KeyCode::Enter => {
                                if let Err(msg) = app.submit_activity() {
                                    error = Some(msg)
                                }
                            }
                            _ => {}
                        }
                    }
                }
                None => {
                    match key.code {
                        KeyCode::Char('k') => app.previous(),
                        KeyCode::Char('j') => app.next(),
                        KeyCode::Char('s') => app.toggle_stats(),
                        KeyCode::Char('o') => app.create_new_activity(),
                        KeyCode::Char('u') => app.undo(),
                        KeyCode::Char('e') => app.edit_activity(),
                        _ => {}
                    }
                    if let Some(combo) = combo_buffer.combo(key.code) {
                        match combo {
                            combo_buffer::Combo::Delete => {
                                app.delete_activity();
                            }
                        }
                    }
                }
            }
        }
    }
}

const TIME_FMT: &[FormatItem<'static>] = format_description!("[hour]:[minute]");

fn ui<B: Backend>(frame: &mut Frame<B>, app: &mut App, error: &mut Option<&'static str>) {
    let chunks = if app.new_activity().is_some() {
        Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(frame.size())
    } else if app.show_stats() {
        Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(frame.size())
    } else {
        Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .constraints([Constraint::Percentage(100), Constraint::Percentage(0)])
            .split(frame.size())
    };
    fn fmt_duration(d: Duration) -> String {
        format!(
            "{:02}:{:02}",
            d.whole_hours(),
            d.whole_minutes().saturating_sub(d.whole_hours() * 60)
        )
    }
    let mut total_month_time = Duration::ZERO;
    let mut total_days = 0;
    let items: SelectedVec<_> = app
        .activities()
        .flat_map(|(date, acts)| {
            let total_time = acts
                .iter()
                .try_fold(Duration::ZERO, |acc, a| {
                    Some(acc + (a.end_time? - a.start_time))
                })
                .map(|d| {
                    total_month_time += d;
                    d
                })
                .map(fmt_duration)
                .unwrap_or_else(|| String::from("N/A"));

            total_days += 1;

            let separator = Row::new([
                date.format(format_description!("[day]/[month]/[year]"))
                    .unwrap(),
                "".to_owned(),
                "".to_owned(),
                total_time,
            ])
            .style(Style::default().bg(Color::Blue).fg(Color::Black));

            std::iter::once((separator, false)).chain(acts.iter().enumerate().map(|(i, a)| {
                let action = a.action.clone();
                let start = a.start_time.format(TIME_FMT).unwrap();
                let end = a
                    .end_time
                    .map(|t| t.format(TIME_FMT).unwrap())
                    .unwrap_or_else(|| "None".to_string());
                let time_spent = a
                    .end_time
                    .map(|t| t - a.start_time)
                    .map(fmt_duration)
                    .unwrap_or_else(String::new);

                (
                    Row::new([action, start, end, time_spent]),
                    Some((*date, i)) == app.selected(),
                )
            }))
        })
        .collect();

    let (items, index) = items.into_parts();

    let items = Table::new(items)
        .header(Row::new(["Action", "start time", "end time", "time spent"]))
        .block(Block::default().borders(Borders::ALL).title("Activities"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ")
        .widths(&[
            Constraint::Percentage(100),
            Constraint::Length(13),
            Constraint::Length(11),
            Constraint::Length(13),
        ]);

    frame.render_stateful_widget(items, chunks[0], &mut {
        let mut state = TableState::default();
        state.select(index);
        state
    });

    if let Some(new) = &app.new_activity() {
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .constraints(if error.is_some() {
                vec![
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                ]
            } else {
                vec![
                    Constraint::Percentage(24),
                    Constraint::Percentage(24),
                    Constraint::Percentage(24),
                    Constraint::Percentage(24),
                ]
            })
            .split(chunks[1]);
        let mkparagraph = |title, buf, action| {
            Paragraph::new(buf)
                .style(if action == new.selected {
                    let color = if new.editing {
                        Color::Yellow
                    } else {
                        Color::Blue
                    };
                    Style::default().fg(color)
                } else {
                    Style::default()
                })
                .block(Block::default().borders(Borders::ALL).title(title))
        };

        let action = mkparagraph("action", new.action.as_str(), Selected::Action);
        let start_time = mkparagraph("start time", &new.start_time, Selected::StartTime);
        let end_time = mkparagraph("end time", &new.end_time, Selected::EndTime);
        let day = mkparagraph("day", &new.day, Selected::Day);

        frame.render_widget(action, chunks[0]);
        frame.render_widget(start_time, chunks[1]);
        frame.render_widget(end_time, chunks[2]);
        frame.render_widget(day, chunks[3]);
        if let Some(msg) = error {
            let e =
                mkparagraph("error", msg, Selected::Action).style(Style::default().fg(Color::Red));
            frame.render_widget(e, chunks[4]);
        }
    } else if app.show_stats() {
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);
        let total_time = Paragraph::new(fmt_duration(total_month_time)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Total time this month"),
        );
        let avg_per_day = Paragraph::new(fmt_duration(total_month_time / total_days)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Average time per day"),
        );
        frame.render_widget(total_time, chunks[0]);
        frame.render_widget(avg_per_day, chunks[1]);
    }
}
