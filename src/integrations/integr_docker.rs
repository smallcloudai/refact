use std::path::Path;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use regex::Regex;
use tokio::io::{duplex, DuplexStream};
use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use tracing::{error, info};
use serde::{Deserialize, Serialize};
use bollard::Docker;
use bollard::container::{CreateContainerOptions, StartContainerOptions, Config};
use bollard::image::BuildImageOptions;
use bollard::models::{HostConfig, ContainerCreateResponse};
use futures_util::stream::StreamExt;
use serde_json::Value;
use futures::stream::once;
use std::default::Default;
use std::fs::File;
use std::io::Read;
use tar::Builder;
use serde_json::json;
use hyper::body::Bytes;

use crate::at_commands::at_commands::AtCommandsContext;
use crate::call_validation::{ContextEnum, ChatMessage};
use crate::tools::tools_description::Tool;

const COMMON_LABEL: &str = "humberto-refact";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct IntegrationDocker {
    pub connect_to_daemon_at: String,   // 127.0.0.1:1337
    // pub ssh_config: Option<SshConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub identity_file: Option<String>,
}

// Bollard features:
// ssl: enable SSL support through Rustls with the ring provider.
// aws-lc-rs: enable SSL support through Rustls with the aws-lc-rs provider.
// ssl_providerless: enable SSL support through Rustls without installing a CryptoProvider. You are responsible to do so.


pub struct ToolDocker {
    settings_docker: IntegrationDocker,
    conn_bollard: Docker,
}

impl ToolDocker {
    pub fn new_if_configured(integrations_value: &serde_yaml::Value) -> Option<Self> {
        let settings_docker_value = integrations_value.get("docker")?;
    
        let settings_docker = serde_yaml::from_value::<IntegrationDocker>(settings_docker_value.clone()).or_else(|e| {
            error!("Failed to parse integration docker: {:?}", e);
            Err(e)
        }).ok()?;
    
        let conn_bollard = {
            let connect_method = if settings_docker.connect_to_daemon_at.starts_with("http://") 
                || settings_docker.connect_to_daemon_at.starts_with("tcp://") {
                Docker::connect_with_http
            } else {
                Docker::connect_with_local
            };
    
            connect_method(&settings_docker.connect_to_daemon_at, 30 * 60, bollard::API_DEFAULT_VERSION)
                .or_else(|e| {
                    error!("Failed to connect to docker daemon: {:?}", e);
                    Err(e)
                }).ok()?
        };
    
        Some(Self { settings_docker, conn_bollard })
    }
}

#[async_trait]
impl Tool for ToolDocker {
    async fn tool_execute(
        &mut self,
        _ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>,
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let command = match args.get("command") {
            Some(Value::String(s)) => s,
            Some(v) => return Err(format!("argument `command` is not a string: {:?}", v)),
            None => return Err("Missing argument `command`".to_string())
        };

        // Parse the command arguments
        let parsed_args = shell_words::split(command).map_err(|e| e.to_string())?;
        if parsed_args.is_empty() {
            return Err("Parsed command is empty".to_string());
        }

        let mut json_result = serde_json::json!({});

        // Example: docker run --label horrible --name stupid_script_100 oleg_aaa1 python3 stupid_script.py arg1 arg2
        // if parsed_args[0] == "docker" && parsed_args[1] == "run" {
        //     let container_name = parsed_args.iter().position(|x| x == "--name").map(|i| parsed_args[i + 1].clone()).unwrap_or_default();
        //     let image_name = parsed_args[2].clone();
        //     let cmd_args: Vec<String> = parsed_args[3..].to_vec();

        //     let config = Config {
        //         image: Some(image_name),
        //         cmd: Some(cmd_args),
        //         labels: Some(HashMap::from([("horrible".to_string(), "".to_string())])),
        //         ..Default::default()
        //     };

        //     let create_options = CreateContainerOptions {
        //         name: container_name.clone(),
        //         platform: None,
        //     };

        //     let container: ContainerCreateResponse = self.conn_bollard.create_container(Some(create_options), config).await.map_err(|e| e.to_string())?;
        //     self.conn_bollard.start_container(&container.id, None::<StartContainerOptions<String>>).await.map_err(|e| e.to_string())?;
        // }


        // Example: docker create --name my_stupid_script_container --label task=stupid_script aaa1
        // if parsed_args[0] == "docker" && parsed_args[1] == "create" {
            // Implement docker create logic using bollard
        // }

        if parsed_args[0] == "docker" && parsed_args[1] == "images" {
            let images = self.conn_bollard.list_images(Some(bollard::image::ListImagesOptions::<String> {
                all: true,
                ..Default::default()
            })).await.map_err(|e| e.to_string())?;
            json_result = serde_json::json!({ "images": [] });
            for image in images {
                info!("{:?}", image);
                json_result["images"].as_array_mut().unwrap().push(serde_json::json!({
                    "repo_tags": image.repo_tags,
                    "id": &image.id[7..19],
                }));
            }
        }

        // Example: docker build -t aaa1 . && \
        // docker run -d --name my_stupid_script_container --label task=stupid_script aaa1 && \
        // docker cp /path/to/your/binary my_stupid_script_container:/path/in/container/binary && \
        // docker exec -it my_stupid_script_container /path/in/container/binary && \
        // docker stop my_stupid_script_container && \
        // docker rm my_stupid_script_container
        if parsed_args[0] == "docker" && parsed_args[1] == "build" {
            json_result = build_docker_image(&parsed_args, &self.conn_bollard).await?;
        }

        let mut results = vec![];
        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: json_result.to_string(),
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((false, results))
    }
}

