use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AMutex;
use tokio::sync::RwLock as ARwLock;
use serde_json::Value;

use base64::{engine::general_purpose, Engine as _};

use crate::global_context::GlobalContext;
use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ContextEnum, ChatMessage, ChatContent, ChatUsage};
use crate::integrations::integr_abstract::{IntegrationCommon, IntegrationConfirmation, IntegrationTrait};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use reqwest::{Client, header};
use thiserror::Error;

const API_BASE_URL: &str = "https://api.bitbucket.org/2.0";

#[derive(Error, Debug)]
pub enum BitbucketError {
    #[error("Network error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("JSON parsing error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Deserialize, Debug)]
pub struct PullRequest {
    pub id: u64,
    pub title: String,
    pub author: Author,
    pub state: String,
    #[allow(dead_code)]
    pub created_on: String,
}

#[derive(Deserialize, Debug)]
pub struct Author {
    pub display_name: String,
}

#[derive(Deserialize, Debug)]
pub struct Paginated<T> {
    pub values: Vec<T>,
    #[allow(dead_code)]
    pub next: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct CreatePullRequest {
    pub title: String,
    pub source: Branch,
    pub destination: Branch,
}

#[derive(Serialize, Debug)]
pub struct Branch {
    pub branch: Name,
}

#[derive(Serialize, Debug)]
pub struct Name {
#[allow(dead_code)]
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Repository {
    pub slug: String,
#[allow(dead_code)]
    pub name: String,
    pub description: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct File {
    pub path: String,
    pub size: u64,
    pub attributes: Vec<String>,
}

#[derive(Clone)]
pub struct BitbucketClient {
    client: Client,
    workspace: String,
}

impl BitbucketClient {
    pub fn new(username: &str, token: &str, workspace: &str) -> Result<Self, BitbucketError> {
        let mut headers = header::HeaderMap::new();
        let auth_value = format!("Basic {}", general_purpose::STANDARD.encode(format!("{}:{}", username, token)));
        headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&auth_value).unwrap());
        
        let client = Client::builder()
            .default_headers(headers)
            .build()?;
            
        Ok(Self {
            client,
            workspace: workspace.to_string(),
        })
    }

    pub async fn get_pull_requests(&self, repo_slug: &str) -> Result<Vec<PullRequest>, BitbucketError> {
        let url = format!("{}/repositories/{}/{}/pullrequests", API_BASE_URL, self.workspace, repo_slug);
        let response = self.client.get(&url).send().await?;
        if response.status().is_success() {
            let prs = response.json::<Paginated<PullRequest>>().await?.values;
            Ok(prs)
        } else {
            Err(BitbucketError::Api(response.text().await?))
        }
    }

    pub async fn get_pull_request(&self, repo_slug: &str, pr_id: u64) -> Result<PullRequest, BitbucketError> {
        let url = format!("{}/repositories/{}/{}/pullrequests/{}", API_BASE_URL, self.workspace, repo_slug, pr_id);
        let response = self.client.get(&url).send().await?;
        if response.status().is_success() {
            let pr = response.json::<PullRequest>().await?;
            Ok(pr)
        } else {
            Err(BitbucketError::Api(response.text().await?))
        }
    }

    pub async fn create_pull_request(&self, repo_slug: &str, pr: CreatePullRequest) -> Result<PullRequest, BitbucketError> {
        let url = format!("{}/repositories/{}/{}/pullrequests", API_BASE_URL, self.workspace, repo_slug);
        let response = self.client.post(&url).json(&pr).send().await?;
        if response.status().is_success() {
            let pr = response.json::<PullRequest>().await?;
            Ok(pr)
        } else {
            Err(BitbucketError::Api(response.text().await?))
        }
    }

    pub async fn get_repositories(&self) -> Result<Vec<Repository>, BitbucketError> {
        let url = format!("{}/repositories/{}", API_BASE_URL, self.workspace);
        let response = self.client.get(&url).send().await?;
        if response.status().is_success() {
            let repos = response.json::<Paginated<Repository>>().await?.values;
            Ok(repos)
        } else {
            Err(BitbucketError::Api(response.text().await?))
        }
    }

