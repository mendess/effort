use time::{Duration, Weekday};

pub mod time_fmt {
    use time::{format_description::FormatItem, macros::format_description};

    pub const TIME_FMT: &[FormatItem<'static>] = format_description!("[hour]:[minute]");
    pub const DATE_FMT: &[FormatItem<'static>] = format_description!("[day]/[month]/[year]");
    pub const DATE_FMT_FULL: &[FormatItem<'static>] =
        format_description!("[day]/[month]/[year] [weekday]");
}

pub fn size_slice<T, const N: usize>(s: &[T]) -> &[T; N] {
    assert!(s.len() == N);
    unsafe { &*(s.as_ptr() as *const [T; N]) }
}

pub fn is_weekend(date: &time::Date) -> bool {
    matches!(date.weekday(), Weekday::Saturday | Weekday::Sunday)
}

pub fn fmt_duration(d: Duration) -> String {
    format!(
        "{:02}:{:02}",
        d.whole_hours(),
        d.whole_minutes().saturating_sub(d.whole_hours() * 60)
    )
}
