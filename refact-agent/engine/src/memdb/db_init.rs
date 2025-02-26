use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Notify as ANotify;
use parking_lot::Mutex as ParkMutex;
use rusqlite::Connection;

use crate::memdb::db_structs::MemDB;
use crate::vecdb::vdb_structs::VecdbConstants;

fn _make_connection(
    config_dir: &PathBuf,
    constants: &VecdbConstants,
) -> Result<Arc<ParkMutex<MemDB>>, String> {
    let db_path = config_dir.join("memdb.sqlite");
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
    let db = MemDB {
        lite: Arc::new(ParkMutex::new(db)),
        vecdb_constants: constants.clone(),
        dirty_memids: Vec::new(),
        dirty_everything: true,
        memdb_sleeping_point: Arc::new(ANotify::new()),
    };
    Ok(Arc::new(ParkMutex::new(db)))
}

pub async fn memdb_init(
    config_dir: &PathBuf,
    constants: &VecdbConstants,
    reset_memory: bool,
) -> Arc<ParkMutex<MemDB>> {
    let db = match _make_connection(config_dir, constants) {
        Ok(db) => db,
        Err(err) => panic!("Failed to initialize chore database: {}", err),
    };
    let (lite_arc, memdb_sleeping_point) = {
        let locked_db = db.lock();
        (locked_db.lite.clone(), locked_db.memdb_sleeping_point.clone())
    };
    crate::memdb::db_schema::create_tables_202412(&*lite_arc.lock(), memdb_sleeping_point, reset_memory)
        .expect("Failed to create tables");
    db
}