    pub async fn get_file(&self, repo_slug: &str, commit: &str, path: &str) -> Result<String, BitbucketError> {
        let url = format!("{}/repositories/{}/{}/src/{}/{}", API_BASE_URL, self.workspace, repo_slug, commit, path);
        let response = self.client.get(&url).send().await?;
        if response.status().is_success() {
            let content = response.text().await?;
            Ok(content)
        } else {
            Err(BitbucketError::Api(response.text().await?))
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct SettingsBitbucket {
    pub bitbucket_token: String,
    pub bitbucket_username: String,
    pub bitbucket_workspace: String,
}

#[derive(Default)]
pub struct ToolBitbucket {
    pub common: IntegrationCommon,
    pub settings_bitbucket: SettingsBitbucket,
    pub config_path: String,
}

#[async_trait]
impl IntegrationTrait for ToolBitbucket {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn integr_settings_apply(&mut self, _gcx: Arc<ARwLock<GlobalContext>>, config_path: String, value: &serde_json::Value) -> Result<(), serde_json::Error> {
        self.settings_bitbucket = serde_json::from_value(value.clone())?;
        self.common = serde_json::from_value(value.clone())?;
        self.config_path = config_path;
        Ok(())
    }

    fn integr_settings_as_json(&self) -> Value {
        serde_json::to_value(&self.settings_bitbucket).unwrap_or_default()
    }

    fn integr_common(&self) -> IntegrationCommon {
        self.common.clone()
    }

    async fn integr_tools(&self, _integr_name: &str) -> Vec<Box<dyn crate::tools::tools_description::Tool + Send>> {
        vec![Box::new(ToolBitbucket {
            common: self.common.clone(),
            settings_bitbucket: self.settings_bitbucket.clone(),
            config_path: self.config_path.clone(),
        })]
    }

    fn integr_schema(&self) -> &str { BITBUCKET_INTEGRATION_SCHEMA }
}

#[async_trait]
impl Tool for ToolBitbucket {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "bitbucket".to_string(),
            display_name: "Bitbucket".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Integration,
                config_path: self.config_path.clone(),
            },
            agentic: true,
            experimental: false,
            description: "Access to Bitbucket API, to fetch issues, review PRs.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "repo_slug".to_string(),
                    param_type: "string".to_string(),
                    description: "The repository slug.".to_string(),
                },
                ToolParam {
                    name: "command".to_string(),
                    param_type: "string".to_string(),
                    description: "Examples:\n`list_prs`\n`get_pr --id 123`".to_string(),
                }
            ],
            parameters_required: vec!["repo_slug".to_string(), "command".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let repo_slug = match args.get("repo_slug") {
            Some(Value::String(s)) => s,
            _ => return Err("Missing or invalid `repo_slug` argument".to_string()),
        };
        let command = match args.get("command") {
            Some(Value::String(s)) => s,
            _ => return Err("Missing or invalid `command` argument".to_string()),
        };

        let client = BitbucketClient::new(
            &self.settings_bitbucket.bitbucket_username,
            &self.settings_bitbucket.bitbucket_token,
            &self.settings_bitbucket.bitbucket_workspace,
        ).map_err(|e| e.to_string())?;

        let content = match command.as_str() {
            "list_prs" => {
                let prs = client.get_pull_requests(repo_slug).await.map_err(|e| e.to_string())?;
                let mut pr_list = String::new();
                for pr in prs {
                    pr_list.push_str(&format!(
                        "#{} {}: {} (by {})\n",
                        pr.id, pr.title, pr.state, pr.author.display_name
                    ));
                }
                pr_list
            }
            "get_pr" => {
                let pr_id = match args.get("id") {
                    Some(Value::Number(n)) => n.as_u64().unwrap(),
                    _ => return Err("Missing or invalid `id` argument".to_string()),
                };
                let pr = client.get_pull_request(repo_slug, pr_id).await.map_err(|e| e.to_string())?;
                format!(
                    "#{} {}: {} (by {})\n",
                    pr.id, pr.title, pr.state, pr.author.display_name
                )
            }
            "create_pr" => {
                let title = match args.get("title") {
                    Some(Value::String(s)) => s,
                    _ => return Err("Missing or invalid `title` argument".to_string()),
                };
                let source_branch = match args.get("source_branch") {
                    Some(Value::String(s)) => s,
                    _ => return Err("Missing or invalid `source_branch` argument".to_string()),
                };
                let destination_branch = match args.get("destination_branch") {
                    Some(Value::String(s)) => s,
                    _ => return Err("Missing or invalid `destination_branch` argument".to_string()),
                };
                let pr = CreatePullRequest {
                    title: title.to_string(),
                    source: Branch {
                        branch: Name {
                            name: source_branch.to_string(),
                        },
                    },
                    destination: Branch {
                        branch: Name {
                            name: destination_branch.to_string(),
                        },
                    },
                };
                let new_pr = client.create_pull_request(repo_slug, pr).await.map_err(|e| e.to_string())?;
                format!("Created PR #{}: {}", new_pr.id, new_pr.title)
            }
            "list_repos" => {
                let repos = client.get_repositories().await.map_err(|e| e.to_string())?;
                let mut repo_list = String::new();
                for repo in repos {
                    repo_list.push_str(&format!(
                        "{}: {}\n",
                        repo.slug,
                        repo.description.unwrap_or_default()
                    ));
                }
                repo_list
            }
            "get_file" => {
                let commit = match args.get("commit") {
                    Some(Value::String(s)) => s,
                    _ => return Err("Missing or invalid `commit` argument".to_string()),
                };
                let path = match args.get("path") {
                    Some(Value::String(s)) => s,
                    _ => return Err("Missing or invalid `path` argument".to_string()),
                };
                client.get_file(repo_slug, commit, path).await.map_err(|e| e.to_string())?
            }
            _ => format!("Unknown command: {}", command),
        };

        Ok((false, vec![ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: ChatContent::SimpleText(content),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        })]))
    }

