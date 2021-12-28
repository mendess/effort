use std::{borrow::Cow, iter::repeat};

use time::Duration;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::{
    app::{Activity, ActivityBeingBuilt, App, Selected},
    selected_vec::SelectedVec,
    util::{
        fmt_duration, is_weekend, size_slice,
        time_fmt::{DATE_FMT_FULL, TIME_FMT},
    },
};

impl Activity {
    fn to_row(&self) -> Row {
        let action = self.action.clone();
        let start = self.start_time.format(TIME_FMT).unwrap();
        let end = self
            .end_time
            .map(|t| t.format(TIME_FMT).unwrap())
            .unwrap_or_else(|| "None".to_string());
        let time_spent = self
            .end_time
            .map(|t| t - self.start_time)
            .map(fmt_duration)
            .unwrap_or_else(String::new);

        Row::new([action, start, end, time_spent])
    }

    fn distance(&self, next: &Activity) -> Option<Row> {
        let bubble_start = match self.end_time {
            Some(s) => s,
            None => return None,
        };
        let bubble_end = next.start_time;
        let bubble_length = bubble_end - bubble_start;
        bubble_length.is_positive().then(|| {
            Row::new([
                Cell::from(String::new()),
                String::new().into(),
                String::new().into(),
                Cell::from(fmt_duration(bubble_length))
                    .style(Style::default().fg(Color::Black).bg(Color::DarkGray)),
            ])
        })
    }
}

struct Stats {
    total_month_time: Duration,
    work_days: u32,
    total_worked_days: u32,
}

fn render_table<B: Backend>(frame: &mut Frame<B>, rect: Rect, app: &App) -> Stats {
    let mut total_month_time = Duration::ZERO;
    let mut total_worked_days = 0;
    let selected_id = app.selected_id();
    let items: SelectedVec<_> = app
        .activities()
        .filter(|(_, acts)| !acts.is_empty())
        .flat_map(|(date, acts)| {
            total_worked_days += 1;
            let is_selected = |a: &Activity| Some(a.id) == selected_id;

            let (total_time, some_none) = {
                let mut some_none = false;
                let total_time = acts
                    .iter()
                    .filter_map(|a| {
                        if let Some(end_time) = a.end_time {
                            Some(end_time - a.start_time)
                        } else {
                            some_none = true;
                            None
                        }
                    })
                    .sum();
                total_month_time += total_time;
                (fmt_duration(total_time), some_none)
            };

            let separator = Row::new([
                Cell::from(date.format(DATE_FMT_FULL).unwrap()),
                Cell::from(String::new()),
                Cell::from(String::new()),
                Cell::from(total_time).style(Style::default().fg(if some_none {
                    Color::Red
                } else {
                    Color::Black
                })),
            ])
            .style(
                Style::default()
                    .bg(if is_weekend(date) {
                        Color::Yellow
                    } else {
                        Color::Blue
                    })
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

            let interspersed = acts.windows(2).map(size_slice).flat_map(move |[a, next]| {
                let mut iteration = vec![(a.to_row(), is_selected(a))];
                if let Some(bubble) = a.distance(next) {
                    iteration.push((bubble, false))
                }
                iteration
            });

            let last = acts.last().map(|a| (a.to_row(), is_selected(a)));

            std::iter::once((separator, false))
                .chain(interspersed)
                .chain(last)
        })
        .collect();

    let (items, index) = items.into_parts();

    let items = Table::new(items)
        .header(Row::new(["Action", "start time", "end time", "time spent"]))
        .block(Block::default())
        .highlight_style(
            Style::default()
                // .bg(Color::LightGreen)
                // .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ")
        .widths(&[
            Constraint::Percentage(100),
            Constraint::Length(13),
            Constraint::Length(11),
            Constraint::Length(13),
        ]);

    frame.render_stateful_widget(items, rect, &mut {
        let mut state = TableState::default();
        state.select(index);
        state
    });
    Stats {
        total_month_time,
        work_days: app.n_workdays_so_far(),
        total_worked_days,
    }
}

pub type InfoPopup = Option<Result<Cow<'static, str>, Cow<'static, str>>>;

pub fn ui<B: Backend>(frame: &mut Frame<B>, app: &mut App, info_popup: &InfoPopup) {
    let main = frame.size();
    if let Some(new) = app.new_activity() {
        render_table(frame, main, app);
        render_new_activity(frame, main, new);
    } else {
        let stats_height = app
            .show_stats()
            .then(|| frame.size().height.checked_sub(stats_size::TOTAL_HEIGHT))
            .flatten();
        match stats_height {
            Some(height) => {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(height),
                        Constraint::Length(stats_size::TOTAL_HEIGHT),
                    ])
                    .split(main);

                let stats = render_table(frame, layout[0], app);
                render_stats(frame, layout[1], stats);
            }
            _ => {
                render_table(frame, main, app);
            }
        }
    }

    if let Some(y) = main.height.checked_sub(3) {
        let info = Rect {
            y,
            height: 3,
            ..main
        };
        match info_popup {
            Some(Ok(m)) => render_info(frame, info, "info", m, Color::Green),
            Some(Err(error)) => render_info(frame, info, "error", error, Color::Red),
            None => {}
        }
    }
}

