mod activity;
mod history;
mod state;

use std::{
    cmp::Reverse,
    collections::BTreeMap,
    fs::File,
    io::{self, Cursor},
};

pub use activity::{load_activities, store_activities, Activity, ActivityBeingBuilt, Selected};
use history::{Action, History};
pub use state::ActivityVec;
use state::State;
use time::Date;

pub struct App {
    filename: String,
    selected: Option<(Date, usize)>,
    activities: State,
    new_activity: Option<ActivityBeingBuilt>,
    show_stats: bool,
    history: History,
}

impl App {
    pub fn new(filename: String, activities: Vec<Activity>) -> Self {
        Self {
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
            new_activity: None,
            show_stats: false,
            history: History::default(),
        }
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

    pub fn create_new_activity(&mut self) {
        self.new_activity = Some(Default::default());
    }

    pub fn editing(&self) -> bool {
        matches!(self.new_activity.as_ref().map(|a| a.editing), Some(true))
    }

    pub fn n_days(&self) -> usize {
        self.activities.len()
    }

    pub fn new_activity(&self) -> &Option<ActivityBeingBuilt> {
        &self.new_activity
    }

    pub fn new_activity_mut(&mut self) -> &mut Option<ActivityBeingBuilt> {
        &mut self.new_activity
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

    pub fn selected(&self) -> Option<(Date, usize)> {
        self.selected
    }

    pub fn undo(&mut self) {
        self.history.undo(&mut self.activities)
    }

    pub fn redo(&mut self) {
        self.history.redo(&mut self.activities)
    }

    pub fn save(&self) -> io::Result<()> {
        let acts = self.activities.iter().flat_map(|(_, acts)| acts.iter());
        File::create(&self.filename).and_then(|f| store_activities(f, acts))
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
        let act = match self
            .activities
            .get(&Reverse(date))
            .and_then(|a| a.get(index))
        {
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
        match self.activities.add(to_submit.clone()) {
            Some(prev) => self.history.frwd(Action::Edit { prev }),
            None => self.history.frwd(Action::AddActivity(to_submit)),
        }
        self.new_activity = None;
        Ok(())
    }

    /// Delete the currently selected activity
    pub fn delete_activity(&mut self) {
        let (date, index) = match self.selected {
            Some(s) => s,
            None => return,
        };
        if let Some(act) = self.activities.remove(date, index) {
            self.history.frwd(Action::DeleteActivity(act))
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        println!("Auto saving file");
        if let Err(e) = self.save() {
            eprintln!("Fatal error writting file '{}'!!", self.filename);
            eprintln!("{:?}", e);
            let mut s = Vec::new();
            let c = Cursor::new(&mut s);
            match store_activities(c, self.activities.iter().flat_map(|(_, acts)| acts.iter())) {
                Ok(_) => eprintln!("{}", String::from_utf8_lossy(&s)),
                Err(e) => {
                    eprintln!("Failed to serialize csv in memory: {:?}", e);
                    eprintln!("{:?}", self.activities);
                }
            };
        }
    }
}
