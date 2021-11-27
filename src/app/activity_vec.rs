use std::{mem::replace, ops::Deref};

use uuid::Uuid;

use super::activity::Activity;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Default)]
pub struct ActivityVec {
    v: Vec<Activity>,
}

impl Deref for ActivityVec {
    type Target = Vec<Activity>;

    fn deref(&self) -> &Self::Target {
        &self.v
    }
}

impl ActivityVec {
    pub fn push(&mut self, t: Activity) -> Option<Activity> {
        let prev = if let Some(i) = self.v.iter().position(|a| a.id == t.id) {
            Some(self.v.remove(i))
        } else {
            None
        };
        self.v.push(t);
        self.v.sort();
        prev
    }

    pub fn remove(&mut self, index: usize) -> Activity {
        self.v.remove(index)
    }

    pub fn replace(&mut self, new: Activity) -> Activity {
        replace(self.v.iter_mut().find(|old| old.id == new.id).unwrap(), new)
    }

    pub fn remove_by_id(&mut self, id: Uuid) -> Option<Activity> {
        self.v
            .iter()
            .position(|a| a.id == id)
            .map(|i| self.v.remove(i))
    }
}

impl From<ActivityVec> for Vec<Activity> {
    fn from(s: ActivityVec) -> Vec<Activity> {
        s.v
    }
}
