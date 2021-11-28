use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};
use time::{Date, Month, OffsetDateTime, Time};
use uuid::Uuid;

use crate::util::time_fmt::{DATE_FMT, TIME_FMT};

#[derive(Debug, Clone)]
pub struct ActivityBeingBuilt {
    id: Uuid,
    pub action: String,
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
}

impl Selected {
    pub fn next(self) -> Self {
        match self {
            Self::Action => Self::StartTime,
            Self::StartTime => Self::EndTime,
            Self::EndTime => Self::Day,
            Self::Day => Self::Action,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Action => Self::Day,
            Self::StartTime => Self::Action,
            Self::EndTime => Self::StartTime,
            Self::Day => Self::EndTime,
        }
    }
}

impl Default for ActivityBeingBuilt {
    fn default() -> Self {
        Self {
            id: Uuid::default(),
            action: String::new(),
            start_time: String::default(),
            end_time: String::new(),
            day: String::new(),
            selected: Selected::Action,
            editing: true,
        }
    }
}

impl ActivityBeingBuilt {
    pub fn select_next(&mut self) {
        self.selected = self.selected.next();
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.prev();
    }

    pub fn selected_buf(&mut self) -> &mut String {
        match self.selected {
            Selected::Action => &mut self.action,
            Selected::StartTime => &mut self.start_time,
            Selected::EndTime => &mut self.end_time,
            Selected::Day => &mut self.day,
        }
    }
}

impl From<&Activity> for ActivityBeingBuilt {
    fn from(a: &Activity) -> Self {
        Self {
            id: a.id,
            action: a.action.clone(),
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
        if builder.start_time.is_empty() {
            return Err("start time required");
        }
        Ok(Activity {
            id: builder.id,
            start_time: parse_time(&builder.start_time)?,
            end_time: if builder.end_time.is_empty() {
                None
            } else {
                Some(parse_time(&builder.end_time)?)
            },
            day: parse_day(&builder.day)?,
            action: builder.action.clone(),
        })
    }
}

impl TryFrom<&mut ActivityBeingBuilt> for Activity {
    type Error = &'static str;

    fn try_from(builder: &mut ActivityBeingBuilt) -> Result<Self, Self::Error> {
        Activity::try_from(&*builder)
    }
}

fn parse_time(s: &str) -> Result<Time, &'static str> {
    let (hour, minute) = s
        .split_once(':')
        .ok_or("failed to parse time: expected ':'")?;
    let hour = hour
        .parse()
        .map_err(|_| "failed to parse time: invalid hour")?;
    let minute = minute
        .parse()
        .map_err(|_| "failed to parse time: invalid minute")?;

    Time::from_hms(hour, minute, 0)
        .map_err(|_| "failed to parse time: hour or minute out of bounds")
}

fn parse_day(s: &str) -> Result<Date, &'static str> {
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Activity {
    pub day: Date,
    pub start_time: Time,
    pub end_time: Option<Time>,
    pub action: String,
    #[serde(skip_serializing, skip_deserializing, default = "Uuid::new_v4")]
    pub id: Uuid,
}

pub fn load_activities<P: AsRef<Path>>(path: P) -> io::Result<Vec<Activity>> {
    match File::open(path) {
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
