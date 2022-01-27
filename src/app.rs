mod activity;
mod history;
mod state;

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{self, Cursor},
    iter::successors,
    path::Path,
};

pub use activity::{load_activities, store_activities, Activity, ActivityBeingBuilt, Selected};
use history::{Action, History};
pub use state::ActivityVec;
use state::State;
use time::{macros::format_description, Date, OffsetDateTime};

use crate::util::{fmt_duration, is_weekend};

use self::activity::{load_days_off, parse_day, store_days_off, ActivityId};

pub enum PopUp {
    EditingActivity(ActivityBeingBuilt),
    DaysOff {
        selected: usize,
        new_day_off: Option<String>,
    },
}

pub struct App {
    filename: String,
    backup: String,
    selected: Option<(Date, usize)>,
    activities: State,
    days_off: BTreeSet<Reverse<Date>>,
    pop_up: Option<PopUp>,
    show_stats: bool,
    history: History,
    clipboard: Option<Activity>,
}

impl App {
    pub fn load(p: String) -> io::Result<Self> {
        let acts = load_activities(&p)?;
        let days_off = load_days_off(&p)?;
        Ok(Self::new(p, acts, days_off))
    }

    pub fn new(filename: String, activities: Vec<Activity>, days_off: Vec<Date>) -> Self {
        Self {
            backup: format!("{}.bck", filename),
            filename,
            selected: None,
            activities: activities
                .into_iter()
                .fold(
                    BTreeMap::<Reverse<Date>, ActivityVec>::new(),
                    |mut acc, a| {
                        acc.entry(Reverse(a.day)).or_default().add(a);
                        acc
                    },
                )
                .into(),
            days_off: days_off.into_iter().map(Reverse).collect(),
            pop_up: None,
            show_stats: false,
            history: History::default(),
            clipboard: None,
        }
    }

    pub fn n_workdays_so_far(&self) -> u32 {
        let mut iter = self.activities.iter();
        let last = match iter.next() {
            Some((d, _)) => d.0,
            None => return 0,
        };
        let mut first = match iter.next_back() {
            Some((d, _)) => d.0,
            None => return 1,
        };
        let mut counter = 0;
        while first <= last {
            if !is_weekend(&first) {
                counter += 1;
            }
            first = first.next_day().unwrap();
        }
        counter
    }

    pub fn next(&mut self) {
        fn from_new_kv((date, _): (&Reverse<Date>, &ActivityVec)) -> (Date, usize) {
            (date.0, 0)
        }
        if let Some((date, index)) = self.selected {
            let len = match self.activities.get(&Reverse(date)) {
                Some(acts) => acts.len(),
                None => {
                    self.selected = None;
                    return self.next();
                }
            };
            if index + 1 >= len {
                self.selected = self
                    .activities
                    .range(Reverse(date)..)
                    .nth(1)
                    .map(from_new_kv)
                    .or_else(|| self.activities.iter().next().map(from_new_kv));
            } else {
                self.selected = Some((date, index + 1));
            }
        } else {
            self.selected = self.activities.iter().next().map(from_new_kv);
        }
    }

    pub fn previous(&mut self) {
        fn from_new_kv((date, acts): (&Reverse<Date>, &ActivityVec)) -> (Date, usize) {
            (date.0, acts.len().saturating_sub(1))
        }

        self.selected = match self.selected {
            Some((date, 0)) => self
                .activities
                .range(..Reverse(date))
                .next_back()
                .map(from_new_kv)
                .or_else(|| self.activities.iter().next_back().map(from_new_kv)),
            Some((date, index)) => Some((date, index - 1)),
            None => self.activities.iter().next_back().map(from_new_kv),
        }
    }

    pub fn select_first(&mut self) {
        self.selected = self.activities.iter().next().map(|(d, _)| (d.0, 0));
    }

    pub fn select_last(&mut self) {
        self.selected = self
            .activities
            .iter()
            .next_back()
            .map(|(d, acts)| (d.0, acts.len() - 1))
    }

    pub fn selected_id(&self) -> Option<ActivityId> {
        let (date, index) = self.selected?;
        self.activities
            .get(&Reverse(date))
            .and_then(|v| v.get(index))
            .map(|a| a.id)
    }

