use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use regex::Regex;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::subchat::subchat_single;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ChatToolCall, ChatToolFunction, ContextEnum, ContextFile};


const MODEL_NAME: &str = "gpt-4o-mini"; // TODO: move to a ChatPost


pub struct AttLocate;

#[async_trait]
impl Tool for AttLocate{
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<Vec<ContextEnum>, String> {
        let problem_statement_summary = match args.get("problem_statement") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `problem_statement` is not a string: {:?}", v)),
            None => return Err("Missing argument `problem_statement`".to_string())
        };

        let (top_n_default, n_ctx_default) = {
            let mut ccx_lock = ccx.lock().await;
            let (top_n_default, n_ctx_default) = (ccx_lock.top_n, ccx_lock.n_ctx);
            ccx_lock.top_n = 30;
            ccx_lock.n_ctx = 64_000; // TODO: move to a ChatPost
            (top_n_default, n_ctx_default)
        };

        let problem_message_mb = {
            let ccx_locked = ccx.lock().await;
            ccx_locked.messages.iter().filter(|m| m.role == "user").last().map(|x|x.content.clone())
        };

        let mut problem_statement = format!("Problem statement:\n{}", problem_statement_summary);
        if let Some(problem_message) = problem_message_mb {
            problem_statement = format!("{}\n\nProblem described by user:\n{}", problem_statement, problem_message);
        }

        let res = locate_relevant_files(ccx.clone(), problem_statement.as_str(), tool_call_id.clone()).await?;

        {
            let mut ccx_lock = ccx.lock().await;
            ccx_lock.top_n = top_n_default;
            ccx_lock.n_ctx = n_ctx_default;
        }

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: format!("{}", serde_json::to_string_pretty(&res).unwrap()),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok(results)
    }
    fn tool_depends_on(&self) -> Vec<String> {
        vec!["ast".to_string()]
    }
}

fn pretend_tool_call(tool_name: &str, tool_arguments: &str) -> ChatMessage {
    let tool_call = ChatToolCall {
        id: format!("{tool_name}_123"),
        function: ChatToolFunction {
            arguments: tool_arguments.to_string(),
            name: tool_name.to_string()
        },
        tool_type: "function".to_string(),
    };
    ChatMessage {
        role: "assistant".to_string(),
        content: "".to_string(),
        tool_calls: Some(vec![tool_call]),
        tool_call_id: "".to_string(),
        ..Default::default()
    }
}

fn reduce_by_counter<I>(values: I, top_n: usize) -> Vec<String>
where
    I: Iterator<Item = String>,
{
    let mut counter = HashMap::new();
    for s in values {
        *counter.entry(s).or_insert(0) += 1;
    }
    let mut counts_vec: Vec<(String, usize)> = counter.into_iter().collect();
    counts_vec.sort_by(|a, b| b.1.cmp(&a.1));
    let top_n: Vec<(String, usize)> = counts_vec.into_iter().take(top_n).collect();
    top_n.into_iter().map(|x| x.0).collect()
}

