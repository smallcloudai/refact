use std::os::raw::{c_int, c_void};
use std::sync::Arc;
use rusqlite::Connection;
use tokio::sync::Notify;
use tracing::info;

pub fn setup_triggers(conn: &Connection, table_name: &str, fields: Vec<&str>, id_field: &str) -> Result<(), String> {
    for method in ["INSERT", "UPDATE", "DELETE"] {
        let field_obj = if method == "DELETE" { "OLD" } else { "NEW" };
        let json_object_fields: String = fields
            .iter()
            .map(|field| format!("'{field}', {field_obj}.{field}"))
            .collect::<Vec<String>>()
            .join(",\n");
        let sql = format!(
            "CREATE TRIGGER IF NOT EXISTS pubsub_events_on_insert
             AFTER {method} ON {table_name}
             BEGIN
                 INSERT INTO pubsub_events (pubevent_action, pubevent_channel, pubevent_obj_id, pubevent_obj_json)
                 VALUES ('{method}', '{table_name}', '{field_obj}.{id_field}', json_object(
                     {json_object_fields}
                 ));
             END;",
            table_name = table_name,
            json_object_fields = json_object_fields
        );
        conn.execute(&sql, []).map_err(|e| e.to_string())?;
    }
    Ok(())
}

extern "C" fn pubsub_trigger_hook(
    user_data: *mut c_void,
    action: c_int,
    db_name: *const i8,
    table_name: *const i8,
    _: i64,
) {
    let notify = unsafe { &*(user_data as *const Notify) };
    let db_name = unsafe { std::ffi::CStr::from_ptr(db_name).to_str().unwrap_or("unknown") };
    let table_name = unsafe { std::ffi::CStr::from_ptr(table_name).to_str().unwrap_or("unknown") };
    let operation = match action {
        18 => "INSERT",
        9 => "DELETE",
        23 => "UPDATE",
        _ => "UNKNOWN",
    };
    if db_name != "main" && table_name != "pubsub_events" {
        return;
    }
    info!("pubsub {} action triggered", operation);
    notify.notify_waiters();
}


