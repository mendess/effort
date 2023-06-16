use std::{
    any::Any,
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
    marker::PhantomData,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use serde::{Deserialize, Serialize};
use time::{
    format_description::FormatItem, macros::format_description, Date, Month, OffsetDateTime, Time,
};

use tui::{
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::traits::EditingPopUp;
use crate::util::time_fmt::{DATE_FMT, TIME_FMT};

#[derive(Debug, Clone)]
pub struct ActivityBeingBuilt {
    id: ActivityId,
    last_time: Option<Time>,
    pub action: String,
    pub issue: String,
    pub start_time: String,
    pub end_time: String,
    pub day: String,
    pub selected: Selected,
    pub editing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Selected {
    Action,
    StartTime,
    EndTime,
    Day,
    Issue,
}

impl Selected {
    pub fn next(self) -> Self {
        match self {
            Self::Issue => Self::Action,
            Self::Action => Self::StartTime,
            Self::StartTime => Self::EndTime,
            Self::EndTime => Self::Day,
            Self::Day => Self::Issue,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Issue => Self::Day,
            Self::Action => Self::Issue,
            Self::StartTime => Self::Action,
            Self::EndTime => Self::StartTime,
            Self::Day => Self::EndTime,
        }
    }
}

impl ActivityBeingBuilt {
    pub fn new(last_time: Option<Time>) -> Self {
        Self {
            id: ActivityId::default(),
            last_time,
            action: String::new(),
            issue: String::new(),
            start_time: String::default(),
            end_time: String::new(),
            day: String::new(),
            selected: Selected::Issue,
            editing: true,
        }
    }
}

impl EditingPopUp for ActivityBeingBuilt {
    fn select_next(&mut self) {
        self.selected = self.selected.next();
    }

    fn select_prev(&mut self) {
        self.selected = self.selected.prev();
    }

    fn selected_buf(&mut self) -> &mut String {
        match self.selected {
            Selected::Action => &mut self.action,
            Selected::Issue => &mut self.issue,
            Selected::StartTime => &mut self.start_time,
            Selected::EndTime => &mut self.end_time,
            Selected::Day => &mut self.day,
        }
    }

    fn set_editing(&mut self, state: bool) {
        self.editing = state;
    }

    fn submit(&self, app: &mut App) -> Result<(), &'static str> {
        let to_submit: Activity = self.try_into()?;
        app.add_activity(to_submit);
        app.pop_up = None;
        let _ = app.save_to(&app.filename);
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn is_editing(&self) -> bool {
        self.editing
    }

    fn render(&self) -> Vec<tui::widgets::Paragraph<'_>> {
        let mkparagraph = |title, buf, action| {
            Paragraph::new(buf)
                .style(if action == self.selected {
                    let color = if self.editing {
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
        vec![
            mkparagraph("issue", self.issue.as_str(), Selected::Issue),
            mkparagraph("action", self.action.as_str(), Selected::Action),
            mkparagraph("start time", &self.start_time, Selected::StartTime),
            mkparagraph("end time", &self.end_time, Selected::EndTime),
            mkparagraph("day", &self.day, Selected::Day),
        ]
    }

    fn popup_type(&self) -> crate::app::PopUpType {
        crate::app::PopUpType::EditActivity
    }
}

impl From<(&Activity, Option<Time>)> for ActivityBeingBuilt {
    fn from((a, last_time): (&Activity, Option<Time>)) -> Self {
        Self {
            id: a.id,
            last_time,
            action: a.action.clone(),
            issue: a.issue.clone(),
            start_time: a.start_time.format(TIME_FMT).unwrap(),
            end_time: a
                .end_time
                .map(|t| t.format(TIME_FMT).unwrap())
                .unwrap_or_default(),
            day: a.day.format(DATE_FMT).unwrap(),
            selected: Selected::Action,
            editing: true,
        }
    }
}

impl TryFrom<&ActivityBeingBuilt> for Activity {
    type Error = &'static str;

    fn try_from(builder: &ActivityBeingBuilt) -> Result<Self, Self::Error> {
        if builder.action.is_empty() {
            return Err("action field is mandatory");
        }
        let start_time = parse_time(&builder.start_time, true, builder.last_time)?;
        Ok(Activity {
            id: builder.id,
            start_time,
            end_time: if builder.end_time.is_empty() {
                None
            } else {
                let end_time = parse_time(&builder.end_time, false, None)?;
                if end_time < start_time {
                    return Err("end time can't be before start time");
                } else {
                    Some(end_time)
                }
            },
            day: parse_day(&builder.day)?,
            action: builder.action.clone(),
            issue: builder.issue.clone(),
            _m: PhantomData,
        })
    }
}

impl TryFrom<&mut ActivityBeingBuilt> for Activity {
    type Error = &'static str;

    fn try_from(builder: &mut ActivityBeingBuilt) -> Result<Self, Self::Error> {
        Activity::try_from(&*builder)
    }
}

fn parse_time(s: &str, assume_now: bool, last_time: Option<Time>) -> Result<Time, &'static str> {
    let now = OffsetDateTime::now_local()
        .map(OffsetDateTime::time)
        .map_err(|_| "The system's UTC offset could not be determined")?;
    if s.eq_ignore_ascii_case("now") || (s.is_empty() && assume_now) {
        return Ok(Time::from_hms(now.hour(), now.minute(), 0).unwrap());
    } else if s.eq_ignore_ascii_case("last") {
        if let Some(last_time) = last_time {
            return Ok(last_time);
        } else {
            return Err("no previous time available");
        }
    }
    let (hour, minute) = s.split_once(':').unwrap_or((s, ""));
    let hour = hour
        .parse()
        .map_err(|_| "failed to parse time: invalid hour")?;
    let minute = if minute.is_empty() {
        if hour == now.hour() {
            now.minute()
        } else {
            return Err("can't use current minute because you are not inputing current hour");
        }
    } else {
        minute
            .parse()
            .map_err(|_| "failed to parse time: invalid minute")?
    };

    Time::from_hms(hour, minute, 0)
        .map_err(|_| "failed to parse time: hour or minute out of bounds")
}

pub fn parse_day(s: &str) -> Result<Date, &'static str> {
    let mut today = OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .date();
    let mut iter = s.split(&['/', '-'][..]);
    if let Some(day) = iter.next().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let day = day
            .trim()
            .parse()
            .map_err(|_| "failed to parse date: invalid day")?;
        today = Date::from_calendar_date(today.year(), today.month(), day)
            .map_err(|_| "failed to parse date: day out of bounds")?;
    }

    if let Some(month) = iter.next() {
        let month = match month
            .trim()
            .parse()
            .map_err(|_| "failed to parse date: invalid month number")?
        {
            1 => Month::January,
            2 => Month::February,
            3 => Month::March,
            4 => Month::April,
            5 => Month::May,
            6 => Month::June,
            7 => Month::July,
            8 => Month::August,
            9 => Month::September,
            10 => Month::October,
            11 => Month::November,
            12 => Month::December,
            _ => return Err("failed to parse date: month number out of bounds"),
        };
        today = Date::from_calendar_date(today.year(), month, today.day())
            .map_err(|_| "failed to parse date: invalid month")?;
    }
    if let Some(year) = iter.next() {
        let year = year
            .trim()
            .parse()
            .map_err(|_| "failed to parse date: invalid year")?;
        today = Date::from_calendar_date(year, today.month(), today.day())
            .map_err(|_| "failed to parse date: year out of bounds")?;
    }

    Ok(today)
}

static ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActivityId(usize);

impl Default for ActivityId {
    fn default() -> Self {
        Self(ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Activity {
    pub day: Date,
    pub start_time: Time,
    pub end_time: Option<Time>,
    pub action: String,
    pub issue: String,
    #[serde(skip_serializing, skip_deserializing, default)]
    pub id: ActivityId,
    #[serde(skip)]
    _m: PhantomData<()>, // prevent constructing this type outside this module
}

pub fn load_activities<P: AsRef<Path>>(path: P) -> io::Result<Vec<Activity>> {
    match File::open(&path) {
        Ok(f) => {
            let file = BufReader::new(f);
            Ok(csv::Reader::from_reader(file)
                .deserialize::<Activity>()
                .collect::<Result<Vec<_>, _>>()?)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(e),
    }
}

const FMT: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]");

pub fn load_days_off<P: AsRef<Path>>(path: P) -> io::Result<Vec<Date>> {
    load_list_dates(format!("{}-off", path.as_ref().display()))
}

pub fn load_holidays<P: AsRef<Path>>(path: P) -> io::Result<Vec<Date>> {
    load_list_dates(format!("{}-holidays", path.as_ref().display()))
}

pub fn load_list_dates<P: AsRef<Path>>(path: P) -> io::Result<Vec<Date>> {
    match File::open(path) {
        Ok(mut f) => {
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            let dates = s
                .split('\n')
                .filter(|l| !l.is_empty())
                .map(|l| {
                    Date::parse(l, FMT).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(dates)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(e),
    }
}

pub fn store_activities<'a, I, W>(writer: W, activities: I) -> io::Result<()>
where
    I: Iterator<Item = &'a Activity>,
    W: Write,
{
    let file = BufWriter::new(writer);
    let mut writer = csv::Writer::from_writer(file);
    for a in activities {
        writer.serialize(a)?;
    }
    Ok(())
}

pub fn store_list_dates<'a, I, W>(writer: W, days_off: I) -> io::Result<()>
where
    I: Iterator<Item = &'a Date>,
    W: Write,
{
    use time::error::Format;
    let mut writer = BufWriter::new(writer);
    for d in days_off {
        match d.format_into(&mut writer, FMT) {
            Ok(_) => {}
            Err(Format::StdIo(e)) => return Err(e),
            Err(e) => panic!("{}", e),
        }
        writeln!(writer)?;
    }
    Ok(())
}
