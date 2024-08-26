use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use regex::Regex;

use async_trait::async_trait;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use futures_util::future::join_all;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_tools::subchat::subchat;
use crate::at_tools::tools::Tool;
use crate::call_validation::{ChatMessage, ContextEnum, SubchatParameters};
use crate::global_context::GlobalContext;


const RF_OUTPUT_FILES: usize = 6;
const RF_ATTEMPTS: usize = 1;


pub struct AttRelevantFiles;

#[async_trait]
impl Tool for AttRelevantFiles {
    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<Vec<ContextEnum>, String> {
        let problem_statement = match args.get("problem_statement") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => return Err(format!("argument `problem_statement` is not a string: {:?}", v)),
            None => return Err("Missing argument `problem_statement`".to_string())
        };

        let params = crate::at_tools::execute_att::unwrap_subchat_params(ccx.clone(), "locate").await?;
        let ccx_subchat = {
            let ccx_lock = ccx.lock().await;
            let mut t = AtCommandsContext::new(
                ccx_lock.global_context.clone(),
                params.subchat_n_ctx,
                30,
                false,
                ccx_lock.messages.clone(),
            ).await;
            t.subchat_tx = ccx_lock.subchat_tx.clone();
            t.subchat_rx = ccx_lock.subchat_rx.clone();
            Arc::new(AMutex::new(t))
        };

