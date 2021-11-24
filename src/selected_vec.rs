pub struct SelectedVec<T> {
    v: Vec<T>,
    index: Option<usize>,
}

impl<T> SelectedVec<T> {
    pub fn into_parts(self) -> (Vec<T>, Option<usize>) {
        (self.v, self.index)
    }
}

impl<T> FromIterator<(T, bool)> for SelectedVec<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (T, bool)>,
    {
        let iter = iter.into_iter();
        let mut vec = Vec::with_capacity(iter.size_hint().0);
        let mut i = None;
        for (index, (e, selected)) in iter.enumerate() {
            vec.push(e);
            if selected {
                i = Some(index);
            }
        }
        Self { v: vec, index: i }
    }
}
