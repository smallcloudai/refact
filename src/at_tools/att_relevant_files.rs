use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use regex::Regex;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use futures_util::future::join_all;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::subchat::{subchat, subchat_single};
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ChatToolCall, ChatToolFunction, ContextEnum, ContextFile};
use crate::global_context::GlobalContext;


const MODEL_NAME: &str = "gpt-4o-mini";


pub struct AttRelevantFiles;

#[async_trait]
impl Tool for AttRelevantFiles {
    async fn tool_execute(&mut self, ccx: Arc<AMutex<AtCommandsContext>>, tool_call_id: &String, _args: &HashMap<String, Value>) -> Result<Vec<ContextEnum>, String> {
        let (top_n_default, n_ctx_default) = {
            let mut ccx_lock = ccx.lock().await;
            let (top_n_default, n_ctx_default) = (ccx_lock.top_n, ccx_lock.n_ctx);
            ccx_lock.top_n = None;
            ccx_lock.n_ctx = 64_000;
            (top_n_default, n_ctx_default)
        };
        let problem = {
            let ccx_locked = ccx.lock().await;
            ccx_locked.messages.iter().filter(|m| m.role == "user").last().map(|x|x.content.clone()).ok_or(
                "relevant_files: unable to find user problem description".to_string()
            )?
        };

        let res = find_relevant_files(ccx, tool_call_id.clone(), problem.as_str()).await?;

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
        None,
        Some("strategy-tree".to_string()),
        None,
        None,
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
        Some(5usize),
        Some("strategy-tree-choices".to_string()),
        None,
        None,
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
        Some(5usize),
        Some("strategy-definitions-references".to_string()),
        None,
        None,
    ).await?;

