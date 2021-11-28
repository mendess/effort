use std::mem::swap;

use super::{activity::Activity, state::State};

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

    pub fn redo(&mut self, state: &mut State) {
        if let Some(mut action) = self.future.pop() {
            match &mut action {
                Action::DeleteActivity(a) => {
                    state.remove_by_id(a.day, a.id);
                }
                Action::Edit { prev } => {
                    let mut old = state
                        .find_by_id(prev.day, prev.id)
                        .expect("there should be a vec here since we are undoing an edit");
                    swap(&mut *old, prev);
                }
                Action::AddActivity(a) => {
                    state.add(a.clone());
                }
            }
            self.past.push(action);
        }
    }

    pub fn undo(&mut self, state: &mut State) {
        if let Some(mut action) = self.past.pop() {
            match &mut action {
                Action::DeleteActivity(a) => {
                    state.add(a.clone());
                }
                Action::Edit { prev } => {
                    let mut old = state
                        .find_by_id(prev.day, prev.id)
                        .expect("there should be a vec here since we are undoing an edit");
                    swap(&mut *old, prev);
                }
                Action::AddActivity(act) => {
                    state.remove_by_id(act.day, act.id);
                }
            }
            self.future.push(action);
        }
    }
}
