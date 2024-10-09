use std::sync::Arc;
use std::collections::HashMap;
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

#[derive(Clone, Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct IntegrationDocker {
    pub connect_to_daemon_at: String,   // 127.0.0.1:1337
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

        let conn_bollard = Docker::connect_with_local(
            "unix:///Users/humbertoyusta/.docker/run/docker.sock", 120, bollard::API_DEFAULT_VERSION).unwrap();
        //     &settings_docker.connect_to_daemon_at,
        //     120,
        //     bollard::API_DEFAULT_VERSION,
        // ).unwrap();

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
                let short_id = &image.id[7..19];
                json_result["images"].as_array_mut().unwrap().push(serde_json::json!({
                    "repo_tags": image.repo_tags,
                    "id": short_id
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

// fn parse_options(
//     args: &[String],
//     flag_options: &[(String, String)],
//     value_options: &[(String, String)],
// ) -> Result<(Vec<String>, HashMap<String, String>), String> {
//     let mut options = HashMap::new();
//     let mut remaining_args = Vec::new();
//     let mut iter = args.iter().peekable();

//     // Convert the tuples into sets for easier lookup
//     let flag_set: std::collections::HashSet<_> = flag_options.iter().flat_map(|(s, l)| vec![s, l]).collect();
//     let value_set: std::collections::HashSet<_> = value_options.iter().flat_map(|(s, l)| vec![s, l]).collect();

//     while let Some(arg) = iter.next() {
//         if arg.starts_with("--") || arg.starts_with("-") {
//             if let Some(eq_pos) = arg.find('=') {
//                 // Handle --option=value or -o=value
//                 let key = arg[..eq_pos].to_string();
//                 let value = arg[eq_pos + 1..].to_string();
//                 if value_set.contains(&key.as_str()) {
//                     options.insert(key, value);
//                 } else {
//                     return Err(format!("Unknown option: {}", key));
//                 }
//             } else if flag_set.contains(&arg.as_str()) {
//                 // Handle flag options like --interactive or -i
//                 options.insert(arg.clone(), String::new());
//             } else if value_set.contains(&arg.as_str()) {
//                 // Handle value options like --option value or -o value
//                 if let Some(next_arg) = iter.peek() {
//                     if !next_arg.starts_with("--") && !next_arg.starts_with("-") {
//                         options.insert(arg.clone(), next_arg.clone());
//                         iter.next(); // Skip the next argument as it is the value for the current option
//                     } else {
//                         return Err(format!("Expected value for option: {}", arg));
//                     }
//                 } else {
//                     return Err(format!("Expected value for option: {}", arg));
//                 }
//             } else {
//                 // Handle unknown options
//                 return Err(format!("Unknown option: {}", arg));
//             }
//         } else {
//             remaining_args.push(arg.clone());
//         }
//     }

//     Ok((remaining_args, options))
// }

// use bollard::image::BuildImageOptions;
// use bollard::Docker;


async fn build_docker_image(parsed_args: &Vec<String>, docker: &Docker) -> Result<Value, String> {
    let image_name = "test";
    let folder_path = parsed_args[2].clone();

    let build_options = BuildImageOptions {
        t: image_name.to_string(),
        rm: true,
        ..Default::default()
    };

    // Create a tar archive of the folder
    let tar_data = {
        let mut tar = Builder::new(Vec::new());
        tar.append_dir_all(".", &folder_path).map_err(|e| e.to_string())?;
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