use std::collections::HashMap;

use serde_json::Value;

pub struct PatchArguments {
    pub paths: Vec<String>,
    pub symbol_names: Option<Vec<String>>,
    pub todo: String,
}

pub async fn parse_arguments(
    args: &HashMap<String, Value>,
) -> Result<PatchArguments, String> {
    let paths = match args.get("paths") {
        Some(Value::String(s)) => s.split(",").map(|x| x.to_string()).collect::<Vec<String>>(),
        Some(v) => { return Err(format!("argument `paths` is not a string: {:?}", v)) }
        None => { return Err("argument `path` is not a string".to_string()) }
    };
    let symbol_names = match args.get("symbols") {
        Some(Value::String(s)) => Some(s.split(",").map(|x| x.to_string()).collect::<Vec<String>>()),
        Some(v) => { return Err(format!("argument `symbols` is not a string: {:?}", v)) }
        None => None
    };
    let todo = match args.get("todo") {
        Some(Value::String(s)) => s.clone(),
        Some(v) => { return Err(format!("argument `todo` is not a string: {:?}", v)) }
        None => { "".to_string() }
    };
    Ok(PatchArguments {
        paths,
        symbol_names,
        todo,
    })
}