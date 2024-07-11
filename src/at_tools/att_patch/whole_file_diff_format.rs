use crate::call_validation::DiffChunk;

pub struct WholeFileDiffFormat {}

impl WholeFileDiffFormat {
    pub fn prompt() -> String {
        r#"Act as an expert software developer.
Your task is make changes to provided files using the provided task.
To suggest changes to a file you MUST return the entire content of the updated file.
You MUST use this *file listing* format

Follow these steps in order to produce the unified diff:
1. **Analyze Tasks and Files:**
   -- Review the tasks and files provided
   -- Identify the specific changes required
   -- Use chain of thoughts to make sure nothing will be missed
   -- Assess after diff is generated, including its format validity

2. **Generate files changes:**
-- To suggest changes to a file you MUST return the entire content of the updated file.

-- You MUST use this *file listing* format:
    path/to/filename.js
    {fence[0]}
    // entire file content ...
    // ... goes in between
    {fence[1]}

-- Every *file listing* MUST use this format:
--- First line: the filename with any originally provided path
--- Second line: opening {fence[0]}
--- ... entire content of the file ...
--- Final line: closing {fence[1]}

-- To suggest changes to a file you MUST return a *file listing* that contains the entire content of the file.

-- *NEVER* skip, omit or elide content from a *file listing* using "..." or by adding comments like "... rest of code..."!

-- Create a new file you MUST return a *file listing* which includes an appropriate filename, including any appropriate path.
"#.to_string()
    }

    pub async fn parse_message(
        _: &str,
    ) -> Result<Vec<DiffChunk>, String> {
        todo!()
    }
}
