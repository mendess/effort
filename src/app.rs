mod activity;
mod activity_vec;
mod history;

use std::{collections::BTreeMap, fs::File, io::Cursor};

pub use activity::{load_activities, store_activities, Selected};
use activity::{Activity, ActivityBeingBuilt};
use time::Date;

use activity_vec::ActivityVec;

use self::history::{Action, History};

pub struct App {
    filename: String,
    selected: Option<(Date, usize)>,
    activities: BTreeMap<Date, ActivityVec>,
    new_activity: Option<ActivityBeingBuilt>,
    show_stats: bool,
    history: History,
}

impl App {
    pub fn new(filename: String, activities: Vec<Activity>) -> Self {
        Self {
            filename,
            selected: None,
            activities: activities.into_iter().fold(BTreeMap::new(), |mut acc, a| {
                acc.entry(a.day).or_default().push(a);
                acc
            }),
            new_activity: None,
            show_stats: false,
            history: History::default(),
        }
    }

    pub fn next(&mut self) {
        fn from_new_kv((date, _): (&Date, &ActivityVec)) -> (Date, usize) {
            (*date, 0)
        }
        if let Some((date, index)) = self.selected {
            let len = match self.activities.get(&date) {
                Some(acts) => acts.len(),
                None => {
                    self.selected = None;
                    return self.next();
                }
            };
            if index + 1 >= len {
                self.selected = self
                    .activities
                    .range(date..)
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
        fn from_new_kv((date, acts): (&Date, &ActivityVec)) -> (Date, usize) {
            (*date, acts.len().saturating_sub(1))
        }

        self.selected = match self.selected {
            Some((date, 0)) => self
                .activities
                .range(..date)
                .next_back()
                .map(from_new_kv)
                .or_else(|| self.activities.iter().next_back().map(from_new_kv)),
            Some((date, index)) => Some((date, index - 1)),
            None => self.activities.iter().next_back().map(from_new_kv),
        }
    }

    pub fn create_new_activity(&mut self) {
        self.new_activity = Some(Default::default());
    }

    pub fn editing(&self) -> bool {
        matches!(self.new_activity.as_ref().map(|a| a.editing), Some(true))
    }

    pub fn new_activity(&mut self) -> &mut Option<ActivityBeingBuilt> {
        &mut self.new_activity
    }

    pub fn toggle_stats(&mut self) {
        self.show_stats = !self.show_stats
    }

    pub fn show_stats(&self) -> bool {
        self.show_stats
    }

    pub fn activities(&self) -> impl Iterator<Item = (&Date, &[Activity])> {
        self.activities
            .iter()
            .map(|(date, acts)| (date, acts.as_slice()))
    }

    pub fn selected(&self) -> Option<(Date, usize)> {
        self.selected
    }

    pub fn undo(&mut self) {
        self.history.undo(&mut self.activities)
    }

    pub fn redo(&mut self) {
        self.history.redo(&mut self.activities)
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
        let act = match self.activities.get(&date).and_then(|a| a.get(index)) {
            Some(act) => act,
            None => return,
        };
        self.new_activity = Some(act.into())
    }

    /// Submit a currently being edited activity
    pub fn submit_activity(&mut self) -> Result<(), &'static str> {
        let to_submit: Activity = match &self.new_activity {
            Some(n) => n.try_into()?,
            None => return Ok(()),
        };
        let acts = self.activities.entry(to_submit.day).or_default();
        match acts.remove_by_id(to_submit.id) {
            Some(prev) => self.history.frwd(Action::Edit { prev }),
            None => self.history.frwd(Action::AddActivity(to_submit.clone())),
        }
        acts.push(to_submit);
        self.new_activity = None;
        Ok(())
    }

    /// Delete the currently selected activity
    pub fn delete_activity(&mut self) {
        let (date, index) = match self.selected {
            Some(s) => s,
            None => return,
        };
        let acts = match self.activities.get_mut(&date) {
            Some(acts) => acts,
            None => return,
        };
        if acts.len() > index {
            let act = acts.remove(index);
            if acts.is_empty() {
                self.activities.remove(&date);
            }
            self.previous();
            self.history.frwd(Action::DeleteActivity(act));
        }
    }

}

impl Drop for App {
    fn drop(&mut self) {
        println!("Auto saving file");
        let acts = self.activities.iter().flat_map(|(_, acts)| acts.iter());
        let r = File::create(&self.filename).and_then(|f| store_activities(f, acts.clone()));
        if let Err(e) = r {
            eprintln!("Fatal error writting file '{}'!!", self.filename);
            eprintln!("{:?}", e);
            let mut s = Vec::new();
            let c = Cursor::new(&mut s);
            match store_activities(c, acts) {
                Ok(_) => eprintln!("{}", String::from_utf8_lossy(&s)),
                Err(e) => {
                    eprintln!("Failed to serialize csv in memory: {:?}", e);
                    eprintln!("{:?}", self.activities);
                }
            };
        }
    }
}