        let res = find_relevant_files(
            ccx_subchat,
            params,
            tool_call_id.clone(),
            problem_statement,
        ).await?;

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

Potential strategies:

TREEGUESS = call tree(), spot up to 20 suspicious files just by looking at file names.

GOTODEF = call definition("xxx", skeleton=true) in parallel for symbols either visible in task description, or symbols you can guess; don't call definition() for symbols from standard libraries, only
symbols within the project are indexed. It's fine, or even desirable to have a second round of parallel definition("xxx", skeleton=true) calls, but only for symbols that are actually relevant for
the task. If the strategy doesn't work, just give up and save time.

VECDBSEARCH = search() can find semantically similar code, text in comments, and sometimes documentation.

CUSTOM = a different strategy that makes sense for the task at hand, try something dissimilar to the strategies described.

You'll receive additional instructions that start with 💿. Those are not coming from the user, they are programmed to help you operate
well between chat restarts and they are always in English. Answer in the language the user prefers.

EXPLAIN YOUR ACTIONS BEFORE CALLING ANY FUNCTIONS. IT'S FORBIDDEN TO CALL TOOLS UNTIL YOU EXPLAINED WHAT YOU ARE GOING TO DO.
"###;

const RF_EXPERT_PLEASE_WRAP_UP: &str = r###"You are out of turns or tokens for this chat. Now you need to save your progress, such that a new chat can pick up from there. Use this structure:
{
    "OUTPUT": {                       // The output is dict<filename, info_dict>.
        "dir/dir/file.ext": {           // Here you need a strict absolute path with no ambiguity at all.
            "SYMBOLS": "symbol1,symbol2", // Comma-separated list of functions/classes/types/variables/etc defined within this file that are actually relevant, for example "MyClass::my_function". List all symbols that are relevant, not just some of them. Write "*" to indicate the whole file is necessary. Write "TBD" to indicate you didn't look inside yet.
            "WHY_CODE": "string",         // Write down the reason to include this file in output, pick one of: TOCHANGE, DEFINITIONS, HIGHLEV, USERCODE. Put TBD if you didn't look inside.
            "WHY_DESC": "string",         // Describe why this file matters wrt the task, what's going on inside? Put TBD if you didn't look inside.
            "RELEVANCY": 0                // Critically evaluate how is this file really relevant to the task. Rate from 1 to 5. 1 = no evidence this file even exists, 2 = file exists but you didn't look inside, 3 = might provide good insight into the logic behind the program but not directly relevant, 5 = exactly what is needed.
        }
    ],
}
"###;

const RF_REDUCE_SYSTEM_PROMPT: &str = r###"
You will receive output generated by experts using different strategies. They will give you this format:

{
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

You'll receive additional instructions that start with 💿. Those are not coming from the user, they are programmed to help you operate
well between chat restarts and they are always in English. Answer in the language the user prefers.
"###;


const RF_REDUCE_USER_MSG: &str = r###"
💿 Call cat() once with all the files and symbols. You have enough tokens for one big call, don't worry. Don't call anything else.
"###;

const RF_REDUCE_WRAP_UP: &str = r###"
Look at the expert outputs above, think step by step. Follow this plan:

1. Write down a couple of interpretations of the original task, something like "Interpretation 1: user wants to do this, and the best place to start this change is at file1.ext, near my_function1, in my_function2".

2. Decide which interpretation is most likely correct.

3. Decide which one or two files will receive the most meaningful updates if the user was to change the code in that interpretation. You'll need to label them TOCHANGE later.

4. Write down which files might support the change, some of them contain high-level logic, some have definitions.

5. Experts make mistakes; take their RELEVANCY ratings critically, and write your own by looking at the actual code and the best interpretation.
All the files cannot have relevancy 5; most of them are likely 3, "might provide good insight into the logic behind the program but not directly relevant", but you can
write 1 or 2 if you accidentally wrote a file name and changed your mind about how useful it is, not a problem.

6. After you have completed 1-5, go ahead and formalize your best interpretation in the following JSON format, write "REDUCE_OUTPUT", and continue with triple backquotes.

REDUCE_OUTPUT
```
{
    "dir/dir/file.ext": {
        "SYMBOLS": "symbol1,symbol2",     // Comma-separated list of functions/classes/types/variables/etc defined within this file that are actually relevant. List all symbols that are relevant, not just some of them. Use your own judgement, don't just copy from an expert.
        "WHY_CODE": "string",             // Write down the reason to include this file in output, pick one of: TOCHANGE, DEFINITIONS, HIGHLEV, USERCODE. Use your own judgement, don't just copy from an expert.
        "WHY_DESC": "string",             // Describe why this file matters wrt the task, what's going on inside? Describe the file in general in a sentense or two, and then describe what specifically is the relation to the task.
        "RELEVANCY": 0                    // Critically evaluate how is this file really relevant to your interpretation of the task. Rate from 1 to 5. 1 = has TBD, role is unclear, 3 = might provide good insight into the logic behind the program but not directly relevant, 5 = exactly what is needed.
    }
}
```
"###;

fn parse_reduce_output(content: &str) -> Result<Value, String> {
    let re = Regex::new(r"(?s)REDUCE_OUTPUT\s*```(?:json)?\s*(.+?)\s*```").unwrap();
    let json_str = re.captures(content)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().trim())
        .ok_or_else(|| {
            tracing::warn!("Unable to find REDUCE_OUTPUT section:\n{}", content);
            "Unable to find REDUCE_OUTPUT section".to_string()
        })?;
    let output: Value = serde_json::from_str(json_str).map_err(|e| {
            tracing::warn!("Unable to parse JSON:\n{}({})", json_str, e);
            format!("Unable to parse JSON: {:?}", e)
        })?;
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


async fn find_relevant_files(
    ccx: Arc<AMutex<AtCommandsContext>>,
    subchat_params: SubchatParameters,
    tool_call_id: String,
    user_query: String,
) -> Result<Value, String> {
    let gcx: Arc<ARwLock<GlobalContext>> = ccx.lock().await.global_context.clone();
    let _vecdb_on = {
        let gcx = gcx.read().await;
        let vecdb = gcx.vec_db.lock().await;
        vecdb.is_some()
    };

    let sys = RF_SYSTEM_PROMPT
        .replace("{ATTEMPTS}", &format!("{RF_ATTEMPTS}"))
        .replace("{RF_OUTPUT_FILES}", &format!("{RF_OUTPUT_FILES}"));
    let log_prefix = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();

    let mut strategy_messages = vec![];
    strategy_messages.push(ChatMessage::new("system".to_string(), sys.to_string()));
    strategy_messages.push(ChatMessage::new("user".to_string(), user_query.to_string()));

    let tools_subset = vec!["definition", "references", "tree", "cat"].iter().map(|x|x.to_string()).collect::<Vec<_>>();
    let mut futures = vec![];

    let mut strategy_tree = strategy_messages.clone();
    strategy_tree.push(crate::at_tools::att_locate::pretend_tool_call("tree", "{}", "I'll use TREEGUESS strategy, to do that I need to start with a tree() call.".to_string()));
    futures.push(subchat(
        ccx.clone(),
        subchat_params.subchat_model.as_str(),
        strategy_tree,
        tools_subset.clone(),
        0,
        subchat_params.subchat_max_new_tokens,
        RF_EXPERT_PLEASE_WRAP_UP,
        4,
        Some(0.8),
        Some(format!("{log_prefix}-rf-step1-treeguess")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-rf-step1-treeguess")),
    ));

    let mut strategy_gotodef = strategy_messages.clone();
    strategy_gotodef.push(ChatMessage::new("user".to_string(), "💿 Use GOTODEF strategy.".to_string()));
    futures.push(subchat(
        ccx.clone(),
        subchat_params.subchat_model.as_str(),
        strategy_gotodef,
        vec!["definition", "references", "cat"].iter().map(|x|x.to_string()).collect::<Vec<_>>(),
        5,
        subchat_params.subchat_max_new_tokens,
        RF_EXPERT_PLEASE_WRAP_UP,
        1,
        Some(0.2),
        Some(format!("{log_prefix}-rf-step1-gotodef")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-rf-step1-gotodef")),
    ));

    let results: Vec<Vec<Vec<ChatMessage>>> = join_all(futures).await.into_iter().filter_map(|x| x.ok()).collect();
    let only_last_messages: Vec<ChatMessage> = results.into_iter()
        .flat_map(|choices| {
            choices.into_iter().filter_map(|mut messages| {
                messages.pop().filter(|msg| msg.role == "assistant")
            })
        })
        .collect();

    // Reduce
    let mut messages = vec![];
    messages.push(ChatMessage::new("system".to_string(), RF_REDUCE_SYSTEM_PROMPT.to_string()));
    messages.push(ChatMessage::new("user".to_string(), format!("User provided task:\n\n{}", user_query)));
    for (i, expert_message) in only_last_messages.into_iter().enumerate() {
        messages.push(ChatMessage::new("user".to_string(), format!("Expert {} says:\n\n{}", i + 1, expert_message.content)));
    }
    messages.push(ChatMessage::new("user".to_string(), format!("{}", RF_REDUCE_USER_MSG)));
    let result = subchat(
        ccx.clone(),
        subchat_params.subchat_model.as_str(),
        messages,
        vec!["cat".to_string()],
        1,
        subchat_params.subchat_max_new_tokens,
        RF_REDUCE_WRAP_UP,
        1,
        Some(0.0),
        Some(format!("{log_prefix}-rf-step2-reduce")),
        Some(tool_call_id.clone()),
        Some(format!("{log_prefix}-rf-step2-reduce")),
    ).await?[0].clone();

    let answer = parse_reduce_output(&result.last().unwrap().content)?;
    Ok(answer)
}
