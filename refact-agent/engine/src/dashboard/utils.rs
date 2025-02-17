use chrono::{Datelike, DateTime, Utc};

pub fn robot_human_ratio(robot: i64, human: i64) -> f32 {
    if human == 0 {
        return 1.0;
    }
    if robot == 0 {
        return 0.0;
    }
    // in older versions of refact LSP negative values of human metric existed
    if robot + human == 0 {
        return 0.0;
    }
    return robot as f32 / (robot + human) as f32;
}

pub fn get_week_n(date: &DateTime<Utc>, from_year: i32) -> i32 {
    let week_num = date.iso_week().week() as i32;
    let mut total_weeks = 0;
    for year in from_year..date.year() {
        total_weeks += if chrono::naive::NaiveDate::from_ymd_opt(year, 12, 28).unwrap().iso_week().year() == year {
            53
        } else {
            52
        };
    }
    total_weeks + week_num
}
