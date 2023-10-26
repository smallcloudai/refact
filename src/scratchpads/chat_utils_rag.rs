use crate::call_validation::{ChatMessage, ChatPost, ContextFile};
use crate::vecdb::structs::{SearchResult, VecdbSearch};

pub async fn embed_vecdb_results<T>(
    vecdb: &T,
    msgs: &Vec<ChatMessage>,
    limit_examples_cnt: usize,
) -> Vec<ChatMessage> where T: VecdbSearch {
    let latest_msg_cont = &msgs.last().unwrap().content;
    let vdb_resp = vecdb.search(latest_msg_cont.clone(), limit_examples_cnt).await;
    let vdb_cont = vecdb_resp_to_prompt(&vdb_resp);
    if vdb_cont.is_ok() {
        return [
            &msgs[..msgs.len() - 1],
            &[ChatMessage {
                role: "context_file".to_string(),
                content: vdb_cont.unwrap(),
            }],
            &msgs[msgs.len() - 1..],
        ].concat();
    } else {
        return msgs.clone();
    }
}

fn vecdb_resp_to_prompt(
    resp: &Result<SearchResult, String>
) -> serde_json::Result<String> {
    let context_files: Vec<ContextFile> = match resp {
        Ok(search_res) => {
            search_res.results.iter().map(
                |x| ContextFile {
                    file_name: x.file_path.to_str().unwrap().to_string(),
                    file_content: x.window_text.clone(),
                    line1: x.start_line as i32,
                    line2: x.end_line as i32,
                }
            ).collect()
        }
        Err(_) => vec![]
    };
    serde_json::to_string(&context_files)
}
