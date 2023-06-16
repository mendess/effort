use std::{borrow::Cow, iter::repeat};

use time::Duration;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table,
        TableState,
    },
    Frame,
};

use crate::{
    app::{Activity, App, PopUp},
    selected_vec::SelectedVec,
    traits::EditingPopUp,
    util::{
        fmt_duration, is_weekend, size_slice,
        time_fmt::{DATE_FMT_FULL, TIME_FMT},
    },
};

impl Activity {
    fn to_row(&self) -> Row {
        let action = self.action.clone();
        let issue = self.issue.clone();
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

        Row::new([action, issue, start, end, time_spent])
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
    month_time: Duration,
    work_days: u16,
    workdays_worked: u32,
    weekend_days_worked: u32,
    holiday_days_worked: u32,
    days_off: u16,
    work_day_hours: f32,
    time_spent_on_issue: Option<Duration>,
}

fn render_table<B: Backend>(frame: &mut Frame<B>, rect: Rect, app: &App) -> Stats {
    let mut month_time = Duration::ZERO;
    let mut workdays_worked = 0;
    let mut weekend_worked_days = 0;
    let mut holiday_worked_days = 0;
    let selected_id = app.selected_id();
    let items: SelectedVec<_> = app
        .activities()
        .filter(|(_, acts)| !acts.is_empty())
        .flat_map(|(date, acts)| {
            if is_weekend(date) {
                weekend_worked_days += 1;
            } else if app.is_free_holiday(date) {
                holiday_worked_days += 1;
            } else{
                workdays_worked += 1;
            }
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
                month_time += total_time;
                (fmt_duration(total_time), some_none)
            };

            let separator = Row::new([
                Cell::from(date.format(DATE_FMT_FULL).unwrap()),
                Cell::from(String::new()),
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
                        Color::Red
                    } else if app.is_free_holiday(date) {
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
        .header(Row::new([
            "Action",
            "Issue",
            "start time",
            "end time",
            "time spent",
        ]))
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
        month_time,
        work_days: app.n_workdays_so_far(),
        workdays_worked,
        weekend_days_worked: weekend_worked_days,
        holiday_days_worked: holiday_worked_days,
        days_off: app.n_days_off_up_to_today(),
        work_day_hours: app.config.work_day_hours,
        time_spent_on_issue: app.selected_issue_total_time(),
    }
}

pub type InfoPopup = Option<Result<Cow<'static, str>, Cow<'static, str>>>;

pub fn ui<B: Backend>(frame: &mut Frame<B>, app: &mut App, info_popup: &InfoPopup) {
    let main = frame.size();
    match app.pop_up() {
        Some(PopUp::EditingPopUp(new)) => {
            render_table(frame, main, app);
            render_new_popup(frame, main, &**new);
        }
        Some(PopUp::DaysOff {
            selected,
            new_day_off,
        }) => {
            render_table(frame, main, app);
            render_days_off(frame, main, app, *selected, new_day_off);
        }
        Some(PopUp::Holidays {
            selected,
            new_holiday,
        }) => {
            render_table(frame, main, app);
            render_holidays(frame, main, app, *selected, new_holiday);
        }
        None => {
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
    pub(super) const NUM_WIDGETS: u16 = 5;
    pub(super) const WIDGET_HEIGHT: u16 = 3;
    pub(super) const TOTAL_HEIGHT: u16 = NUM_WIDGETS * WIDGET_HEIGHT;
}

fn render_new_popup<B: Backend>(frame: &mut Frame<B>, rect: Rect, new: &dyn EditingPopUp) {
    let bottom = bottom_of_rect(rect, new_act_sizes::TOTAL_HEIGHT);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            repeat(Constraint::Length(new_act_sizes::WIDGET_HEIGHT))
                .take(new_act_sizes::NUM_WIDGETS.into())
                .collect::<Vec<_>>(),
        )
        .split(bottom);
    frame.render_widget(Clear, bottom);
    new.render()
        .into_iter()
        .zip(&chunks)
        .for_each(|(a, c)| frame.render_widget(a, *c));
}

mod stats_size {
    pub(super) const TOTAL_HEIGHT: u16 = 9;
}

fn render_stats<B: Backend>(
    frame: &mut Frame<B>,
    rect: Rect,
    Stats {
        month_time,
        work_days,
        workdays_worked,
        weekend_days_worked,
        holiday_days_worked,
        days_off,
        work_day_hours,
        time_spent_on_issue,
    }: Stats,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .title("Stats");
    let legend_style = Style::default().add_modifier(Modifier::BOLD);
    let worked_days = work_days.saturating_sub(days_off);
    let table = Table::new(vec![
        Row::new([
            Span::styled("Total time this month: ", legend_style),
            Span::raw(fmt_duration(month_time)),
        ]),
        Row::new([
            Span::styled("Average time per work day: ", legend_style),
            Span::raw(fmt_duration(
                month_time
                    .checked_div(worked_days.into())
                    .unwrap_or_default(),
            )),
        ]),
        {
            let work_day_mins = (work_day_hours * 60.0) as u16;
            let otime = work_day_mins * worked_days;
            let overtime = month_time - Duration::minutes(otime.into());
            let (legend, dur, legend_style) = if overtime.is_negative() {
                (
                    "Undertime hours:",
                    overtime * -1,
                    legend_style.fg(Color::Red),
                )
            } else {
                ("Overtime hours:", overtime, legend_style.fg(Color::Green))
            };
            Row::new([
                Span::styled(legend, legend_style),
                Span::raw(fmt_duration(dur)),
            ])
        },
        Row::new([
            Span::styled("Total work days: ", legend_style),
            Span::raw(work_days.to_string()),
        ]),
        Row::new([
            Spans::from(vec![Span::styled("Total worked days: ", legend_style)]),
            Spans::from(vec![
                Span::raw(format!("{} (", workdays_worked + weekend_days_worked + holiday_days_worked)),
                Span::styled(
                    format!("{}", workdays_worked),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw("/"),
                Span::styled(
                    format!("{}", weekend_days_worked),
                    Style::default().fg(Color::Red),
                ),
                Span::raw("/"),
                Span::styled(
                    format!("{}", holiday_days_worked),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(")"),
            ]),
        ]),
        Row::new([
            Span::styled("Days off: ", legend_style),
            Span::raw(days_off.to_string()),
        ]),
        Row::new([
            Span::styled("Time Spent On Issue: ", legend_style),
            Span::raw(
                time_spent_on_issue
                    .map(fmt_duration)
                    .unwrap_or_else(|| "None".to_owned()),
            ),
        ]),
    ])
    .block(block)
    .widths(&[Constraint::Length(27), Constraint::Percentage(100)]);

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

mod new_date_sizes {
    pub(super) const NUM_WIDGETS: u16 = 1;
    pub(super) const WIDGET_HEIGHT: u16 = 3;
    pub(super) const TOTAL_HEIGHT: u16 = NUM_WIDGETS * WIDGET_HEIGHT;
}

fn render_days_off<B: Backend>(
    frame: &mut Frame<B>,
    rect: Rect,
    app: &App,
    selected: usize,
    new_day_off: &Option<String>,
) {
    let smaller = Rect {
        x: rect.x + 5,
        y: rect.y + 5,
        width: rect.width.saturating_sub(10),
        height: rect.height.saturating_sub(10),
    };
    frame.render_widget(Clear, smaller);
    let items = List::new(
        app.days_off()
            .map(|d| d.format(DATE_FMT_FULL).unwrap())
            .map(ListItem::new)
            .collect::<Vec<_>>(),
    )
    .block(Block::default().borders(Borders::ALL).title("days off"))
    .highlight_style(
        Style::default()
            // .bg(Color::LightGreen)
            // .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("> ");

    frame.render_stateful_widget(items, smaller, &mut {
        let mut state = ListState::default();
        state.select(Some(selected));
        state
    });

    if let Some(new_day_off) = new_day_off {
        let bottom = bottom_of_rect(smaller, new_date_sizes::TOTAL_HEIGHT);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                repeat(Constraint::Length(new_date_sizes::WIDGET_HEIGHT))
                    .take(new_act_sizes::NUM_WIDGETS.into())
                    .collect::<Vec<_>>(),
            )
            .split(bottom);
        frame.render_widget(Clear, bottom);
        [Paragraph::new(new_day_off.clone())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("date"))]
        .into_iter()
        .zip(&chunks)
        .for_each(|(a, c)| frame.render_widget(a, *c));
    }
}

fn render_holidays<B: Backend>(
    frame: &mut Frame<B>,
    rect: Rect,
    app: &App,
    selected: usize,
    new_holiday: &Option<String>,
) {
    let smaller = Rect {
        x: rect.x + 5,
        y: rect.y + 5,
        width: rect.width.saturating_sub(10),
        height: rect.height.saturating_sub(10),
    };
    frame.render_widget(Clear, smaller);
    let items = List::new(
        app.holidays()
            .map(|d| d.format(DATE_FMT_FULL).unwrap())
            .map(ListItem::new)
            .collect::<Vec<_>>(),
    )
    .block(Block::default().borders(Borders::ALL).title("holidays"))
    .highlight_style(
        Style::default()
            // .bg(Color::LightGreen)
            // .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("> ");

    frame.render_stateful_widget(items, smaller, &mut {
        let mut state = ListState::default();
        state.select(Some(selected));
        state
    });

    if let Some(new_holiday) = new_holiday {
        let bottom = bottom_of_rect(smaller, new_date_sizes::TOTAL_HEIGHT);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                repeat(Constraint::Length(new_date_sizes::WIDGET_HEIGHT))
                    .take(new_act_sizes::NUM_WIDGETS.into())
                    .collect::<Vec<_>>(),
            )
            .split(bottom);
        frame.render_widget(Clear, bottom);
        [Paragraph::new(new_holiday.clone())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("date"))]
        .into_iter()
        .zip(&chunks)
        .for_each(|(a, c)| frame.render_widget(a, *c));
    }
}
