mod activity_vec;

use std::{cmp::Reverse, collections::BTreeMap, ops::Deref};

use time::Date;
use uuid::Uuid;

use self::activity_vec::ActivityVecGuard;

use super::activity::Activity;
pub use activity_vec::ActivityVec;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State(BTreeMap<Reverse<Date>, ActivityVec>);

impl State {
    pub fn remove(&mut self, date: Date, index: usize) -> Option<Activity> {
        if let Some(acts) = self.0.get_mut(&Reverse(date)) {
            let act = acts.remove(index);
            if acts.is_empty() {
                self.0.remove(&Reverse(date));
            }
            act
        } else {
            None
        }
    }

    pub fn remove_by_id(&mut self, date: Date, id: Uuid) -> Option<Activity> {
        if let Some(acts) = self.0.get_mut(&Reverse(date)) {
            let act = acts.remove_by_id(id);
            if acts.is_empty() {
                self.0.remove(&Reverse(date));
            }
            act
        } else {
            None
        }
    }

    pub fn find_by_id(&mut self, date: Date, id: Uuid) -> Option<ActivityVecGuard<'_>> {
        self.0
            .get_mut(&Reverse(date))
            .and_then(|acts| acts.find_by_id(id))
    }

    pub fn add(&mut self, a: Activity) -> Option<Activity> {
        self.0.entry(Reverse(a.day)).or_default().add(a)
    }
}

impl Deref for State {
    type Target = BTreeMap<Reverse<Date>, ActivityVec>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BTreeMap<Reverse<Date>, ActivityVec>> for State {
    fn from(m: BTreeMap<Reverse<Date>, ActivityVec>) -> Self {
        Self(m)
    }
}