pub fn create_tables_202412(conn: &Connection, sleeping_point: Arc<Notify>, reset_memory: bool) -> Result<(), String> {
    if reset_memory {
        conn.execute("DROP TABLE IF EXISTS pubsub_events", []).map_err(|e| e.to_string())?;
        conn.execute("DROP TABLE IF EXISTS chores", []).map_err(|e| e.to_string())?;
        conn.execute("DROP TABLE IF EXISTS chore_events", []).map_err(|e| e.to_string())?;
        conn.execute("DROP TABLE IF EXISTS cthreads", []).map_err(|e| e.to_string())?;
        conn.execute("DROP TABLE IF EXISTS cmessages", []).map_err(|e| e.to_string())?;
        conn.execute("DROP TABLE IF EXISTS memories", []).map_err(|e| e.to_string())?;
    }
    conn.execute(
    "CREATE TABLE IF NOT EXISTS pubsub_events (
            pubevent_id INTEGER PRIMARY KEY AUTOINCREMENT,
            pubevent_channel TEXT NOT NULL,
            pubevent_action TEXT NOT NULL,
            pubevent_obj_id TEXT NOT NULL,                  -- useful for extra filtering
            pubevent_obj_json TEXT NOT NULL,
            pubevent_ts TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
    "CREATE TRIGGER IF NOT EXISTS pubsub_events_delete_old
         AFTER INSERT ON pubsub_events
         BEGIN
             DELETE FROM pubsub_events WHERE pubevent_ts <= datetime('now', '-15 minutes');
         END;",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
    "CREATE TABLE IF NOT EXISTS chores (
            chore_id TEXT PRIMARY KEY,
            chore_title TEXT NOT NULL,
            chore_spontaneous_work_enable BOOLEAN NOT NULL,
            chore_created_ts REAL NOT NULL,
            chore_archived_ts REAL NOT NULL
        )",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
    "CREATE TABLE IF NOT EXISTS chore_events (
            chore_event_id TEXT PRIMARY KEY,
            chore_event_belongs_to_chore_id TEXT NOT NULL,
            chore_event_summary TEXT NOT NULL,
            chore_event_ts REAL NOT NULL,
            chore_event_link TEXT NOT NULL,
            chore_event_cthread_id TEXT,                -- optional, can be NULL
            FOREIGN KEY (chore_event_belongs_to_chore_id)
                REFERENCES chores(chore_id)
                ON DELETE CASCADE
        )",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
    "CREATE TABLE IF NOT EXISTS cthreads (
            cthread_id TEXT PRIMARY KEY,
            cthread_belongs_to_chore_event_id TEXT DEFAULT NULL,
            cthread_title TEXT NOT NULL,
            cthread_toolset TEXT NOT NULL,
            cthread_model TEXT NOT NULL,
            cthread_temperature REAL NOT NULL,
            cthread_max_new_tokens INT NOT NULL DEFAULT 2048,
            cthread_n INT NOT NULL DEFAULT 1,
            cthread_error TEXT NOT NULL,
            cthread_anything_new BOOLEAN NOT NULL,
            cthread_created_ts REAL NOT NULL,
            cthread_updated_ts REAL NOT NULL,
            cthread_archived_ts REAL NOT NULL,
            cthread_locked_by TEXT NOT NULL,           -- for autonomous work to start, cthread is locked first, ts more than an hour old means the lock is outdated
            cthread_locked_ts REAL NOT NULL,
            FOREIGN KEY (cthread_belongs_to_chore_event_id)
                REFERENCES chore_events(chore_event_id)
                ON DELETE CASCADE                       -- means cthread will be deleted together with chore_event, even though chore_event_cthread_id is optional
        )",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
    "CREATE TABLE IF NOT EXISTS cmessages (
            cmessage_belongs_to_cthread_id TEXT NOT NULL,
            cmessage_alt INT NOT NULL,
            cmessage_num INT NOT NULL,
            cmessage_prev_alt INT NOT NULL,
            cmessage_usage_model TEXT NOT NULL,
            cmessage_usage_prompt INT NOT NULL,
            cmessage_usage_completion INT NOT NULL,
            cmessage_json TEXT NOT NULL,
            PRIMARY KEY (cmessage_belongs_to_cthread_id, cmessage_alt, cmessage_num),
            FOREIGN KEY (cmessage_belongs_to_cthread_id)
                REFERENCES cthreads(cthread_id)
                ON DELETE CASCADE
        )",
        [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
    "CREATE TABLE IF NOT EXISTS memories (
            memid TEXT PRIMARY KEY,
            m_type TEXT NOT NULL,
            m_goal TEXT NOT NULL,
            m_project TEXT NOT NULL,
            m_payload TEXT NOT NULL,
            m_origin TEXT NOT NULL,
            mstat_correct REAL NOT NULL DEFAULT 0,
            mstat_relevant REAL NOT NULL DEFAULT 0,
            mstat_times_used INTEGER NOT NULL DEFAULT 0
        )",
        [],
    ).map_err(|e| e.to_string())?;

    // Embeddings
    conn.execute("DROP TABLE IF EXISTS embeddings", []).map_err(|e| e.to_string())?;
    conn.execute(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS embeddings using vec0(
              embedding float[1536] distance_metric=cosine,
              +memid text
            );"),
                 [],
    ).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS embeddings_delete_old
             AFTER DELETE ON memories
             BEGIN
                 DELETE FROM embeddings WHERE memid = OLD.memid;
             END;",
        [],
    ).map_err(|e| e.to_string())?;

    setup_triggers(&conn, "memories", vec![
        "memid", "m_type", "m_goal", "m_project", "m_payload", "m_origin",
        "mstat_correct", "mstat_relevant", "mstat_times_used"
    ], "memid")?;
    setup_triggers(&conn, "cthreads", vec![
        "cthread_id", "cthread_belongs_to_chore_event_id", "cthread_title",
        "cthread_toolset", "cthread_model", "cthread_temperature",
        "cthread_max_new_tokens", "cthread_n", "cthread_error",
        "cthread_anything_new", "cthread_created_ts", "cthread_updated_ts",
        "cthread_archived_ts", "cthread_locked_by", "cthread_locked_ts"
    ], "cthread_id")?;
    setup_triggers(&conn, "cmessages", vec![
        "cmessage_belongs_to_cthread_id", "cmessage_alt", "cmessage_num",
        "cmessage_prev_alt", "cmessage_usage_model", "cmessage_usage_prompt",
        "cmessage_usage_completion", "cmessage_json"
    ], "cmessage_belongs_to_cthread_id")?;
    setup_triggers(&conn, "chore_events", vec![
        "chore_event_id", "chore_event_belongs_to_chore_id", "chore_event_summary",
        "chore_event_ts", "chore_event_link", "chore_event_cthread_id"
    ], "chore_event_id")?;
    unsafe {
        libsqlite3_sys::sqlite3_update_hook(
            conn.handle(),
            Some(pubsub_trigger_hook),
            Arc::into_raw(sleeping_point.clone()) as *mut c_void,
        );
    }

    // Useful to speed up SELECT .. JOIN
    // conn.execute("CREATE INDEX IF NOT EXISTS idx_chore_event_belongs_to_chore_id ON chore_events (chore_event_belongs_to_chore_id)", []).map_err(|e| e.to_string())?;
    // conn.execute("CREATE INDEX IF NOT EXISTS idx_cthread_belongs_to_chore_event_id ON cthreads (cthread_belongs_to_chore_event_id)", []).map_err(|e| e.to_string())?;
    // conn.execute("CREATE INDEX IF NOT EXISTS idx_cmessage_belongs_to_cthread_id ON cmessages (cmessage_belongs_to_cthread_id)", []).map_err(|e| e.to_string())?;
    Ok(())
}
