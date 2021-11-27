use std::ops::Deref;

use uuid::Uuid;

use crate::app::activity::Activity;

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
    pub fn add(&mut self, t: Activity) -> Option<Activity> {
        let prev = if let Some(i) = self.v.iter().position(|a| a.id == t.id) {
            Some(self.v.remove(i))
        } else {
            None
        };
        let i = match self.v.binary_search(&t) {
            Ok(i) => i,
            Err(i) => i,
        };
        self.v.insert(i, t);
        prev
    }

    pub fn remove(&mut self, index: usize) -> Option<Activity> {
        (self.v.len() > index).then(|| self.v.remove(index))
    }

    pub fn find_by_id(&mut self, id: Uuid) -> Option<&mut Activity> {
        self.v.iter_mut().find(|a| a.id == id)
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
