use std::ops::{Deref, DerefMut};

use crate::app::activity::{Activity, ActivityId};

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

    pub fn find_by_id(&mut self, id: ActivityId) -> Option<ActivityVecGuard<'_>> {
        self.v
            .iter()
            .position(|a| a.id == id)
            .map(|i| ActivityVecGuard { vec: self, i })
    }

    pub fn remove_by_id(&mut self, id: ActivityId) -> Option<Activity> {
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

pub struct ActivityVecGuard<'v> {
    vec: &'v mut ActivityVec,
    i: usize,
}

impl<'v> Deref for ActivityVecGuard<'v> {
    type Target = Activity;

    fn deref(&self) -> &Self::Target {
        unsafe { self.vec.v.get_unchecked(self.i) }
    }
}

impl<'v> DerefMut for ActivityVecGuard<'v> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.vec.v.get_unchecked_mut(self.i) }
    }
}

impl<'v> Drop for ActivityVecGuard<'v> {
    fn drop(&mut self) {
        self.vec.v.sort_unstable();
    }
}
