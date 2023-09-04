use crate::scratchpads::scratchpad_abstract::CodeCompletionScratchpad;
use crate::scratchpads::call_validation::CodeCompletionPost;

use tokenizers::Tokenizer;
use ropey::Rope;
use tracing::info;


pub struct SingleFileFIM<'a> {
    pub tokenizer: &'a Tokenizer,
    pub post: &'a CodeCompletionPost,
}

impl<'a> SingleFileFIM<'a> {
    pub fn new(
        tokenizer: &'a Tokenizer,
        post: &'a CodeCompletionPost,
    ) -> Self {
        SingleFileFIM { tokenizer, post }
    }
}

impl<'a> CodeCompletionScratchpad for SingleFileFIM<'a> {
    fn prompt(
        &self,
        context_size: usize,
    ) -> Result<String, String> {
        // TODO: assert one token
        let fim_prefix = "<fim_prefix>";
        let fim_suffix = "<fim_suffix>";
        let fim_middle = "<fim_middle>";
        // let toks = self.tokenizer.encode("hello world".to_string(), false).unwrap();
        info!("self.post.inputs.cursor.file: {}", self.post.inputs.cursor.file);
        let text_cursor_file_maybe = self.post.inputs.sources.get(&self.post.inputs.cursor.file);
        let text = match text_cursor_file_maybe {
            Some(x) => Rope::from_str(x),
            None => {
                return Err("Cursor is in file not found in sources".to_string());
            }
        };
        let mut token_count = context_size;
        let pos = &self.post.inputs.cursor;
        let mut before_iter = text.lines_at(pos.line as usize + 1).reversed();
        let mut after_iter = text.lines_at(pos.line as usize);
        let mut before_line = before_iter.next();
        let col = pos.character as usize;
        if let Some(line) = before_line {
            before_line = Some(line.slice(0..col));
        }
        let mut after_line = after_iter.next();
        if let Some(line) = after_line {
            after_line = Some(line.slice(col..));
        }
        let mut before = vec![];
        let mut after = String::new();
        while before_line.is_some() || after_line.is_some() {
            if let Some(before_line) = before_line {
                let before_line = before_line.to_string();
                let tokens = self.tokenizer
                    .encode(before_line.clone(), false)
                    .map_err(|err| {
                        return format!("Encoding error: {}", err);
                    })
                    .unwrap()
                    .len();
                if tokens > token_count {
                    break;
                }
                token_count -= tokens;
                before.push(before_line);
            }
            if let Some(after_line) = after_line {
                let after_line = after_line.to_string();
                let tokens = self.tokenizer
                    .encode(after_line.clone(), false)
                    .map_err(|err| {
                        return format!("Encoding error: {}", err);
                    })
                   .unwrap()
                   .len();
                if tokens > token_count {
                    break;
                }
                token_count -= tokens;
                after.push_str(&after_line);
            }
            before_line = before_iter.next();
            after_line = after_iter.next();
        }
        Ok(format!(
            "{}{}{}{}{}",
            fim_prefix,
            before.into_iter().rev().collect::<Vec<_>>().join(""),
            fim_suffix,
            after,
            fim_middle
        ))
    }

    fn re_stream_response(&self)
    {
    }
}
