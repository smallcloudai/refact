use std::path::PathBuf;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};
use crate::subchat::subchat_single;
use std::sync::Arc;
use hashbrown::HashMap;
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use tracing::warn;
use crate::files_in_workspace::detect_vcs_for_a_file_path;

const DIFF_ONLY_PROMPT: &str = r#"Analyze the given diff and generate a clear and descriptive commit message that explains the purpose of the changes. Your commit message should convey *why* the changes were made, *how* they improve the code, or what features or fixes are implemented, rather than just restating *what* the changes are. Aim for an informative, concise summary that would be easy for others to understand when reviewing the commit history.

# Steps
1. Analyze the code diff to understand the changes made.
2. Determine the functionality added or removed, and the reason for these adjustments.
3. Summarize the details of the change in an accurate and informative, yet concise way.
4. Structure the message in a way that starts with a short summary line, followed by optional details if the change is complex.

# Output Format

The output should be a single commit message in the following format:
- A **first line summarizing** the purpose of the change. This line should be concise.
- Optionally, include a **second paragraph** with *additional context* if the change is complex or otherwise needs further clarification.
  (e.g., if there's a bug fix, mention what problem was fixed and why the change works.)

# Examples

**Input (diff)**:
```diff
- public class UserManager {
-     private final UserDAO userDAO;

+ public class UserManager {
+     private final UserService userService;
+     private final NotificationService notificationService;

  public UserManager(UserDAO userDAO) {
-     this.userDAO = userDAO;
+     this.userService = new UserService();
+     this.notificationService = new NotificationService();
  }
```

**Output (commit message)**:
```
Refactor `UserManager` to use `UserService` and `NotificationService`

Replaced `UserDAO` with `UserService` and introduced `NotificationService` to improve separation of concerns and make user management logic reusable and extendable.
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

**Output (commit message)**:
```
Simplify age check logic for accessing permissions by using a single expression
```

# Notes
- Make sure the commit messages are descriptive enough to convey why the change is being made without being too verbose.
- If applicable, add `Fixes #<issue-number>` or other references to link the commit to specific tickets.
- Avoid wording: "Updated", "Modified", or "Changed" without explicitly stating *why*—focus on *intent*."#;

const DIFF_WITH_USERS_TEXT_PROMPT: &str = r#"Generate a commit message using the diff and the provided initial commit message as a template for context.

[Additional details as needed.]

# Steps

1. Analyze the code diff to understand the changes made.
2. Review the user's initial commit message to understand the intent and use it as a contextual starting point.
3. Determine the functionality added or removed, and the reason for these adjustments.
4. Combine insights from the diff and user's initial commit message to generate a more descriptive and complete commit message.
5. Summarize the details of the change in an accurate and informative, yet concise way.
6. Structure the message in a way that starts with a short summary line, followed by optional details if the change is complex.

# Output Format

The output should be a single commit message in the following format:
- A **first line summarizing** the purpose of the change. This line should be concise.
- Optionally, include a **second paragraph** with *additional context* if the change is complex or otherwise needs further clarification.
  (e.g., if there's a bug fix, mention what problem was fixed and why the change works.)

# Examples

**Input (initial commit message)**:
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

  public UserManager(UserDAO userDAO) {
-     this.userDAO = userDAO;
+     this.userService = new UserService();
+     this.notificationService = new NotificationService();
  }
```

**Output (commit message)**:
```
Refactor `UserManager` to use `UserService` and `NotificationService`

Replaced `UserDAO` with `UserService` and introduced `NotificationService` to improve separation of concerns and make user management logic reusable and extendable.
```

**Input (initial commit message)**:
```
Simplify age check logic
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

**Output (commit message)**:
```
Simplify age check logic for accessing permissions by using a single expression
```

# Notes
- Make sure the commit messages are descriptive enough to convey why the change is being made without being too verbose.
- If applicable, add `Fixes #<issue-number>` or other references to link the commit to specific tickets.
- Avoid wording: "Updated", "Modified", or "Changed" without explicitly stating *why*—focus on *intent*."#;
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
    let model_name = match try_load_caps_quickly_if_not_present(gcx.clone(), 0).await {
        Ok(caps) => caps
            .read()
            .map(|x| Ok(x.default_models.chat_model.clone()))
            .map_err(|_| "Caps are not available".to_string())?,
        Err(_) => Err("No caps available".to_string()),
    }?;
    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(
        AtCommandsContext::new(
            gcx.clone(),
            N_CTX,
            1,
            false,
            messages.clone(),
            "".to_string(),
            false,
        )
            .await,
    ));
    let new_messages = subchat_single(
        ccx.clone(),
        model_name.as_str(),
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
            .current_dir(&folder)
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