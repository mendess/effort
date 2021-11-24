use std::{
    fs::File,
    io::{self, BufReader, BufWriter},
    path::Path,
};

use serde::{Deserialize, Serialize};
use time::{
    format_description::FormatItem, macros::format_description, Date, Month, OffsetDateTime, Time,
};

pub const TIME_FMT: &[FormatItem<'static>] = format_description!("[hour]:[minute]");
pub const DATE_FMT: &[FormatItem<'static>] = format_description!("[day]/[month]/[year]");

#[derive(Debug, Clone)]
pub struct ActivityBeingBuilt {
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

    pub fn to_activity(&self) -> Result<Activity, &'static str> {
        if self.action.is_empty() {
            return Err("action field is mandatory");
        }
        if self.start_time.is_empty() {
            return Err("start time required");
        }
        Ok(Activity {
            start_time: parse_time(&self.start_time)?,
            end_time: if self.end_time.is_empty() {
                None
            } else {
                Some(parse_time(&self.end_time)?)
            },
            day: parse_day(&self.day)?,
            action: self.action.clone(),
        })
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Activity {
    pub day: Date,
    pub action: String,
    pub start_time: Time,
    pub end_time: Option<Time>,
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

pub fn store_activities<'a, I, P>(path: P, activities: I) -> io::Result<()>
where
    I: Iterator<Item = &'a Activity>,
    P: AsRef<Path>,
{
    match File::create(path) {
        Ok(f) => {
            let file = BufWriter::new(f);
            let mut writer = csv::Writer::from_writer(file);
            for a in activities {
                writer.serialize(a)?;
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}