    async fn command_to_match_against_confirm_deny(
        &self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        _args: &HashMap<String, Value>,
    ) -> Result<String, String> {
        Ok("".to_string())
    }

    fn tool_depends_on(&self) -> Vec<String> {
        vec![]
    }

    fn usage(&mut self) -> &mut Option<ChatUsage> {
        static mut DEFAULT_USAGE: Option<ChatUsage> = None;
        #[allow(static_mut_refs)]
        unsafe { &mut DEFAULT_USAGE }
    }

    fn confirm_deny_rules(&self) -> Option<IntegrationConfirmation> {
        Some(self.integr_common().confirmation)
    }

    fn has_config_path(&self) -> Option<String> {
        Some(self.config_path.clone())
    }
}

const BITBUCKET_INTEGRATION_SCHEMA: &str = r#"
fields:
  bitbucket_token:
    f_type: string_long
    f_desc: "Bitbucket App Password, you can create one [here](https://support.atlassian.com/bitbucket-cloud/docs/app-passwords/). If you don't want to send your key to the AI model that helps you to configure the agent, put it into secrets.yaml and write `$MY_SECRET_VARIABLE` in this field."
    f_placeholder: "xxxxxxxxxxxxxxxx"
    f_label: "App Password"
    smartlinks:
      - sl_label: "Open secrets.yaml"
        sl_goto: "EDITOR:secrets.yaml"
  bitbucket_username:
    f_type: string_long
    f_desc: "Your Bitbucket username."
    f_placeholder: "my_username"
    f_label: "Username"
  bitbucket_workspace:
    f_type: string_long
    f_desc: "Your Bitbucket workspace."
    f_placeholder: "my_workspace"
    f_label: "Workspace"
description: |
  The Bitbucket integration allows interaction with Bitbucket repositories using the Bitbucket Cloud API.
available:
  on_your_laptop_possible: true
  when_isolated_possible: true
confirmation:
  ask_user_default: []
  deny_default: []
smartlinks:
  - sl_label: "Test"
    sl_chat:
      - role: "user"
        content: |
          🔧 The `bitbucket` tool should be visible now. To test the tool, list opened pull requests for a repository.
          If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.
    sl_enable_only_with_tool: true
"#;
