use std::collections::HashMap;
use chrono::{Datelike, DateTime};
use serde_json::{json, Value};
use crate::dashboard::structs::{RHData, RHTableStatsByDate, RHTableStatsByLang};
use crate::dashboard::utils::{get_week_n};


async fn table_stats_by_lang(records: &Vec<RHData>) -> Value {
    let mut lang2stats: HashMap<String, RHTableStatsByLang> = HashMap::new();

    for r in records.iter() {
        let lang = r.file_extension.clone();
        let stats = lang2stats.entry(lang.clone()).or_insert(RHTableStatsByLang::new(lang.clone()));
        stats.update(r);
    }

    let mut lang_stats_records: Vec<RHTableStatsByLang> = lang2stats.iter().map(|(_, v)| v.clone()).collect();
    lang_stats_records.sort_by(|a, b| b.total.cmp(&a.total));
    json!({
        "data": lang_stats_records,
        "columns": vec!["Language", "Refact", "Human", "Total (characters)", "Refact Impact", "Completions"],
        "title": "Refact's impact by language",
    })
}

async fn refact_impact_dates(
    context: &DashboardContext,
    records: &Vec<RHData>
) -> Value{
    let mut day2stats: HashMap<String, RHTableStatsByDate> = HashMap::new();
    let mut week_n2stats: HashMap<i32, RHTableStatsByDate> = HashMap::new();
    let mut week_str2stats: HashMap<String, RHTableStatsByDate> = HashMap::new();

    for r in records.iter() {
        let day = DateTime::from_timestamp(r.ts_end, 0).unwrap().format("%Y-%m-%d").to_string();

        let stats = day2stats.entry(day.clone()).or_insert(RHTableStatsByDate::new());
        stats.update(r);

        let week_n = context.date2week_n.get(&day);
        if week_n.is_none() {
            continue;
        }
        let week_n = week_n.unwrap();
        let stats_week_n = week_n2stats.entry(*week_n).or_insert(RHTableStatsByDate::new());
        stats_week_n.update(r);
    }

    for (k, v) in week_n2stats.iter() {
        let week_str = context.week_n2date.get(k);
        if week_str.is_none() {
            continue;
        }
        let week_str = week_str.unwrap();
        week_str2stats.insert(week_str.clone(), v.clone());
    }
    json!({
        "data": {
            "daily": day2stats,
            "weekly": week_str2stats,
        }
    })
}

struct DashboardContext {
    date2week_n: HashMap<String, i32>,
    week_n2date: HashMap<i32, String>,
}


async fn get_context(records: &Vec<RHData>) -> Result<DashboardContext, String> {
    if records.is_empty() {
        return Err("no records".to_string())
    }
    let from_year = DateTime::from_timestamp(records.get(0).unwrap().ts_end, 0).unwrap().year();
    let mut date2week_n: HashMap<String, i32> = HashMap::new();

    for r in records {
        let date = DateTime::from_timestamp(r.ts_end, 0).unwrap();
        if date2week_n.contains_key(&date.format("%Y-%m-%d").to_string()) {
            continue;
        }
        let week_n = get_week_n(&date, from_year);
        date2week_n.insert(date.format("%Y-%m-%d").to_string(), week_n);
    }

    let mut week_n2date: HashMap<i32, String> = HashMap::new();
    for (date, week_n) in date2week_n.iter() {
        week_n2date.insert(*week_n, date.to_string());
    }

    Ok(DashboardContext {
        date2week_n,
        week_n2date,
    })
}
pub async fn records2plots(records: &mut Vec<RHData>) -> Result<Value, String>{
    records.sort_by(|a, b| a.ts_end.cmp(&b.ts_end));

    let context = get_context(records).await?;

    let table_refact_impact_json = table_stats_by_lang(records).await;
    let refact_impact_dates_json = refact_impact_dates(&context, records).await;

    Ok(json!({
        "table_refact_impact": table_refact_impact_json,
        "refact_impact_dates": refact_impact_dates_json,
    }))
}