    let mut filenames = vec![];
    let mut symbols = vec![];
    for ch_messages in n_choices.into_iter() {
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
            None,
            None,
            None,
            None,
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
) -> Result<Vec<String>, String> {
    let mut messages = vec![];
    messages.push(ChatMessage::new("system".to_string(), STEP1_DET_SYSTEM_PROMPT.to_string()));
    messages.push(ChatMessage::new("user".to_string(), user_query.to_string()));

    let mut supercat_args = HashMap::new();
    supercat_args.insert("paths".to_string(), files.join(","));
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
        None,
        Some("supercat1-extract-det".to_string()),
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
        Some(5usize),
        Some("supercat2-extract".to_string()),
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
) -> Result<Value, String> {
    let mut messages = vec![];
    messages.push(ChatMessage::new("system".to_string(), STEP1_DET_SYSTEM_PROMPT.to_string()));
    messages.push(ChatMessage::new("user".to_string(), user_query.to_string()));

    let mut supercat_args = HashMap::new();
    supercat_args.insert("paths".to_string(), files.join(","));
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
        None,
        Some("supercat1-det".to_string()),
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
        Some(5usize),
        Some("supercat2-choices".to_string()),
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

#[allow(dead_code)]
async fn find_relevant_files_det(
    ccx: Arc<AMutex<AtCommandsContext>>,
    user_query: &str
) -> Result<Value, String> {
    // all_results = [*strategy_1, *strategy_2, .. *strategy_n] -> call supercat(files or files + symbols) -> let model decide

    let mut paths_chosen = vec![];
    let tree_files = strategy_tree(ccx.clone(), user_query).await?;
    let (def_ref_files, mut symbols) = strategy_definitions_references(ccx.clone(), user_query).await?;
    println!("\n\nSYMBOLS: {:?}\n\n", symbols);
    paths_chosen.extend(tree_files);
    paths_chosen.extend(def_ref_files);
    
    let extra_symbols = supercat_extract_symbols(
        ccx.clone(),
        user_query,
        paths_chosen.clone(),
        symbols.clone(),
    ).await?;
    println!("\n\nEXTRA SYMBOLS: {:?}\n\n", extra_symbols);
    symbols.extend(extra_symbols);
    

    let results = supercat_decider(
        ccx.clone(),
        user_query,
        paths_chosen,
        symbols,
    ).await?;
    // let results = json!(paths_chosen.into_iter().map(|x|SuperCatResultItem{
    //     file_path: x.to_string(),
    //     reason: "none".to_string(),
    // }).collect::<Vec<_>>());
    Ok(results)
}


#[allow(dead_code)]
async fn find_relevant_files(
    ccx: Arc<AMutex<AtCommandsContext>>,
    tool_call_id: String,
    user_query: &str,
) -> Result<Value, String> {
    let gcx: Arc<ARwLock<GlobalContext>> = ccx.lock().await.global_context.clone();
    let vecdb_on = {
        let gcx = gcx.read().await;
        let vecdb = gcx.vec_db.lock().await;
        vecdb.is_some()
    };

    let sys = RF_SYSTEM_PROMPT
        .replace("{ATTEMPTS}", &format!("{RF_ATTEMPTS}"))
        .replace("{OUTPUT_FILES}", &format!("{RF_OUTPUT_FILES}"));

    let mut messages = vec![];
    messages.push(ChatMessage::new("system".to_string(), sys.to_string()));
    messages.push(ChatMessage::new("user".to_string(), user_query.to_string()));
    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

    let mut tools_subset = vec!["definition", "references", "tree", "knowledge", "file"].iter().map(|x|x.to_string()).collect::<Vec<_>>();
    let mut strategies = vec!["CATFILES", "GOTODEF", "GOTOREF", "CUSTOM"];

    if vecdb_on {
        tools_subset.push("search".to_string());
        strategies.push("VECDBSEARCH");
    }
    let mut futures = vec![];
    for strategy in strategies {
        let mut messages_copy = messages.clone();
        messages_copy.push(ChatMessage::new("user".to_string(), USE_STRATEGY_PROMPT.replace("{USE_STRATEGY}", &strategy)));
        let f = subchat(
            ccx.clone(),
            MODEL_NAME,
            messages_copy,
            tools_subset.clone(),
            RF_WRAP_UP_DEPTH,
            RF_WRAP_UP_TOKENS_CNT,
            RF_PLEASE_WRITE_MEM,
            None,
            Some(format!("{log_prefix}-rf-step1-{strategy}")),
            Some(tool_call_id.clone()),
            Some(format!("{log_prefix}-rf-step1-{strategy}")),
        );
        futures.push(f);
    }

    let results = join_all(futures).await.into_iter().filter_map(|x|x.ok()).collect::<Vec<_>>();
    let only_last_messages = results.into_iter()
        .filter_map(|mut x| x.pop())
        .filter(|x| x.role == "assistant").collect::<Vec<_>>();

    let mut messages = vec![];
    messages.push(ChatMessage::new("system".to_string(), RF_REDUCE_SYSTEM_PROMPT.to_string()));
    messages.push(ChatMessage::new("user".to_string(), format!("User provided task:\n\n{}", user_query)));
    for (i, expert_message) in only_last_messages.into_iter().enumerate() {
        messages.push(ChatMessage::new("user".to_string(), format!("Expert {} says:\n\n{}", i + 1, expert_message.content)));
    }
    messages.push(ChatMessage::new("user".to_string(), format!("{}", RF_REDUCE_USER_MSG)));

    let result = subchat_single(
        ccx.clone(),
        MODEL_NAME,
        messages,
        vec![],
        None,
        false,
        None,
        None,
        Some(format!("{log_prefix}-rf-step2-reduce")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-rf-step2-reduce")),
    ).await?[0].clone();

    let answer = parse_reduce_output(&result.last().unwrap().content)?;
    Ok(answer)
}

const RF_OUTPUT_FILES: usize = 6;
const RF_ATTEMPTS: usize = 1;
const RF_WRAP_UP_DEPTH: usize = 5;
const RF_WRAP_UP_TOKENS_CNT: usize = 8000;


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

Format thee must obey:
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
3. pick at least 10 files that will help thee solving the problem (ones that give you the context and ones that shall be changed);
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


const USE_STRATEGY_PROMPT: &str = r###"
💿 The strategy you must follow is {USE_STRATEGY}
"###;

const RF_SYSTEM_PROMPT: &str = r###"You are an expert in finding relevant files within a big project. Your job is to find files, don't propose any changes.

Look at task description. Here's the list of reasons a file might be relevant wrt task description:
TOCHANGE = changes to that file are necessary to complete the task
DEFINITIONS = file has classes/functions/types involved, but no changes needed
HIGHLEV = file is crucial to understand the logic, such as a database scheme, high level script
USERCODE = file has code that uses the things the task description is about

You have to be mindful of the token count, as some files are large. It's essential to
select specific symbols within a file that are relevant. Another expert will
pick up your results, likely they will have to only look at symbols selected by you,
not whole files, because of the space constraints.

Here's your plan:
1. Call knowledge(), pass a short version of the task as im_going_to_do parameter. This call is
a memory mechanism, it will tell you about your previous attempts at doing the same task. Don't
plan anything until you see your achievements in previous attempts.
2. Don't rewrite data from memory. You need to decide if you want to continue with the unfinished strategy, or try a strategy you didn't use yet, so write only about that. Prefer new strategies, over already tried ones.
3. If the strategy is finished or goes in circles, try a new strategy. Don't repeat the same actions over again. A new strategy is better than a tried one, because it's likely to bring you results you didn't see yet.
4. Make sure the output form is sufficiently filled, actively fill the gaps.
5. There's a hard limit of {ATTEMPTS} attempts. Your memory will tell you which attempt you are on. Make sure on attempt number {ATTEMPTS} to put together your best final answer.

Potential strategies:
CATFILES = call tree(), spot up to {OUTPUT_FILES} suspicious files just by looking at file names, look into them by calling file() in parallel, write down relevant function/class names, summarize what they do. Stop this after checking {OUTPUT_FILES} files, switch to a different strategy.
GOTODEF = call definition() for symbols involved, get more files this way. Don't call for symbols from standard libraries, only symbols within the project are indexed.
GOTOREF = call references() to find usages of the code to be changed to identify what exactly calls or uses the thing in question.
VECDBSEARCH = search() can find semantically similar code, text in comments, and sometimes documentation.
CUSTOM = a different strategy that makes sense for the task at hand.

You'll receive additional instructions that start with 💿. Those are not coming from the user, they are programmed to help you operate
well between chat restarts and they are always in English. Answer in the language the user prefers.

EXPLAIN YOUR ACTIONS BEFORE CALLING ANY FUNCTIONS. IT'S FORBIDDEN TO CALL TOOLS UNTIL YOU EXPLAINED WHAT YOU ARE GOING TO DO.
"###;

const RF_PLEASE_WRITE_MEM: &str = r###"You are out of turns or tokens for this chat. Now you need to save your progress, such that a new chat can pick up from there. Use this structure:
{
  "PROGRESS": {
    "UNFINISHED_STRATEGY": "string",             // Maybe you've got interrupted at a worst possible moment, you were in the middle of executing a good plan! Write down your strategy, which is it? "I was calling this and this tool and looking for this". This is a text field, feel free to write a paragraph. Leave an empty string to try something else on the next attempt.
    "UNFINISHED_STRATEGY_POINTS_TODO": "string", // In that unfinished strategy, what specific file names or symbols are left to explore? Don't worry about any previous strategies, just the unfinished one. Use comma-separated list, leave an empty string if there's no unfinished strategy. For file paths, omit most of the path, maybe leave one or two parent dirs, just enough for the path not to be ambiguous.
    "STRATEGIES_IN_MEMORY": "string",            // Write comma-separated list of which strategies you can detect in memory (CATFILES, GOTODEF, GOTOREF, VECDBSEARCH, CUSTOM) by looking at action sequences.
    "STRATEGIES_DIDNT_USE": "string"             // Which strategies you can't find in memory or in this chat? (CATFILES, GOTODEF, GOTOREF, VECDBSEARCH, CUSTOM)
  },
  "ACTION_SEQUENCE": {
    "ACTIONS": [           // Write the list of your actions in this chat (not the actions from memory), don't be shy of mistakes, don't omit anything.
      ["string", "string"] // Each element is a tuple with a tool call or a change in your interpretation (aha moments, discoveries, errors), for example ["call", "definition", "MyClass"] or ["discovery", "there are no MyClass in this project, but there is MyClass2"]
    ],
    "GOAL": "string",      // What the goal of the actions above appears to be? It could be the original goal, but try to be more specific.
    "SUCCESSFUL": 0,       // Did your actions actually get you closer to the goal? Rate from 1 to 5. 1 = no visible progress at all or moving in circles, 3 = at least it's going somewhere, 5 = clearly moving towards the goal.
    "REFLECTION": "string" // If the actions were inefficient, what would you have done differently? Write an empty string if the actions were good as it is, or it's not clear how to behave better.
  },
  "OUTPUT": {                       // The output is dict<filename, info_dict>. You don't have to repeat all the previous files visible in memory, but you need to add new relevant files (not all files, only the relevant files) from the current attempt, as well as update info for files already visible in memory, if there are updates in the current chat.
    "dir/dir/file.ext": {           // Here you need a strict absolute path with no ambiguity at all.
      "SYMBOLS": "symbol1,symbol2", // Comma-separated list of functions/classes/types/variables/etc defined within this file that are actually relevant, for example "MyClass::my_function". List all symbols that are relevant, not just some of them. Write "*" to indicate the whole file is necessary. Write "TBD" to indicate you didn't look inside yet.
      "WHY_CODE": "string",         // Write down the reason to include this file in output, pick one of: TOCHANGE, DEFINITIONS, HIGHLEV, USERCODE. Put TBD if you didn't look inside.
      "WHY_DESC": "string",         // Describe why this file matters wrt the task, what's going on inside? Put TBD if you didn't look inside.
      "RELEVANCY": 0                // Critically evaluate how is this file really relevant to the task. Rate from 1 to 5. 1 = this file doesn't even exist, 3 = might provide good insight into the logic behind the program but not directly relevant, 5 = exactly what is needed.
    }
  ],
  "READY": 0,                       // Is the output good enough to give to the user, when you look at output visible in memory and add the output from this attempt? Rate from 1 to 5. 1 = not much was found, 3 = some good files were found, but there are gaps left to fill, such as symbols that actually present in the file and relevant. 5 = the output is very very good, all the classes files (TOCHANGE, DEFINITIONS, HIGHLEV, USERCODE) were checked.
  "STRATEGIES_THIS_CHAT": "string", // Write comma-separated list of which strategies you see you used in this chat (CATFILES, GOTODEF, GOTOREF, VECDBSEARCH, CUSTOM)
  "ALL_STRATEGIES": 0               // Rate from 1 to 5. 1 = one of less strategies tried, 3 = a couple were attempted, 5 = three or more strategies that make sense for the task attempted, and successfully so.
}
"###;

const RF_REDUCE_SYSTEM_PROMPT: &str = r###"
You will receive output generated by experts using different strategies. They will give you this format:

{
  ...
  "OUTPUT": {
    "dir/dir/file.ext": {
      "SYMBOLS": "symbol1,symbol2", // Comma-separated list of functions/classes/types/variables/etc defined within this file that are actually relevant. "*" might indicate the whole file is necessary. "TBD" might indicate the expert didn't look into the file.
      "WHY_CODE": "string",         // The reason to include this file in expert's output, one of: TOCHANGE, DEFINITIONS, HIGHLEV, USERCODE.
      "WHY_DESC": "string",         // Description why this file matters wrt the task.
      "RELEVANCY": 0                // Expert's own evaluation of their results, 1 to 5. 1 = this file doesn't even exist, 3 = might provide good insight into the logic behind the program but not directly relevant, 5 = exactly what is needed.
    }
  ],
  ...
}

Experts can make mistakes. Your role is to reduce their noisy output into a single more reliable output.
"###;


const RF_REDUCE_USER_MSG: &str = r###"
1. Look at expert outputs above.
2. Write down a couple of interpretations of the task, something like "Interpretation 1: user wants to do this, and the best set of files to start this change is file1 file2 file3".
3. Decide which interpretation is most likely correct.
4. Go ahead and formalize that interpretation in the following JSON format, write "REDUCE_OUTPUT", continue with triple backquotes.

REDUCE_OUTPUT
```
{
  "dir/dir/file.ext": {
    "SYMBOLS": "symbol1,symbol2",     // Comma-separated list of functions/classes/types/variables/etc defined within this file that are actually relevant. List all symbols that are relevant, not just some of them.  Use your own judgement, don't just copy from an expert.
    "WHY_CODE": "string",             // Write down the reason to include this file in output, pick one of: TOCHANGE, DEFINITIONS, HIGHLEV, USERCODE. Use your own judgement, don't just copy from an expert.
    "WHY_DESC": "string",             // Describe why this file matters wrt the task, what's going on inside? Copy the best explanation from an expert.
    "RELEVANCY": 0                    // Critically evaluate how is this file really relevant to your interpretation of the task. Rate from 1 to 5. 1 = has TBD, role is unclear, 3 = might provide good insight into the logic behind the program but not directly relevant, 5 = exactly what is needed.
  }
}
```
"###;

fn parse_reduce_output(content: &str) -> Result<Value, String> {
    // Step 1: Extract the JSON content
    let re = Regex::new(r"(?s)REDUCE_OUTPUT\s*```(?:json)?\s*(.+?)\s*```").unwrap();
    let json_str = re.captures(content)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim())
        .ok_or_else(|| "Unable to find REDUCE_OUTPUT section :/".to_string())?;
    let output: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Unable to parse JSON: {:?}", e))?;
    Ok(output)
}


#[derive(Serialize, Deserialize, Debug)]
struct ReduceFileItem {
    #[serde(rename = "FILE_PATH")]
    file_path: String,
    #[serde(rename = "SYMBOLS")]
    symbols: String,
    #[serde(rename = "WHY_CODE")]
    why_code: String,
    #[serde(rename = "WHY_DESC")]
    why_desc: String,
    #[serde(rename = "RELEVANCY")]
    relevancy: u8,
}
