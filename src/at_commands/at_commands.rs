use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_workspace::AtWorkspace;
use crate::at_commands::at_file::AtFile;
use crate::at_commands::structs::AtCommandKind;


pub async fn at_commands_dict() -> HashMap<String, Arc<AMutex<AtCommandKind>>> {
    return HashMap::from([
        ("@workspace".to_string(), Arc::new(AMutex::new(AtCommandKind::AtWorkspace(AtWorkspace::new())))),
        ("@file".to_string(), Arc::new(AMutex::new(AtCommandKind::AtFile(AtFile::new())))),
    ]);
}
