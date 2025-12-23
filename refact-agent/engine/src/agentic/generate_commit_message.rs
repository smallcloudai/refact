use std::path::PathBuf;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::files_correction::CommandSimplifiedDirExt;
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::subchat::subchat_single;
use std::sync::Arc;
use hashbrown::HashMap;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tracing::warn;
use crate::files_in_workspace::detect_vcs_for_a_file_path;

const DIFF_ONLY_PROMPT: &str = r#"Generate a commit message following the Conventional Commits specification.

# Conventional Commits Format

```
<type>(<optional scope>): <description>

[optional body]

[optional footer(s)]
```

## Commit Types (REQUIRED - choose exactly one)
- `feat`: New feature (correlates with MINOR in SemVer)
- `fix`: Bug fix (correlates with PATCH in SemVer)
- `refactor`: Code restructuring without changing behavior
- `perf`: Performance improvement
- `docs`: Documentation only changes
- `style`: Code style changes (formatting, whitespace, semicolons)
- `test`: Adding or correcting tests
- `build`: Changes to build system or dependencies
- `ci`: Changes to CI configuration
- `chore`: Maintenance tasks (tooling, configs, no production code change)
- `revert`: Reverting a previous commit

## Rules

### Subject Line (REQUIRED)
1. Format: `<type>(<scope>): <description>` or `<type>: <description>`
2. Use imperative mood ("add" not "added" or "adds")
3. Do NOT capitalize the first letter of description
4. Do NOT end with a period
5. Keep under 50 characters (hard limit: 72)
6. Scope is optional but recommended for larger projects

### Body (OPTIONAL - use for complex changes)
1. Separate from subject with a blank line
2. Wrap at 72 characters
3. Explain WHAT and WHY, not HOW
4. Use bullet points for multiple items

### Footer (OPTIONAL)
1. Reference issues: `Fixes #123`, `Closes #456`, `Refs #789`
2. Breaking changes: Start with `BREAKING CHANGE:` or add `!` after type
3. Co-authors: `Co-authored-by: Name <email>`

## Breaking Changes
- Add `!` after type/scope: `feat!:` or `feat(api)!:`
- Or include `BREAKING CHANGE:` footer with explanation

# Steps

1. Analyze the diff to understand what changed
2. Determine the PRIMARY type of change (feat, fix, refactor, etc.)
3. Identify scope from affected files/modules (optional)
4. Write description in imperative mood explaining the intent
5. Add body only if the change is complex and needs explanation
6. Add footer for issue references or breaking changes if applicable

# Examples

**Input (diff)**:
```diff
- public class UserManager {
-     private final UserDAO userDAO;
+ public class UserManager {
+     private final UserService userService;
+     private final NotificationService notificationService;
```

**Output**:
```
refactor(user): replace UserDAO with service-based architecture

Introduce UserService and NotificationService to improve separation of
concerns and make user management logic more reusable.
```

**Input (diff)**:
```diff
- if (age > 17) {
-     accessAllowed = true;
- } else {
-     accessAllowed = false;
- }
+ accessAllowed = age > 17;
```

**Output**:
```
refactor: simplify age check with ternary expression
```

**Input (diff)**:
```diff
+ export async function fetchUserProfile(userId: string) {
+   const response = await api.get(`/users/${userId}`);
+   return response.data;
+ }
```

**Output**:
```
feat(api): add user profile fetch endpoint
```

**Input (diff)**:
```diff
- const timeout = 5000;
+ const timeout = 30000;
```

**Output**:
```
fix(database): increase query timeout to prevent failures

Extend timeout from 5s to 30s to resolve query failures during peak load.

Fixes #234
```

**Input (breaking change)**:
```diff
- function getUser(id) { return users[id]; }
+ function getUser(id) { return { user: users[id], metadata: {} }; }
```

**Output**:
```
feat(api)!: wrap user response in object with metadata

BREAKING CHANGE: getUser() now returns { user, metadata } instead of
user directly. Update all callers to access .user property.
```

# Important Guidelines

- Choose the MOST significant type if changes span multiple categories
- Be specific in the description - avoid vague terms like "update", "fix stuff"
- The subject should complete: "If applied, this commit will <description>"
- One commit = one logical change (if diff has unrelated changes, note it)
- Scope should reflect the module, component, or area affected"#;

const DIFF_WITH_USERS_TEXT_PROMPT: &str = r#"Generate a commit message following Conventional Commits, using the user's input as context for intent.

# Conventional Commits Format

```
<type>(<optional scope>): <description>

[optional body]

[optional footer(s)]
```

## Commit Types (REQUIRED - choose exactly one)
- `feat`: New feature (correlates with MINOR in SemVer)
- `fix`: Bug fix (correlates with PATCH in SemVer)
- `refactor`: Code restructuring without changing behavior
- `perf`: Performance improvement
- `docs`: Documentation only changes
- `style`: Code style changes (formatting, whitespace, semicolons)
- `test`: Adding or correcting tests
- `build`: Changes to build system or dependencies
- `ci`: Changes to CI configuration
- `chore`: Maintenance tasks (tooling, configs, no production code change)
- `revert`: Reverting a previous commit

