use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use crate::dashboard::utils::robot_human_ratio;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RHTableStatsByDate {
    pub langs: HashSet<String>,
    pub refact: i64,
    pub human: i64,
    pub total: i64,
    pub refact_impact: f32,
    pub completions: i64,
}

impl RHTableStatsByDate {
    pub fn new() -> Self {
        RHTableStatsByDate {
            langs: HashSet::new(),
            refact: 0,
            human: 0,
            total: 0,
            refact_impact: 0.0,
            completions: 0,
        }
    }
    pub fn update(&mut self, r: &RHData) {
        self.langs.insert(r.file_extension.clone());
        self.refact += r.robot_characters;
        self.human += r.human_characters;
        self.total += r.robot_characters + r.human_characters;
        self.completions += r.completions_cnt;
        self.refact_impact = robot_human_ratio(self.refact, self.human);
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RHData {
    pub id: i64,
    pub tenant_name: String,
    pub ts_reported: i64,
    pub ip: String,
    pub enduser_client_version: String,
    pub completions_cnt: i64,
    pub file_extension: String,
    pub human_characters: i64,
    pub model: String,
    pub robot_characters: i64,
    pub teletype: String,
    pub ts_start: i64,
    pub ts_end: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RHTableStatsByLang {
    pub lang: String,
    pub refact: i64,
    pub human: i64,
    pub total: i64,
    pub refact_impact: f32,
    pub completions: i64,
}

impl RHTableStatsByLang {
    pub fn new(lang: String) -> Self {
        RHTableStatsByLang {
            lang,
            refact: 0,
            human: 0,
            total: 0,
            refact_impact: 0.0,
            completions: 0,
        }
    }
    pub fn update(&mut self, r: &RHData) {
        self.refact += r.robot_characters;
        self.human += r.human_characters;
        self.total += r.robot_characters + r.human_characters;
        self.completions += r.completions_cnt;
        self.refact_impact = robot_human_ratio(self.refact, self.human);
    }
}
