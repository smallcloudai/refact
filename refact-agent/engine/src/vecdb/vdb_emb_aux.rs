use chrono::{DateTime, NaiveDateTime, Utc};
use log::warn;
use std::hash::{DefaultHasher, Hasher};
use tokio_rusqlite::Connection;

struct TableInfo {
    name: String,
    creation_time: DateTime<Utc>,
}

pub fn create_emb_table_name(workspace_folders: &Vec<String>) -> String {
    fn _make_hash(msg: String) -> String {
        let mut hasher = DefaultHasher::new();
        hasher.write(msg.as_bytes());
        format!("{:x}", hasher.finish())
    }

    let now = Utc::now();
    let workspace_folder_list = workspace_folders.join(":");
    let hash = _make_hash(workspace_folder_list);
    format!("emb_{}_{}", hash, now.format("%Y%m%d_%H%M%S"))
}

fn parse_table_timestamp(table_name: &str) -> Option<DateTime<Utc>> {
    let parts: Vec<&str> = table_name.split('_').collect();
    if parts.len() >= 3 {
        let date = parts[parts.len() - 2];
        let time = parts[parts.len() - 1];

        if date.len() == 8 && time.len() == 6 {
            let datetime_str = format!(
                "{} {}",
                format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]),
                format!("{}:{}:{}", &time[0..2], &time[2..4], &time[4..6])
            );
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S")
            {
                return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }
    }
    None
}

pub async fn cleanup_old_emb_tables(conn: &Connection, days: usize, max_count: usize) -> Result<(), String> {
    async fn get_all_emb_tables(
        conn: &Connection,
    ) -> rusqlite::Result<Vec<TableInfo>, String> {
        Ok(conn.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'emb_%'",
            )?;
            let tables = stmt.query_map([], |row| {
                let name: String = row.get(0)?;
                Ok(name)
            })?;
            let mut table_infos = Vec::new();
            for table_result in tables {
                let table_name = table_result?;
                if let Some(creation_time) = parse_table_timestamp(&table_name) {
                    table_infos.push(TableInfo {
                        name: table_name,
                        creation_time,
                    });
                }
            }
            Ok(table_infos)
        })
        .await
        .map_err(|e| e.to_string())?)
    }

    let mut tables = get_all_emb_tables(conn).await?;
    tables.sort_by_key(|t| t.creation_time);
    let cutoff = Utc::now() - chrono::Duration::days(days as i64);
    if !tables.is_empty() {
        conn.call(move |conn| {
            for table in tables.iter().take(tables.len().saturating_sub(max_count)) {
                warn!(
                    "dropping emb table: {} (created at {})",
                    table.name, table.creation_time
                );
                conn.execute(&format!("DROP TABLE {}", table.name), [])?;
            }       
            for table in tables.iter().skip(tables.len().saturating_sub(max_count)) {
                if table.creation_time < cutoff {
                    warn!(
                        "dropping emb table: {} (created at {})",
                        table.name, table.creation_time
                    );
                    conn.execute(&format!("DROP TABLE {}", table.name), [])?;
                }
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