    fn selected_activity(&self) -> Option<&Activity> {
        self.selected.and_then(|(date, index)| {
            self.activities
                .get(&Reverse(date))
                .and_then(|day| day.get(index))
        })
    }

    pub fn create_new_activity(&mut self) {
        let last_time = self.selected_activity().and_then(|a| a.end_time);
        self.pop_up = Some(PopUp::EditingActivity(ActivityBeingBuilt::new(last_time)));
    }

    pub fn editing(&self) -> bool {
        matches!(
            self.pop_up
                .as_ref()
                .map(|a| matches!(a, PopUp::EditingActivity(a) if a.editing)),
            Some(true)
        )
    }

    pub fn pop_up(&self) -> &Option<PopUp> {
        &self.pop_up
    }

    pub fn pop_up_mut(&mut self) -> &mut Option<PopUp> {
        &mut self.pop_up
    }

    pub fn toggle_stats(&mut self) {
        self.show_stats = !self.show_stats
    }

    pub fn show_stats(&self) -> bool {
        self.show_stats
    }

    pub fn activities(&self) -> impl DoubleEndedIterator<Item = (&Date, &[Activity])> {
        self.activities
            .iter()
            .map(|(date, acts)| (&date.0, acts.as_slice()))
    }

    #[allow(dead_code)]
    pub fn activities_filled(&self) -> impl Iterator<Item = (Date, &[Activity])> {
        static EMPTY: &[Activity] = &[];
        let most_recent = self.activities.iter().next().map(|(d, _)| d.0);
        successors(most_recent, |d| d.previous_day().filter(|d| d.day() != 20)).map(|d| match self
            .activities
            .get(&Reverse(d))
        {
            Some(acts) => (d, acts.as_slice()),
            None => (d, EMPTY),
        })
    }

    pub fn undo(&mut self) {
        self.history.undo(&mut self.activities)
    }

