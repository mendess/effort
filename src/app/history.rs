use std::collections::BTreeMap;

use time::Date;

use super::{activity::Activity, activity_vec::ActivityVec};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum Action {
    DeleteActivity(Activity),
    Edit { prev: Activity },
    AddActivity(Activity),
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct History {
    past: Vec<Action>,
    future: Vec<Action>,
}

impl History {
    pub fn frwd(&mut self, a: Action) {
        self.past.push(a);
        self.future.clear();
    }

    pub fn redo(&mut self, state: &mut BTreeMap<Date, ActivityVec>) {
        if let Some(action) = self.future.pop() {
            match &action {
                Action::DeleteActivity(a) => {
                    state.get_mut(&a.day)
                        .expect("there should be a vec here since we are redoing a delete")
                        .remove_by_id(a.id);
                }
                Action::Edit { prev } => {
                    state.get_mut(&prev.day)
                        .expect("there should be a vec here since we are redoing an edit")
                        .replace(prev.clone());
                }
                Action::AddActivity(a) => {
                    state.entry(a.day).or_default().push(a.clone());
                }
            }
            self.past.push(action);
        }
    }

    pub fn undo(&mut self, state: &mut BTreeMap<Date, ActivityVec>) {
        if let Some(action) = self.past.pop() {
            match &action {
                Action::DeleteActivity(a) => {
                    state.entry(a.day).or_default().push(a.clone());
                }
                Action::Edit { prev } => {
                    state
                        .get_mut(&prev.day)
                        .expect("there should be a vec here since we are undoing an edit")
                        .replace(prev.clone());
                }
                Action::AddActivity(act) => {
                    state
                        .get_mut(&act.day)
                        .expect("there should be a vec here since we are undoing a add")
                        .remove_by_id(act.id);
                }
            }
            self.future.push(action);
        }
    }
}
