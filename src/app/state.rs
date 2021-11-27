mod activity_vec;

use std::{collections::BTreeMap, ops::Deref};

use time::Date;
use uuid::Uuid;

use super::activity::Activity;
pub use activity_vec::ActivityVec;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State(BTreeMap<Date, ActivityVec>);

impl State {
    pub fn remove(&mut self, date: Date, index: usize) -> Option<Activity> {
        if let Some(acts) = self.0.get_mut(&date) {
            let act = acts.remove(index);
            if acts.is_empty() {
                self.0.remove(&date);
            }
            act
        } else {
            None
        }
    }

    pub fn remove_by_id(&mut self, date: Date, id: Uuid) -> Option<Activity> {
        if let Some(acts) = self.0.get_mut(&date) {
            let act = acts.remove_by_id(id);
            if acts.is_empty() {
                self.0.remove(&date);
            }
            act
        } else {
            None
        }
    }

    pub fn find_by_id(&mut self, date: Date, id: Uuid) -> Option<&mut Activity> {
        self.0.get_mut(&date).and_then(|acts| acts.find_by_id(id))
    }

    pub fn add(&mut self, a: Activity) -> Option<Activity> {
        self.0.entry(a.day).or_default().add(a)
    }
}

impl Deref for State {
    type Target = BTreeMap<Date, ActivityVec>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BTreeMap<Date, ActivityVec>> for State {
    fn from(m: BTreeMap<Date, ActivityVec>) -> Self {
        Self(m)
    }
}