    pub fn redo(&mut self) {
        self.history.redo(&mut self.activities)
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&self.filename)
    }

    pub fn save_to<P: AsRef<Path>>(&self, p: P) -> io::Result<()> {
        let acts = self.activities.iter().flat_map(|(_, acts)| acts.iter());
        File::create(p.as_ref()).and_then(|f| store_activities(f, acts))?;
        if !self.days_off.is_empty() {
            File::create(format!("{}-off", p.as_ref().display()))
                .and_then(|f| store_days_off(f, self.days_off.iter().map(|d| &d.0)))
        } else {
            Ok(())
        }
    }

    pub fn export(&self) -> io::Result<()> {
        let mut acts = self
            .activities
            .iter()
            .flat_map(|(_, acts)| acts.iter())
            .map(|a| {
                if a.end_time.is_some() {
                    Ok(a)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("activity {:?} doesn't have an end time", a),
                    ))
                }
            })
            .collect::<io::Result<Vec<_>>>()?;
        acts.sort_unstable();
        let mut w = csv::Writer::from_path(format!("{}-export.csv", self.filename))?;
        static FMT: &[time::format_description::FormatItem<'static>] =
            format_description!("[month]-[day]-[year]");
        static TIME_FMT: &[time::format_description::FormatItem<'static>] =
            format_description!("[hour repr:12]:[minute] [period]");
        for a in acts.into_iter() {
            w.write_record([
                &a.day.format(FMT).expect("a correct format"),
                &a.action,
                &a.start_time.format(TIME_FMT).unwrap(),
                &a.end_time.unwrap().format(TIME_FMT).unwrap(),
                &fmt_duration(a.end_time.unwrap() - a.start_time),
            ])?;
        }
        Ok(())
    }

    pub fn cancel_edit(&mut self) {
        self.pop_up = None
    }

    pub fn yank_selected(&mut self) -> bool {
        let selected = self.selected_activity().cloned();
        if let Some(selected) = selected {
            self.clipboard = Some(selected);
            true
        } else {
            false
        }
    }

    pub fn show_days_off(&mut self) {
        self.pop_up = Some(PopUp::DaysOff {
            selected: 0,
            new_day_off: None,
        })
    }

    pub fn hide_days_off(&mut self) {
        if matches!(self.pop_up, Some(PopUp::DaysOff { .. })) {
            self.pop_up = None
        }
    }

    pub fn n_days_off(&self) -> usize {
        self.days_off.len()
    }

    pub fn n_days_off_up_to_today(&self) -> usize {
        let today = OffsetDateTime::now_local()
            .unwrap_or_else(|_| OffsetDateTime::now_utc())
            .date();
        self.days_off.iter().filter(|d| d.0 <= today).count()
    }

    pub fn submit_new_day_off(&mut self) -> Result<(), &'static str> {
        match &self.pop_up {
            Some(PopUp::DaysOff {
                new_day_off: Some(d),
                selected,
            }) => {
                let date = parse_day(d)?;
                let selected = *selected;
                self.add_day_off(date)?;
                self.pop_up = Some(PopUp::DaysOff {
                    selected,
                    new_day_off: None,
                });
                let _ = self.save_to(&self.backup);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn add_day_off(&mut self, date: Date) -> Result<(), &'static str> {
        if self.activities.contains_key(&Reverse(date)) {
            Err("you worked that day, can't take it off")
        } else if is_weekend(&date) {
            Err("can't take weekends off")
        } else {
            self.days_off.insert(Reverse(date));
            Ok(())
        }
    }

    pub fn days_off(&self) -> impl Iterator<Item = &Date> {
        self.days_off.iter().map(|d| &d.0)
    }
}

/// Actions that influence the history
impl App {
    /// Start editig the currently selected activity
    pub fn edit_activity(&mut self) {
        let (date, index) = match self.selected {
            Some(s) => s,
            None => return,
        };
        let (act, last) = match self
            .activities
            .get(&Reverse(date))
            .and_then(|a| a.get(index).map(|x| (x, a.get(index.saturating_sub(1)))))
        {
            Some(acts) => acts,
            None => return,
        };
        self.pop_up = Some(PopUp::EditingActivity(
            (act, last.and_then(|a| a.end_time)).into(),
        ));
        let _ = self.save_to(&self.backup);
    }

    /// Submit a currently being edited activity
    pub fn submit_activity(&mut self) -> Result<(), &'static str> {
        let to_submit: Activity = match &self.pop_up {
            Some(PopUp::EditingActivity(n)) => n.try_into()?,
            _ => return Ok(()),
        };
        self.add_activity(to_submit);
        self.pop_up = None;
        let _ = self.save_to(&self.backup);
        Ok(())
    }

    /// Delete the currently selected activity
    pub fn delete_activity(&mut self) {
        let (date, index) = match self.selected {
            Some(s) => s,
            None => return,
        };
        if let Some(act) = self.activities.remove(date, index) {
            self.clipboard = Some(act.clone());
            self.history.frwd(Action::DeleteActivity(act))
        }
        let _ = self.save_to(&self.backup);
    }

    pub fn paste(&mut self) -> Result<(), &'static str> {
        let mut to_paste = match &self.clipboard {
            Some(s) => s.clone(),
            None => return Err("clipboard is empty"),
        };
        to_paste.id = ActivityId::default();
        let length = to_paste.end_time.map(|e| e - to_paste.start_time);
        if let Some(selected) = self.selected_activity() {
            if let Some(end_time) = selected.end_time {
                to_paste.start_time = end_time;
            }
            to_paste.day = selected.day;
        }
        to_paste.end_time = length.map(|l| to_paste.start_time + l);
        self.add_activity(to_paste);
        let _ = self.save_to(&self.backup);
        Ok(())
    }

    fn add_activity(&mut self, a: Activity) {
        match self.activities.add(a.clone()) {
            Some(prev) => self.history.frwd(Action::Edit { prev }),
            None => self.history.frwd(Action::AddActivity(a)),
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        println!("Auto saving file");
        match self.save() {
            Err(e) => {
                eprintln!("Fatal error writing file '{}'!!", self.filename);
                eprintln!("{:?}", e);
                let mut s = Vec::new();
                let c = Cursor::new(&mut s);
                match store_activities(c, self.activities.iter().flat_map(|(_, acts)| acts.iter()))
                {
                    Ok(_) => eprintln!("{}", String::from_utf8_lossy(&s)),
                    Err(e) => {
                        eprintln!("Failed to serialize csv in memory: {:?}", e);
                        eprintln!("{:?}", self.activities);
                    }
                };
            }
            Ok(()) => {
                let _ = std::fs::remove_file(&self.backup);
            }
        }
    }
}
