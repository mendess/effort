use std::iter::repeat;

use time::{Duration, Weekday};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::{
    app::{Activity, ActivityBeingBuilt, App, Selected},
    selected_vec::SelectedVec,
    util::{
        size_slice,
        time_fmt::{DATE_FMT_FULL, TIME_FMT},
    },
};

fn fmt_duration(d: Duration) -> String {
    format!(
        "{:02}:{:02}",
        d.whole_hours(),
        d.whole_minutes().saturating_sub(d.whole_hours() * 60)
    )
}

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
                String::new(),
                String::new(),
                String::new(),
                fmt_duration(bubble_length),
            ])
            .style(Style::default().fg(Color::DarkGray))
        })
    }
}

struct Stats {
    total_month_time: Duration,
    work_days: u32,
}

fn render_table<B: Backend>(frame: &mut Frame<B>, rect: Rect, app: &App) -> Stats {
    let mut total_month_time = Duration::ZERO;
    let mut work_days = 0;
    let selected_id = app.selected_id();
    let items: SelectedVec<_> = app
        .activities()
        .filter(|(_, acts)| !acts.is_empty())
        .flat_map(|(date, acts)| {
            let is_selected = |a: &Activity| Some(a.id) == selected_id;

            let total_time = acts
                .iter()
                .map(|a| Some(a.end_time? - a.start_time))
                .try_fold(Duration::ZERO, |acc, a| Some(acc + a?))
                .map(|d| {
                    total_month_time += d;
                    fmt_duration(d)
                })
                .unwrap_or_else(|| String::from("N/A"));

            let separator = Row::new([
                date.format(DATE_FMT_FULL).unwrap(),
                String::new(),
                String::new(),
                total_time,
            ])
            .style(
                Style::default()
                    .bg(
                        if matches!(date.weekday(), Weekday::Saturday | Weekday::Sunday) {
                            Color::Yellow
                        } else {
                            work_days += 1;
                            Color::Blue
                        },
                    )
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
        work_days,
    }
}

pub fn ui<B: Backend>(frame: &mut Frame<B>, app: &mut App, error: &mut Option<&'static str>) {
    if let Some(new) = app.new_activity() {
        render_table(frame, frame.size(), app);
        render_new_activity(frame, frame.size(), error, new);
    } else if app.show_stats() {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(frame.size().height - stats_size::TOTAL_HEIGHT),
                Constraint::Length(stats_size::TOTAL_HEIGHT),
            ])
            .split(frame.size());

        let stats = render_table(frame, layout[0], app);
        render_stats(frame, layout[1], stats);
    } else {
        render_table(frame, frame.size(), app);
    }
}

mod new_act_sizes {
    pub(super) const NUM_WIDGETS: u16 = 5;
    pub(super) const WIDGET_HEIGHT: u16 = 3;
    pub(super) const TOTAL_HEIGHT: u16 = NUM_WIDGETS * WIDGET_HEIGHT;
}

fn render_new_activity<B: Backend>(
    frame: &mut Frame<B>,
    rect: Rect,
    error: &mut Option<&'static str>,
    new: &ActivityBeingBuilt,
) {
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

    if let Some(msg) = error {
        frame.render_widget(
            Paragraph::new(*msg).block(
                Block::default()
                    .title("error")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            ),
            chunks[4],
        );
    }
}

mod stats_size {
    pub(super) const TOTAL_HEIGHT: u16 = 5;
}

fn render_stats<B: Backend>(
    frame: &mut Frame<B>,
    rect: Rect,
    Stats {
        total_month_time,
        work_days,
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
            Span::raw(fmt_duration(total_month_time / work_days)),
        ]),
        Row::new([
            Span::styled("Total work days (not counting weekends): ", legend_style),
            Span::raw(work_days.to_string()),
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
        y: r.y + (r.height - height),
        height,
        ..r
    }
}
