use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex as AMutex;
use tokio::process::Command;
use async_trait::async_trait;
use tracing::{error, info};
use serde::{Deserialize, Serialize};

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ContextEnum, ChatMessage};

use crate::tools::tools_description::Tool;
use serde_json::Value;


// docker run --label horrible --name stupid_script_100 oleg_aaa1 python3 stupid_script.py arg1 arg2
// docker ps -a --filter "label=horrible"
// docker create --name my_stupid_script_container --label task=stupid_script aaa1
// (not run)
// (follow by start)


// docker build -t aaa1 . && \
// docker run -d --name my_stupid_script_container --label task=stupid_script aaa1 && \
// docker cp /path/to/your/binary my_stupid_script_container:/path/in/container/binary && \
// docker exec -it my_stupid_script_container /path/in/container/binary && \
// docker stop my_stupid_script_container && \
// docker rm my_stupid_script_container
