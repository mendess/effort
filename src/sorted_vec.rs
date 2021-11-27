use std::ops::Deref;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct SortedVec<T> {
    v: Vec<T>,
}

impl<T> Default for SortedVec<T> {
    fn default() -> Self {
        Self {
            v: Default::default(),
        }
    }
}

impl<T> Deref for SortedVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.v
    }
}

impl<T: Ord> SortedVec<T> {
    pub fn push(&mut self, t: T) {
        self.v.push(t);
        self.v.sort();
    }

    pub fn remove(&mut self, index: usize) -> T {
        self.v.remove(index)
    }
}
