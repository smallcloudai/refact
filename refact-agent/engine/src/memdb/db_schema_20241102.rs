use rusqlite::Connection;


pub fn create_tables_20241102(conn: &Connection, reset_memory: bool) -> Result<(), String> {
    if reset_memory {
        conn.execute("DROP TABLE IF EXISTS pubsub_events", []).map_err(|e| e.to_string())?;
        conn.execute("DROP TABLE IF EXISTS chores", []).map_err(|e| e.to_string())?;
        conn.execute("DROP TABLE IF EXISTS chore_events", []).map_err(|e| e.to_string())?;
        conn.execute("DROP TABLE IF EXISTS cthreads", []).map_err(|e| e.to_string())?;
        conn.execute("DROP TABLE IF EXISTS cmessages", []).map_err(|e| e.to_string())?;
    }
    conn.execute(
        "CREATE TABLE IF NOT EXISTS pubsub_events (
            pubevent_id INTEGER PRIMARY KEY AUTOINCREMENT,
            pubevent_channel TEXT NOT NULL,
            pubevent_action TEXT NOT NULL,
            pubevent_json TEXT NOT NULL,
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
    // Useful to speed up SELECT .. JOIN
    // conn.execute("CREATE INDEX IF NOT EXISTS idx_chore_event_belongs_to_chore_id ON chore_events (chore_event_belongs_to_chore_id)", []).map_err(|e| e.to_string())?;
    // conn.execute("CREATE INDEX IF NOT EXISTS idx_cthread_belongs_to_chore_event_id ON cthreads (cthread_belongs_to_chore_event_id)", []).map_err(|e| e.to_string())?;
    // conn.execute("CREATE INDEX IF NOT EXISTS idx_cmessage_belongs_to_cthread_id ON cmessages (cmessage_belongs_to_cthread_id)", []).map_err(|e| e.to_string())?;
    Ok(())
}
