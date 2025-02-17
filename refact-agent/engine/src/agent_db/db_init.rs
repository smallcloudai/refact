use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Notify as ANotify;
use parking_lot::Mutex as ParkMutex;
use rusqlite::Connection;

use crate::agent_db::db_structs::ChoreDB;


fn _make_connection(
    config_dir: &PathBuf,
) -> Result<Arc<ParkMutex<ChoreDB>>, String> {
    let db_path = config_dir.join("chore_db.sqlite");
    let db = Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
        | rusqlite::OpenFlags::SQLITE_OPEN_FULL_MUTEX
        | rusqlite::OpenFlags::SQLITE_OPEN_URI
    ).map_err(|err| format!("Failed to open database: {}", err))?;
    db.busy_timeout(std::time::Duration::from_secs(30)).map_err(|err| format!("Failed to set busy timeout: {}", err))?;
    db.execute_batch("PRAGMA cache_size = 0; PRAGMA shared_cache = OFF;").map_err(|err| format!("Failed to set cache pragmas: {}", err))?;
    let journal_mode: String = db.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0)).map_err(|err| format!("Failed to set journal mode: {}", err))?;
    if journal_mode != "wal" {
        return Err(format!("Failed to set WAL journal mode. Current mode: {}", journal_mode));
    }
    let db = ChoreDB {
        lite: Arc::new(ParkMutex::new(db)),
        chore_sleeping_point: Arc::new(ANotify::new()),
    };
    Ok(Arc::new(ParkMutex::new(db)))
}

pub async fn chore_db_init(
    config_dir: &PathBuf,
    reset_memory: bool,
) -> Arc<ParkMutex<ChoreDB>> {
    let db = match _make_connection(config_dir) {
        Ok(db) => db,
        Err(err) => panic!("Failed to initialize chore database: {}", err),
    };
    let lite_arc = {
        db.lock().lite.clone()
    };
    crate::agent_db::db_schema_20241102::create_tables_20241102(&*lite_arc.lock(), reset_memory).expect("Failed to create tables");
    db
}
