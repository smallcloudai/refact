use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{RwLock as ARwLock, Mutex as AMutex};
use indexmap::IndexSet;

use crate::global_context::GlobalContext;
use crate::agent_db::db_structs::{CThread, CMessage};
use crate::agent_db::chore_pubsub_sleeping_procedure;
use crate::agent_db::db_cthread::CThreadSubscription;
use crate::call_validation::{ChatContent, ChatMessage, ChatUsage};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::subchat::subchat_single;

const SLEEP_IF_NO_WORK_SEC: u64 = 10;
const LOCK_TOO_OLD_SEC: f64 = 600.0;


pub async fn look_for_a_job(
    gcx: Arc<ARwLock<GlobalContext>>,
    worker_n: usize,
) {
    let worker_pid = std::process::id();
    let worker_name = format!("aworker-{}-{}", worker_pid, worker_n);
    let cdb = gcx.read().await.chore_db.clone();
    let lite_arc = cdb.lock().lite.clone();

    let (mut might_work_on_cthread_id, mut last_pubsub_id) = {
        let lite = cdb.lock().lite.clone();
        // intentional unwrap(), it's better to crash quickly than continue with a non-functioning thread
        let max_pubsub_id: i64 = lite.lock().query_row("SELECT COALESCE(MAX(pubevent_id), 0) FROM pubsub_events", [], |row| row.get(0)).unwrap();
        let post = CThreadSubscription {
            quicksearch: "".to_string(),
            limit: 100,
        };
        // intentional unwrap()
        let cthreads = crate::agent_db::db_cthread::cthread_quicksearch(cdb.clone(), &String::new(), &post).unwrap();
        let mut might_work_on_cthread_id = IndexSet::new();
        for ct in cthreads.iter() {
            might_work_on_cthread_id.insert(ct.cthread_id.clone());
        }
        (might_work_on_cthread_id, max_pubsub_id)
    };

    loop {
        let sleep_seconds = if might_work_on_cthread_id.is_empty() { SLEEP_IF_NO_WORK_SEC } else { 1 };
        if !chore_pubsub_sleeping_procedure(gcx.clone(), &cdb, sleep_seconds).await {
            break;
        }
        let (deleted_cthread_ids, updated_cthread_ids) = match crate::agent_db::db_cthread::cthread_subsription_poll(lite_arc.clone(), &mut last_pubsub_id) {
            Ok(x) => x,
            Err(e) => {
                tracing::error!("wait_for_cthread_to_work_on(1): {}", e);
                break;
            }
        };
        might_work_on_cthread_id.extend(updated_cthread_ids.into_iter());
        for deleted_id in deleted_cthread_ids {
            might_work_on_cthread_id.remove(&deleted_id);
        }

        while let Some(cthread_id) = might_work_on_cthread_id.iter().next().cloned() {
            match look_if_the_job_for_me(gcx.clone(), &worker_name, &cthread_id).await {
                Ok(lock_success) => {
                    if lock_success {
                        might_work_on_cthread_id.remove(&cthread_id);
                    }
                }
                Err(e) => {
                    tracing::error!("{} cannot work on {}: {}", worker_name, cthread_id, e);
                    might_work_on_cthread_id.remove(&cthread_id);
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                }
            }
        }
    }
}

async fn look_if_the_job_for_me(
    gcx: Arc<ARwLock<GlobalContext>>,
    worker_name: &String,
    cthread_id: &String,
) -> Result<bool, String> {
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f64();
    let cdb = gcx.read().await.chore_db.clone();
    let lite_arc = cdb.lock().lite.clone();
    let (cthread_rec, cmessages) = {
        let mut conn = lite_arc.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let mut cthread_rec = {
            let mut stmt = tx.prepare("SELECT * FROM cthreads WHERE cthread_id = ?1").unwrap();
            let rows = stmt.query(rusqlite::params![cthread_id]).map_err(|e| e.to_string())?;
            let mut cthreads = crate::agent_db::db_cthread::cthreads_from_rows(rows);
            cthreads.pop().ok_or_else(|| format!("No CThread found with id: {}", cthread_id))?
        };

        let cmessages = {
            let mut stmt = tx.prepare("SELECT * FROM cmessages WHERE cmessage_belongs_to_cthread_id = ?1 ORDER BY cmessage_num, cmessage_alt").unwrap();
            let rows = stmt.query(rusqlite::params![cthread_id]).map_err(|e| e.to_string())?;
            crate::agent_db::db_cmessage::cmessages_from_rows(rows)
        };

        assert!(cthread_rec.cthread_locked_by != *worker_name);

        let busy = !cthread_rec.cthread_locked_by.is_empty() && cthread_rec.cthread_locked_ts + LOCK_TOO_OLD_SEC > now;
        if busy {
            tracing::info!("{} {} busy", worker_name, cthread_id);
            return Ok(false);
        }

        let last_message_is_user = cmessages.last().map_or(false, |cmsg| {
            let cmessage: serde_json::Value = serde_json::from_str(&cmsg.cmessage_json).unwrap();
            cmessage["role"] == "user"
        });

        tracing::info!("{} {} last_message_is_user={} cthread_rec.cthread_error={:?}", worker_name, cthread_id, last_message_is_user, cthread_rec.cthread_error);
        if !last_message_is_user || !cthread_rec.cthread_error.is_empty() {
            return Ok(true);  // true means don't come back to it again
        }

        cthread_rec.cthread_locked_by = worker_name.clone();
        cthread_rec.cthread_locked_ts = now;
        crate::agent_db::db_cthread::cthread_set_lowlevel(&tx, &cthread_rec)?;
        tx.commit().map_err(|e| e.to_string())?;
        (cthread_rec, cmessages)
    };

    tracing::info!("{} {} autonomous work start", worker_name, cthread_id);
    let mut apply_json: serde_json::Value;

    match do_the_job(gcx, worker_name, &cthread_rec, &cmessages).await {
        Ok(result) => {
            apply_json = result;
        }
        Err(e) => {
            apply_json = serde_json::json!({
                "cthread_error": format!("{}", e),
            });
        }
    }
    apply_json["cthread_id"] = serde_json::json!(cthread_id);
    apply_json["cthread_locked_by"] = serde_json::json!("");
    apply_json["cthread_locked_ts"] = serde_json::json!(0);
    tracing::info!("{} {} /autonomous work\n{}", worker_name, cthread_id, apply_json);
    crate::agent_db::db_cthread::cthread_apply_json(cdb, apply_json)?;

    Ok(true)  // true means don't come back to it again
}