// use bollard::image::BuildImageOptions;
// use bollard::Docker;


async fn build_docker_image(parsed_args: &Vec<String>, docker: &Docker) -> Result<Value, String> {
    let (remaining_args, options) = parse_options(parsed_args, &HashSet::new(), &HashSet::from(["file", "tag"]))?;
    let dockerfile = options.get("file").unwrap_or(&"Dockerfile".to_string()).to_string();
    let context_dir = remaining_args.last().unwrap_or(&"".to_string()).to_string();
    let context_dir_name = Regex::new(r"[\\/]").unwrap().split(&context_dir).last().unwrap_or("").to_string();
    let image_tag = options.get("tag").unwrap_or(&context_dir_name).to_string();

    let build_options = BuildImageOptions {
        t: image_tag,
        rm: true,
        dockerfile: dockerfile,
        labels: HashMap::from([(COMMON_LABEL.to_string(), "".to_string())]),
        ..Default::default()
    };

    // Create a tar archive of the folder
    let tar_data = {
        let mut tar = Builder::new(Vec::new());
        tar.append_dir_all(".", &context_dir).map_err(|e| e.to_string())?;
        tar.into_inner().map_err(|e| e.to_string())?
    };
    
    // Build the Docker image
    let mut build_stream = docker.build_image(build_options, None, Some(Bytes::from(tar_data)));    
    let mut build_log = Vec::new();

    while let Some(build_result) = build_stream.next().await {
        match build_result {
            Ok(output) => {
                println!("{:?}", output);
                build_log.push(format!("{:?}", output));
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    let json_result = json!({ "status": "Image built successfully" });
    info!("{:?}", build_log);

    Ok(json_result)
}

// async fn create_tar_from_context(context_dir: &Path) -> Result<DuplexStream, String> {
//     // Create a duplex stream with a 16 KB buffer for async tar streaming
//     let (mut tar_writer, tar_reader) = duplex(16 * 1024);

//     // Read the .dockerignore file and set up the directory walker
//     let dockerignore_path = context_dir.join(".dockerignore");
//     let mut walk_builder = WalkBuilder::new(context_dir);
//     if dockerignore_path.exists() {
//         walk_builder.add_custom_ignore_filename(file_name(&dockerignore_path));
//     }

//     // Create the tar archive asynchronously
//     tokio::spawn(async move {
//         let mut tar = Builder::new(&mut tar_writer);

//         for result in walk_builder.build() {
//             match result {
//                 Ok(entry) => {
//                     let path = entry.path();
//                     if let Ok(relative_path) = path.strip_prefix(context_dir) {
//                         if let Err(e) = tar.append_path_with_name(path, relative_path) {
//                             eprintln!("Error adding path to tar: {}", e);
//                         }
//                     }
//                 }
//                 Err(e) => eprintln!("Error reading directory entry: {}", e),
//             }
//         }

//         // Finalize the tar archive and close the writer
//         if let Err(e) = tar.finish() {
//             eprintln!("Error finishing tar: {}", e);
//         }

//         if let Err(e) = tar_writer.shutdown().await {
//             eprintln!("Error shutting down tar writer: {}", e);
//         }
//     });

//     Ok(tar_reader)
// }

fn parse_options(
    args: &[String],
    flag_options: &HashSet<&str>,
    value_options: &HashSet<&str>,
) -> Result<(Vec<String>, HashMap<String, String>), String> {
    let mut options = HashMap::new();
    let mut remaining_args = Vec::new();
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        if arg.starts_with("-") && !arg.starts_with("--") {
            return Err(format!("Use only long options, starting with \"--\""));
        }
        if arg.starts_with("--") {
            let option_key = arg.strip_prefix("--").unwrap_or(arg);
            if flag_options.contains(option_key) {
                options.insert(option_key.to_string(), "true".to_string());
            } else if value_options.contains(option_key) {
                if let Some(next_arg) = iter.peek() {
                    options.insert(option_key.to_string(), next_arg.to_string());
                } else {
                    return Err(format!("Missing value for option \"{}\"", option_key));
                }
            } else {
                return Err(format!("Option \"{}\" is not supported", option_key));
            }
        } else {
            remaining_args.push(arg.clone());
        }
    }

    Ok((remaining_args, options))
}