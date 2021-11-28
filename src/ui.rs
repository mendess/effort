use std::iter::repeat;

use time::Duration;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::{
    app::{Activity, ActivityBeingBuilt, App, Selected},
    selected_vec::SelectedVec,
    util::{
        size_slice,
        time_fmt::{DATE_FMT, TIME_FMT},
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

fn render_table<B: Backend>(frame: &mut Frame<B>, rect: Rect, app: &App) -> Duration {
    let mut total_month_time = Duration::ZERO;
    let items: SelectedVec<_> = app
        .activities()
        .flat_map(|(date, acts)| {
            let is_selected = |i| Some((*date, i)) == app.selected();

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
                date.format(DATE_FMT).unwrap(),
                String::new(),
                String::new(),
                total_time,
            ])
            .style(Style::default().bg(Color::Blue).fg(Color::Black));

            let interspersed =
                acts.windows(2)
                    .map(size_slice)
                    .enumerate()
                    .flat_map(move |(i, [a, next])| {
                        let mut iteration = vec![(a.to_row(), is_selected(i))];
                        if let Some(bubble) = a.distance(next) {
                            iteration.push((bubble, false))
                        }
                        iteration
                    });

            let last = acts
                .last()
                .map(|a| (a.to_row(), is_selected(acts.len() - 1)));

            std::iter::once((separator, false))
                .chain(interspersed)
                .chain(last)
        })
        .collect();

    let (items, index) = items.into_parts();

    let items = Table::new(items)
        .header(Row::new(["Action", "start time", "end time", "time spent"]))
        .block(Block::default().borders(Borders::ALL).title("Activities"))
        .highlight_style(
            Style::default()
                // .bg(Color::LightGreen)
                // .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ")
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
    total_month_time
}

pub fn ui<B: Backend>(frame: &mut Frame<B>, app: &mut App, error: &mut Option<&'static str>) {
    let layout = Layout::default().direction(Direction::Vertical);
    if let Some(new) = app.new_activity() {
        let chunks = layout
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(frame.size());

        render_table(frame, chunks[0], app);
        render_new_activity(frame, chunks[1], error, new);
    } else if app.show_stats() {
        let chunks = layout
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(frame.size());

        let total_month_time = render_table(frame, chunks[0], app);
        render_stats(frame, chunks[1], total_month_time, app.n_days() as _);
    } else {
        let chunks = layout
            .constraints([Constraint::Percentage(100)])
            .split(frame.size());

        render_table(frame, chunks[0], app);
    }
}

fn render_new_activity<B: Backend>(
    frame: &mut Frame<B>,
    rect: Rect,
    error: &mut Option<&'static str>,
    new: &ActivityBeingBuilt,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints({
            let len = 4 + error.is_some() as u16;
            repeat(Constraint::Percentage(100 / len))
                .take(len.into())
                .collect::<Vec<_>>()
        })
        .split(rect);
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
            mkparagraph("error", msg, Selected::Action).style(Style::default().fg(Color::Red)),
            chunks[4],
        );
    }
}

fn render_stats<B: Backend>(
    frame: &mut Frame<B>,
    rect: Rect,
    total_month_time: Duration,
    n_days: u32,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rect);
    let total_time = Paragraph::new(fmt_duration(total_month_time)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Total time this month"),
    );
    let avg_per_day = Paragraph::new(fmt_duration(total_month_time / n_days)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Average time per day"),
    );
    frame.render_widget(total_time, chunks[0]);
    frame.render_widget(avg_per_day, chunks[1]);
}
