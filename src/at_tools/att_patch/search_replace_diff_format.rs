use crate::call_validation::DiffChunk;

pub struct SearchReplaceDiffFormat {}

impl SearchReplaceDiffFormat {
    pub fn prompt() -> String {
        r#"Act as an expert software developer.
Your task is to create a diff in a specific format using the provided task and all given files.
Diff format is based on *SEARCH/REPLACE* blocks.

Follow these steps in order to produce the unified diff:
1. **Analyze Tasks and Files:**
   -- Review the tasks and files provided
   -- Identify the specific changes required
   -- Use chain of thoughts to make sure nothing will be missed
   -- Assess after diff is generated, including its format validity

2. **Generate Diff:**
Every *SEARCH/REPLACE block* must use this format:
-- The opening fence and code language, eg: ```python
-- The start of search block: <<<<<<< SEARCH
-- A contiguous chunk of lines to search for in the existing source code
-- The dividing line: =======
-- The lines to replace into the source code
-- The end of the replace block: >>>>>>> REPLACE
-- The closing fence: ```
-- Every *SEARCH* section must *EXACTLY MATCH* the existing source code, character for character, including all comments, docstrings, formatting, etc.
-- *SEARCH/REPLACE* blocks will replace *all* matching occurrences.
-- Include enough lines to make the SEARCH blocks unique.
-- Include *ALL* the code being searched and replaced!
-- To move code, use 2 *SEARCH/REPLACE* blocks: 1 to delete it from its current location, 1 to insert it in the new location.
-- If you've opened *SEARCH/REPLACE block* you must close it.
-- ONLY EVER RETURN CODE IN A *SEARCH/REPLACE BLOCK*!"#.to_string()
    }


    pub async fn parse_message(
        _: &str,
    ) -> Result<Vec<DiffChunk>, String> {
        todo!()
    }
}