async fn strategy_tree(
    ccx: Arc<AMutex<AtCommandsContext>>,
    user_query: &str,
    log_prefix: String,
    tool_call_id: String,
) -> Result<Vec<String>, String> {
    // results = problem + tool_tree + pick 5 files * n_choices_times -> reduce(counters: 5)

    let mut messages = vec![];
    messages.push(ChatMessage::new("system".to_string(), STEP1_DET_SYSTEM_PROMPT.to_string()));
    messages.push(ChatMessage::new("user".to_string(), user_query.to_string()));

    messages.push(pretend_tool_call("tree", "{}"));

    let mut messages = subchat_single(
        ccx.clone(),
        MODEL_NAME,
        messages,
        vec!["tree".to_string()],
        None,
        true,
        None,
        1,
        Some(format!("{log_prefix}-locate-step1-tree")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-locate-step1-tree")),
    ).await?.get(0).ok_or("relevant_files: tree deterministic message was empty. Try again later".to_string())?.clone();

    messages.push(ChatMessage::new("user".to_string(), STRATEGY_TREE_PROMPT.to_string()));

    let n_choices = subchat_single(
        ccx.clone(),
        MODEL_NAME,
        messages,
        vec![],
        Some("none".to_string()),
        false,
        Some(0.8),
        5,
        Some(format!("{log_prefix}-locate-step1-tree-result")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-locate-step1-tree-result")),
    ).await?;

    let file_names_pattern = r"\b(?:[a-zA-Z]:\\|/)?(?:[\w-]+[/\\])*[\w-]+\.\w+\b";
    let re = Regex::new(file_names_pattern).unwrap();

    let filenames = n_choices.into_iter()
        .filter_map(|mut x| x.pop())
        .filter(|x| x.role == "assistant")
        .map(|x| {
            re.find_iter(&x.content)
                .map(|mat| mat.as_str().to_string())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<Vec<_>>>();

    let results = reduce_by_counter(filenames.into_iter().flatten(), 10);

    Ok(results)

}

async fn strategy_definitions_references(
    ccx: Arc<AMutex<AtCommandsContext>>,
    user_query: &str,
    log_prefix: String,
    tool_call_id: String,
) -> Result<(Vec<String>, Vec<String>), String>{
    // results = problem -> (collect definitions + references) * n_choices + map(into_filenames) -> reduce(counters: 5)
    let mut messages = vec![];
    messages.push(ChatMessage::new("system".to_string(), STEP1_DET_SYSTEM_PROMPT.to_string()));
    messages.push(ChatMessage::new("user".to_string(), user_query.to_string()));
    messages.push(ChatMessage::new("user".to_string(), STRATEGY_DEF_REF_PROMPT.to_string()));

    // todo: simply ask list of symbols comma separated, don't ask for tools
    let n_choices = subchat_single(
        ccx.clone(),
        MODEL_NAME,
        messages,
        vec!["definition".to_string(), "references".to_string()],
        Some("required".to_string()),
        false,
        Some(0.8),
        5,
        Some(format!("{log_prefix}-locate-step2-defs-refs")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-locate-step2-defs-refs")),
    ).await?;

    let mut filenames = vec![];
    let mut symbols = vec![];
    for (i, ch_messages) in n_choices.into_iter().enumerate() {
        let ch_symbols = ch_messages.last().unwrap().clone().tool_calls.unwrap_or(vec![]).iter()
            .filter_map(|x| {
                let json_value = serde_json::from_str(&x.function.arguments).unwrap_or(Value::Null);
                json_value.get("symbol").and_then(|v| v.as_str()).map(|s| s.to_string())
            }).collect::<Vec<_>>();
        if ch_symbols.is_empty() {
            continue;
        }
        let ch_messages = subchat_single(
            ccx.clone(),
            MODEL_NAME,
            ch_messages,
            vec![],
            None,
            true,
            None,
            1,
            Some(format!("{log_prefix}-locate-step2-defs-refs-result")),
            Some(tool_call_id.clone()),
            Some(format!("{log_prefix}-locate-step2-defs-refs-result")),
        ).await?.get(0).ok_or("relevant_files: no context files found (strategy_definitions_references). Try again later".to_string())?.clone();

        let only_context_files = ch_messages.into_iter().filter(|x| x.role == "context_file").collect::<Vec<_>>();
        let mut context_files = vec![];
        for m in only_context_files {
            let m_context_files: Vec<ContextFile> = serde_json::from_str(&m.content).map_err(|e| e.to_string())?;
            context_files.extend(m_context_files);
        }
        let ch_filenames = context_files.into_iter().map(|x|x.file_name).collect::<Vec<_>>();
        filenames.push(ch_filenames);
        symbols.push(ch_symbols);
    }

    let file_results = reduce_by_counter(filenames.into_iter().flatten(), 10);
    let sym_results = reduce_by_counter(symbols.into_iter().flatten(), 10);

    Ok((file_results, sym_results))
}

async fn supercat_extract_symbols(
    ccx: Arc<AMutex<AtCommandsContext>>,
    user_query: &str,
    files: Vec<String>,
    symbols: Vec<String>,
    log_prefix: String,
    tool_call_id: String,
) -> Result<Vec<String>, String> {
    let mut messages = vec![];
    messages.push(ChatMessage::new("system".to_string(), STEP1_DET_SYSTEM_PROMPT.to_string()));
    messages.push(ChatMessage::new("user".to_string(), user_query.to_string()));

    let mut supercat_args = HashMap::new();
    supercat_args.insert("paths".to_string(), files.join(","));
    supercat_args.insert("skeleton".to_string(), "true".to_string());
    if !symbols.is_empty() {
        supercat_args.insert("symbols".to_string(), symbols.join(","));
    }

    messages.push(pretend_tool_call(
        "supercat",
        serde_json::to_string(&supercat_args).unwrap().as_str()
    ));

    let mut messages = subchat_single(
        ccx.clone(),
        MODEL_NAME,
        messages,
        vec!["supercat".to_string()],
        None,
        true,
        None,
        1,
        Some(format!("{log_prefix}-locate-step3-cat")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-locate-step3-cat")),
    ).await?.get(0).ok_or("relevant_files: supercat message was empty.".to_string())?.clone();

    messages.push(ChatMessage::new("user".to_string(), SUPERCAT_EXTRACT_SYMBOLS_PROMPT.replace("{USER_QUERY}", &user_query)));

    let n_choices = subchat_single(
        ccx.clone(),
        MODEL_NAME,
        messages,
        vec![],
        Some("none".to_string()),
        false,
        Some(0.8),
        5,
        Some(format!("{log_prefix}-locate-step3-cat-result")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-locate-step3-cat-result")),
    ).await?;

    let mut symbols_result = vec![];
    for msg in n_choices.into_iter().map(|x|x.last().unwrap().clone()).filter(|x|x.role == "assistant") {
        let symbols = {
            let re = Regex::new(r"[^,\s]+").unwrap();
            re.find_iter(&msg.content)
                .map(|mat| mat.as_str().to_string())
                .collect::<Vec<_>>()
        };
        symbols_result.push(symbols);
    }

    let results = reduce_by_counter(symbols_result.into_iter().flatten(), 15);

    Ok(results)
}

async fn supercat_decider(
    ccx: Arc<AMutex<AtCommandsContext>>,
    user_query: &str,
    files: Vec<String>,
    symbols: Vec<String>,
    log_prefix: String,
    tool_call_id: String,
) -> Result<Value, String> {
    let mut messages = vec![];
    messages.push(ChatMessage::new("system".to_string(), STEP1_DET_SYSTEM_PROMPT.to_string()));
    messages.push(ChatMessage::new("user".to_string(), user_query.to_string()));

    let mut supercat_args = HashMap::new();
    supercat_args.insert("paths".to_string(), files.join(","));
    supercat_args.insert("skeleton".to_string(), "true".to_string());
    if !symbols.is_empty() {
        supercat_args.insert("symbols".to_string(), symbols.join(","));
    }

    messages.push(pretend_tool_call(
        "supercat",
        serde_json::to_string(&supercat_args).unwrap().as_str()
    ));

    let mut messages = subchat_single(
        ccx.clone(),
        MODEL_NAME,
        messages,
        vec!["supercat".to_string()],
        None,
        true,
        None,
        1,
        Some(format!("{log_prefix}-locate-step4-det")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-locate-step4-det")),
    ).await?.get(0).ok_or("relevant_files: supercat message was empty.".to_string())?.clone();

    messages.push(ChatMessage::new("user".to_string(), SUPERCAT_DECIDER_PROMPT.replace("{USER_QUERY}", &user_query)));

    let n_choices = subchat_single(
        ccx.clone(),
        MODEL_NAME,
        messages,
        vec![],
        Some("none".to_string()),
        false,
        Some(0.8),
        5,
        Some(format!("{log_prefix}-locate-step4-det-result")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-locate-step4-det-result")),
    ).await?;

    let mut results_to_change = vec![];
    let mut results_context = vec![];

    for ch_messages in n_choices {
        let answer_mb = ch_messages.last().filter(|x|x.role == "assistant").map(|x|x.content.clone());
        if answer_mb.is_none() {
            continue;
        }
        let answer = answer_mb.unwrap();
        let results: Vec<SuperCatResultItem> = match serde_json::from_str(&answer) {
            Ok(x) => x,
            Err(_) => continue
        };
        let to_change = results.iter().filter(|x|x.reason == "to_change").map(|x|x.file_path.clone()).collect::<Vec<_>>();
        if to_change.is_empty() || to_change.len() > 1 {
            continue;
        }
        results_to_change.extend(to_change);

        let context = results.iter().filter(|x|x.reason == "context").map(|x|x.file_path.clone()).collect::<Vec<_>>();
        results_context.push(context);
    }

    let file_to_change = reduce_by_counter(results_to_change.into_iter(), 1).get(0).ok_or("relevant_files: no top file to change (supercat). Try again".to_string())?.clone();
    let files_context = reduce_by_counter(results_context.into_iter().flatten(), 5);

    let mut res = vec![];
    res.push(SuperCatResultItem{file_path: file_to_change, reason: "to_change".to_string()});
    res.extend(
        files_context.into_iter().map(|x| SuperCatResultItem{file_path: x, reason: "context".to_string()})
    );
    Ok(json!(res))
}

async fn locate_relevant_files(
    ccx: Arc<AMutex<AtCommandsContext>>,
    user_query: &str,
    tool_call_id: String,
) -> Result<Value, String> {
    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let mut paths_chosen = vec![];

    let tree_files = strategy_tree(
        ccx.clone(),
        user_query,
        log_prefix.clone(),
        tool_call_id.clone()
    ).await?;

    let (def_ref_files, mut symbols) = strategy_definitions_references(
        ccx.clone(),
        user_query,
        log_prefix.clone(),
        tool_call_id.clone()
    ).await?;

    paths_chosen.extend(tree_files);
    paths_chosen.extend(def_ref_files);

    let extra_symbols = supercat_extract_symbols(
        ccx.clone(),
        user_query,
        paths_chosen.clone(),
        symbols.clone(),
        log_prefix.clone(),
        tool_call_id.clone()
    ).await?;

    symbols.extend(extra_symbols);

    let results = supercat_decider(
        ccx.clone(),
        user_query,
        paths_chosen,
        symbols,
        log_prefix.clone(),
        tool_call_id.clone()
    ).await?;
    Ok(results)
}


const SUPERCAT_DECIDER_PROMPT: &str = r###"
Read slowly and carefully the problem text one more time before you start:

{USER_QUERY}

In the previous message you were given a generous context -- skeletonized files.

TODO:
1. analyse thoroughly the problem statement;
2. analyse thoroughly context given (skeletonized files from the previous message);
3. among the files pick the one you need to make changes to solve the problem (according to the problem statement);
4. among the files pick at least 5 more files that will give you the best context to make make changes in the chosen file (from step 3);
5. return the results in a format specified below;

Format you must obey:
[
    {
        "file_path": "/a/b/c/file.py",
        "reason": "to_change"
    },
    {
        "file_path": "/a/b/c/file1.py",
        "reason": "context"
    }
    ...
]

file_path must be an absolute path.
format you return must be a valid JSON, explain nothing, don't use any quotes or backticks.

"###;

#[derive(Serialize, Deserialize, Debug)]
struct SuperCatResultItem {
    file_path: String,
    reason: String,
}

const SUPERCAT_EXTRACT_SYMBOLS_PROMPT: &str = r###"
Read slowly and carefully the problem text one more time before you start:

{USER_QUERY}

1. analyse thoroughly the problem statement;
2. analyse thoroughly context given (skeletonized files from the previous message);
3. from the given context select all the symbols (functino names, classes names etc) that you find relevant to the problem (either give releavant context or need to be changed);
4. return the results comma separated. Do not explain anything. Avoid backticks.

Output must be like this:
MyClass, MyFunction, MyType
"###;

const STRATEGY_TREE_PROMPT: &str = r###"
TODO:
1. analyse thoroughly the problem statement;
2. look thoroughly at the project tree given;
3. pick at least 10 files that will help you solving the problem (ones that give you the context and ones that shall be changed);
4. return chosen files in a json format, explain nothing.

Output must be like this:
[
    "file1.py",
    "file2.py"
]
"###;

const STRATEGY_DEF_REF_PROMPT: &str = r###"
TODO:
1. analyse thoroughly the problem statement;
2. from the problem statement pick up AST Symbols (classes, functions, types, variables etc) that are relevant to the problem;
3. call functions (definition, referencies) to ask for a relevant context that will give you ability to solve the problem.
"###;

const STEP1_DET_SYSTEM_PROMPT: &str = r###"
You are a genious coding assistant named "Refact". You are known for your scruplousness and well thought-out code.
Listening to the user is what makes you the best.
"###;
