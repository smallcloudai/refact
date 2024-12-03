use std::sync::Arc;
use tokio::sync::Mutex as AMutex;
use axum::Extension;
use axum::http::{Response, StatusCode};
use hyper::Body;
use serde::Deserialize;
use tokio::sync::RwLock as ARwLock;
use crate::subchat::subchat_single;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ChatContent, ChatMessage};
use crate::custom_error::ScratchError;
use crate::global_context::{try_load_caps_quickly_if_not_present, GlobalContext};


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


fn remove_fencing(message: &String) -> String {
    let trimmed_message = message.trim();
    let without_leading_fence = if trimmed_message.starts_with("```") {
        let mut lines = trimmed_message.lines();
        lines.next();
        lines.collect::<Vec<&str>>().join("\n")
    } else {
        trimmed_message.to_string()
    };
    let without_trailing_fence = if without_leading_fence.ends_with("```") {
        let mut lines = without_leading_fence.lines().collect::<Vec<&str>>();
        lines.pop();
        lines.join("\n")
    } else {
        without_leading_fence
    };
    without_trailing_fence.trim().to_string()
}


#[derive(Deserialize)]
struct CommitMessageFromDiffPost {
    diff: String,
    #[serde(default)]
    text: Option<String>  // a prompt for the commit message 
}

pub async fn handle_v1_commit_message_from_diff(
    Extension(global_context): Extension<Arc<ARwLock<GlobalContext>>>,
    body_bytes: hyper::body::Bytes,
) -> axum::response::Result<Response<Body>, ScratchError> {
    let post = serde_json::from_slice::<CommitMessageFromDiffPost>(&body_bytes)
        .map_err(|e| ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, format!("JSON problem: {}", e)))?;
    if post.diff.is_empty() {
        return Err(ScratchError::new(StatusCode::UNPROCESSABLE_ENTITY, "The provided diff is empty".to_string()))
    }

    let messages = if let Some(text) = &post.text {
        vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::SimpleText(DIFF_WITH_USERS_TEXT_PROMPT.to_string()),
                ..Default::default()
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::SimpleText(format!("Commit message:\n```\n{}\n```\nDiff:\n```\n{}\n```\n", text, post.diff)),
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
                content: ChatContent::SimpleText(format!("Diff:\n```\n{}\n```\n", post.diff)),
                ..Default::default()
            },
        ]
    };
    let model_name = match try_load_caps_quickly_if_not_present(global_context.clone(), 0).await {
        Ok(caps) => {
            caps.read()
                .map(|x| Ok(x.code_chat_default_model.clone()))
                .map_err(|_|
                    ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "Caps are not available".to_string())
                )?
        },
        Err(_) => Err(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "No caps available".to_string()))
    }?;

    let ccx: Arc<AMutex<AtCommandsContext>> = Arc::new(AMutex::new(
        AtCommandsContext::new(
            global_context.clone(),
            N_CTX, 
            1, 
            false, 
            messages.clone(), 
            "".to_string(), 
            false
        ).await)
    );

    let new_messages = subchat_single(
        ccx.clone(),
        model_name.as_str(),
        messages,
        vec![],
        None,
        false,
        Some(TEMPERATURE),
        None,
        1,
        None,
        None,
        None,
    ).await.map_err(|e| ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)))?;

    let commit_message = new_messages
        .into_iter()
        .next()
        .map(|x| x.into_iter().last().map(|last_m| {
            match last_m.content {
                ChatContent::SimpleText(text) => Some(text),
                ChatContent::Multimodal(_) => { None }
            }
        }))
        .flatten()
        .flatten()
        .ok_or(ScratchError::new(StatusCode::INTERNAL_SERVER_ERROR, "No commit message was generated".to_string()))?;
    Ok(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(remove_fencing(&commit_message)))
            .unwrap()
    )
}