mod new_act_sizes {
    pub(super) const NUM_WIDGETS: u16 = 4;
    pub(super) const WIDGET_HEIGHT: u16 = 3;
    pub(super) const TOTAL_HEIGHT: u16 = NUM_WIDGETS * WIDGET_HEIGHT;
}

fn render_new_activity<B: Backend>(frame: &mut Frame<B>, rect: Rect, new: &ActivityBeingBuilt) {
    let bottom = bottom_of_rect(rect, new_act_sizes::TOTAL_HEIGHT);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            repeat(Constraint::Length(new_act_sizes::WIDGET_HEIGHT))
                .take(new_act_sizes::NUM_WIDGETS.into())
                .collect::<Vec<_>>(),
        )
        .split(bottom);
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

    frame.render_widget(Clear, bottom);
    [
        mkparagraph("action", new.action.as_str(), Selected::Action),
        mkparagraph("start time", &new.start_time, Selected::StartTime),
        mkparagraph("end time", &new.end_time, Selected::EndTime),
        mkparagraph("day", &new.day, Selected::Day),
    ]
    .into_iter()
    .zip(&chunks)
    .for_each(|(a, c)| frame.render_widget(a, *c));
}

mod stats_size {
    pub(super) const TOTAL_HEIGHT: u16 = 7;
}

fn render_stats<B: Backend>(
    frame: &mut Frame<B>,
    rect: Rect,
    Stats {
        total_month_time,
        work_days,
        total_worked_days,
    }: Stats,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .title("Stats");
    let legend_style = Style::default().add_modifier(Modifier::BOLD);
    let table = Table::new(vec![
        Row::new([
            Span::styled("Total time this month: ", legend_style),
            Span::raw(fmt_duration(total_month_time)),
        ]),
        Row::new([
            Span::styled("Average time per work day: ", legend_style),
            Span::raw(fmt_duration(if work_days == 0 {
                Duration::seconds(0)
            } else {
                total_month_time / work_days
            })),
        ]),
        {
            let overtime = total_month_time - Duration::hours((work_days * 8).into());
            let (legend, dur, legend_style) = if overtime.is_negative() {
                (
                    "Undertime hours",
                    overtime * -1,
                    legend_style.fg(Color::Red),
                )
            } else {
                ("Overtime hours", overtime, legend_style.fg(Color::Green))
            };
            Row::new([
                Span::styled(legend, legend_style),
                Span::raw(fmt_duration(dur)),
            ])
        },
        Row::new([
            Span::styled("Total work days (not counting weekends): ", legend_style),
            Span::raw(work_days.to_string()),
        ]),
        Row::new([
            Span::styled("Total worked days (counting weekends): ", legend_style),
            Span::raw(total_worked_days.to_string()),
        ]),
    ])
    .block(block)
    .widths(&[Constraint::Length(42), Constraint::Percentage(100)]);

    let bottom = bottom_of_rect(rect, stats_size::TOTAL_HEIGHT);
    frame.render_widget(Clear, bottom);
    frame.render_widget(table, bottom);
}

fn bottom_of_rect(r: Rect, height: u16) -> Rect {
    Rect {
        y: r.y + (r.height.saturating_sub(height)),
        height,
        ..r
    }
}

fn render_info<B: Backend>(frame: &mut Frame<B>, rect: Rect, title: &str, s: &str, color: Color) {
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(s).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color)),
        ),
        rect,
    );
}