async fn do_the_job(
    gcx: Arc<ARwLock<GlobalContext>>,
    worker_name: &String,
    cthread_rec: &CThread,
    cmessages: &Vec<CMessage>,
) -> Result<serde_json::Value, String> {
    let cdb = gcx.read().await.chore_db.clone();
    let (lite, chore_sleeping_point) = {
        let db = cdb.lock();
        (db.lite.clone(), db.chore_sleeping_point.clone())
    };

    let messages: Vec<ChatMessage> = cmessages.iter().map(|cmsg| { serde_json::from_str(&cmsg.cmessage_json).map_err(|e| format!("{}", e))}).collect::<Result<Vec<_>, _>>()?;
    let message_info: Vec<String> = messages.iter().map(|msg| {
        let role = &msg.role;
        let content_brief = match &msg.content {
            ChatContent::SimpleText(text) => { format!("{}", text.len()) },
            ChatContent::Multimodal(elements) => {
                elements.iter().map(|el| {
                    if el.is_text() {
                        format!("text{}", el.m_content.len())
                    } else {
                        format!("{}[image]", el.m_type)
                    }
                }).collect::<Vec<_>>().join("+")
            },
        };
        let mut tool_calls_brief = match &msg.tool_calls {
            Some(tool_calls) => tool_calls.iter().map(|call| call.function.name.clone()).collect::<Vec<_>>().join("/"),
            None => String::new(),
        };
        if !tool_calls_brief.is_empty() {
            tool_calls_brief.insert(0, '/');
        }
        format!("{}/{}{}", role, content_brief, tool_calls_brief)
    }).collect();
    let message_info_str = message_info.join(", ");
    tracing::info!("{} started work on {}\n[{}]", worker_name, cthread_rec.cthread_id, message_info_str);

    // TODO: make something similar to the `subchat` with chat `wrapping` logic
    // wrap_up_depth: usize,
    // wrap_up_tokens_cnt: usize,
    // wrap_up_prompt: &str,
    // wrap_up_n: usize,
    let mut usage = ChatUsage { ..Default::default() };
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        cthread_rec.cthread_n_ctx,
        10,
        false,
        messages.clone(),
        cthread_rec.cthread_id.clone(),
        false,
    ).await));
    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let chat_response_msgs = subchat_single(
        ccx.clone(),
        cthread_rec.cthread_model.as_str(),
        messages,
        None,
        None,
        false,
        Some(cthread_rec.cthread_temperature as f32),
        Some(cthread_rec.cthread_max_new_tokens),
        cthread_rec.cthread_n,
        Some(&mut usage),
        Some(cthread_rec.cthread_id.clone()),
        Some(format!("{log_prefix}-chore-job")),
    ).await.map_err(|e| format!("Error: {}", e))?;

    let choice0: Vec<ChatMessage> = chat_response_msgs[0].clone();
    {
        let mut lite_locked = lite.lock();
        let tx = lite_locked.transaction().map_err(|e| e.to_string())?;
        for (i, chat_message) in choice0.iter().enumerate() {
            let mut cmessage_usage_prompt = 0;
            let mut cmessage_usage_completion = 0;
            if let Some(u) = &chat_message.usage {
                cmessage_usage_prompt = u.prompt_tokens as i32;
                cmessage_usage_completion = u.completion_tokens as i32;
            }
            let cmessage = CMessage {
                cmessage_belongs_to_cthread_id: cthread_rec.cthread_id.clone(),
                cmessage_alt: 0,
                cmessage_num: (cmessages.len() as i32) + (i as i32),
                cmessage_prev_alt: 0,
                cmessage_usage_model: cthread_rec.cthread_model.clone(),
                cmessage_usage_prompt,
                cmessage_usage_completion,
                cmessage_json: serde_json::to_string(chat_message).map_err(|e| format!("{}", e))?,
            };
            crate::agent_db::db_cmessage::cmessage_set(&tx, cmessage);
        }
        tx.commit().map_err(|e| e.to_string())?;
    }
    chore_sleeping_point.notify_waiters();
    Ok(serde_json::json!({}))
}

pub async fn look_for_a_job_start_tasks(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();
    for n in 0..1 {
        let handle = tokio::spawn(look_for_a_job(
            gcx.clone(),
            n,
        ));
        handles.push(handle);
    }
    handles
}