## Rules

### Subject Line (REQUIRED)
1. Format: `<type>(<scope>): <description>` or `<type>: <description>`
2. Use imperative mood ("add" not "added" or "adds")
3. Do NOT capitalize the first letter of description
4. Do NOT end with a period
5. Keep under 50 characters (hard limit: 72)
6. Scope is optional but recommended for larger projects

### Body (OPTIONAL - use for complex changes)
1. Separate from subject with a blank line
2. Wrap at 72 characters
3. Explain WHAT and WHY, not HOW
4. Use bullet points for multiple items

### Footer (OPTIONAL)
1. Reference issues: `Fixes #123`, `Closes #456`, `Refs #789`
2. Breaking changes: Start with `BREAKING CHANGE:` or add `!` after type
3. Co-authors: `Co-authored-by: Name <email>`

## Breaking Changes
- Add `!` after type/scope: `feat!:` or `feat(api)!:`
- Or include `BREAKING CHANGE:` footer with explanation

# Steps

1. Analyze the user's initial commit message to understand their intent
2. Analyze the diff to understand the actual changes
3. Determine the correct type based on the nature of changes
4. Extract or infer a scope from user input or affected files
5. Synthesize user intent + diff analysis into a proper conventional commit
6. If user mentions an issue number, include it in the footer

# Examples

