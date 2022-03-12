use chrono::Datelike;

const WEEKDAYS: &[&str; 7] = &[
    "Montag",
    "Dienstag",
    "Mittwoch",
    "Donnerstag",
    "Freitag",
    "Samstag",
    "Sonntag",
];

pub fn current_week_day() -> String {
    let current_day = chrono::offset::Local::now().weekday();
    WEEKDAYS[current_day.num_days_from_monday() as usize].to_string()
}
