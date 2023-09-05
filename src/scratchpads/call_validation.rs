use serde::Deserialize;
use std::collections::HashMap;


#[derive(Debug, Deserialize, Clone)]
pub struct CursorPosition {
    pub file: String,
    pub line: i32,
    pub character: i32,
}


#[derive(Debug, Deserialize, Clone)]
pub struct CodeCompletionInputs {
    pub sources: HashMap<String, String>,
    pub cursor: CursorPosition,
    pub multiline: bool,
}


#[derive(Debug, Deserialize, Clone)]
pub struct CodeCompletionPost {
    pub model: String,
    pub stream: bool,
    pub inputs: CodeCompletionInputs,
}

// class SamplingParameters(BaseModel):
//     max_new_tokens: int = Query(default=50, ge=0, le=4096)
//     temperature: Optional[float] = Query(default=None, ge=0.0, le=2.0)
//     top_p: Optional[float] = Query(default=None, ge=0.5, le=1.0)
//     stop: Optional[List[str]] = Query(default=None, min_items=0, max_items=10)