**Input (user's message)**:
```
fix the login bug
```

**Input (diff)**:
```diff
- if (user.password === input) {
+ if (await bcrypt.compare(input, user.passwordHash)) {
```

**Output**:
```
fix(auth): use bcrypt for secure password comparison

Replace plaintext password comparison with bcrypt hash verification
to fix authentication vulnerability.
```

**Input (user's message)**:
```
Refactor UserManager to use services instead of DAOs
```

**Input (diff)**:
```diff
- public class UserManager {
-     private final UserDAO userDAO;
+ public class UserManager {
+     private final UserService userService;
+     private final NotificationService notificationService;
```

**Output**:
```
refactor(user): replace UserDAO with service-based architecture

Introduce UserService and NotificationService to improve separation of
concerns and make user management logic more reusable.
```

**Input (user's message)**:
```
added new endpoint for users #123
```

**Input (diff)**:
```diff
+ @GetMapping("/users/{id}/preferences")
+ public ResponseEntity<Preferences> getUserPreferences(@PathVariable Long id) {
+     return ResponseEntity.ok(userService.getPreferences(id));
+ }
```

**Output**:
```
feat(api): add user preferences endpoint

Refs #123
```

**Input (user's message)**:
```
cleanup
```

**Input (diff)**:
```diff
- // TODO: implement later
- // console.log("debug");
- const unusedVar = 42;
```

**Output**:
```
chore: remove dead code and debug artifacts
```

**Input (user's message)**:
```
BREAKING: change API response format
```

**Input (diff)**:
```diff
- return user;
+ return { data: user, version: "2.0" };
```

**Output**:
```
feat(api)!: wrap responses in versioned data envelope

BREAKING CHANGE: All API responses now return { data, version } object
instead of raw data. Clients must access response.data for the payload.
```

# Important Guidelines

- Preserve the user's intent but format it correctly
- If user mentions "bug", "fix", "broken" → likely `fix`
- If user mentions "add", "new", "feature" → likely `feat`
- If user mentions "refactor", "restructure", "reorganize" → `refactor`
- If user mentions "clean", "remove unused" → likely `chore` or `refactor`
- Extract issue numbers (#123) from user text and move to footer
- The subject should complete: "If applied, this commit will <description>"
- Don't just paraphrase the user - analyze the diff to add specificity"#;
const N_CTX: usize = 32000;
const TEMPERATURE: f32 = 0.5;

pub fn remove_fencing(message: &String) -> Vec<String> {
    let trimmed_message = message.trim();
    if !trimmed_message.contains("```") {
        return Vec::new();
    }
    if trimmed_message.contains("``````") {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut in_code_block = false;

    for (_i, part) in trimmed_message.split("```").enumerate() {
        if in_code_block {
            let part_lines: Vec<&str> = part.lines().collect();
            if !part_lines.is_empty() {
                let start_idx = if part_lines[0].trim().split_whitespace().count() <= 1 && part_lines.len() > 1 {
                    1
                } else {
                    0
                };
                if start_idx < part_lines.len() {
                    let code_block = part_lines[start_idx..].join("\n");
                    if !code_block.is_empty() {
                        results.push(code_block.trim().to_string());
                    }
                }
            }
        }

        in_code_block = !in_code_block;
    }
    if !results.is_empty() {
        results
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_fencing() {
        let input = "Simple text without fencing".to_string();
        assert_eq!(remove_fencing(&input), Vec::<String>::new());
    }

    #[test]
    fn test_simple_fencing() {
        let input = "```\nCode block\n```".to_string();
        assert_eq!(remove_fencing(&input), vec!["Code block".to_string()]);
    }

    #[test]
    fn test_language_tag() {
        let input = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```".to_string();
        assert_eq!(remove_fencing(&input), vec!["fn main() {\n    println!(\"Hello\");\n}".to_string()]);
    }

    #[test]
    fn test_text_before_and_after() {
        let input = "Text before\nText before\n```\nCode block\n```\nText after".to_string();
        assert_eq!(remove_fencing(&input), vec!["Code block".to_string()]);
    }

    #[test]
    fn test_multiple_code_blocks() {
        let input = "First paragraph\n```\nFirst code\n```\nMiddle text\n```python\ndef hello():\n    print('world')\n```\nLast paragraph".to_string();
        assert_eq!(remove_fencing(&input), vec!["First code".to_string(), "def hello():\n    print('world')".to_string()]);
    }

    #[test]
    fn test_empty_code_block() {
        let input = "Text with `````` empty block".to_string();
        assert_eq!(remove_fencing(&input), Vec::<String>::new());
    }
}

pub async fn generate_commit_message_by_diff(
    gcx: Arc<ARwLock<GlobalContext>>,
    diff: &String,
    commit_message_prompt: &Option<String>,
) -> Result<String, String> {
    if diff.is_empty() {
        return Err("The provided diff is empty".to_string());
    }
    let messages = if let Some(text) = commit_message_prompt {
        vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::SimpleText(DIFF_WITH_USERS_TEXT_PROMPT.to_string()),
                ..Default::default()
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::SimpleText(format!(
                    "Commit message:\n```\n{}\n```\nDiff:\n```\n{}\n```\n",
                    text, diff
                )),
                ..Default::default()
            },
        ]
    } else {
        vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::SimpleText(DIFF_ONLY_PROMPT.to_string()),
                ..Default::default()
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::SimpleText(format!("Diff:\n```\n{}\n```\n", diff)),
                ..Default::default()
            },
        ]
    };
    let model_id = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => Ok(caps.defaults.chat_default_model.clone()),
        Err(_) => Err("No caps available".to_string()),
    }?;
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(AtCommandsContext::new(
        gcx.clone(),
        N_CTX,
        1,
        false,
        messages.clone(),
        "".to_string(),
        false,
        model_id.clone(),
    ).await));
    let new_messages = subchat_single(
        ccx.clone(),
        &model_id,
        messages,
        Some(vec![]),
        None,
        false,
        Some(TEMPERATURE),
        None,
        1,
        None,
        true,
        None,
        None,
        None,
    )
        .await
        .map_err(|e| format!("Error: {}", e))?;

    let commit_message = new_messages
        .into_iter()
        .next()
        .map(|x| {
            x.into_iter().last().map(|last_m| match last_m.content {
                ChatContent::SimpleText(text) => Some(text),
                ChatContent::Multimodal(_) => None,
            })
        })
        .flatten()
        .flatten()
        .ok_or("No commit message was generated".to_string())?;

    let code_blocks = remove_fencing(&commit_message);
    if !code_blocks.is_empty() {
        Ok(code_blocks[0].clone())
    } else {
        Ok(commit_message)
    }
}

pub async fn _generate_commit_message_for_projects(
    gcx: Arc<ARwLock<GlobalContext>>,
) -> Result<HashMap<PathBuf, String>, String> {
    let project_folders = gcx.read().await.documents_state.workspace_folders.lock().unwrap().clone();
    let mut commit_messages = HashMap::new();

    for folder in project_folders {
        let command = if let Some((_, vcs_type)) = detect_vcs_for_a_file_path(&folder).await {
            match vcs_type {
                "git" => "git diff",
                "svn" => "svn diff",
                "hg" => "hg diff",
                other => {
                    warn!("Unrecognizable version control detected for the folder {folder:?}: {other}");
                    continue;
                }
            }
        } else {
            warn!("There's no recognizable version control detected for the folder {folder:?}");
            continue;
        };

        let output = tokio::process::Command::new(command)
            .current_dir_simplified(&folder)
            .stdin(std::process::Stdio::null())
            .output()
            .await
            .map_err(|e| format!("Failed to execute command for folder {folder:?}: {e}"))?;

        if !output.status.success() {
            warn!("Command failed for folder {folder:?}: {}", String::from_utf8_lossy(&output.stderr));
            continue;
        }

        let diff_output = String::from_utf8_lossy(&output.stdout).to_string();
        let commit_message = generate_commit_message_by_diff(gcx.clone(), &diff_output, &None).await?;
        commit_messages.insert(folder, commit_message);
    }

    Ok(commit_messages)
}