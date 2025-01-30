use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::Value;

use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use resvg::{tiny_skia, usvg};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::tools::tools_description::Tool;
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};
use crate::files_correction::{correct_to_nearest_dir_path, get_project_dirs};
use crate::files_in_workspace::{get_file_text_from_memory_or_disk, ls_files};
use crate::scratchpads::multimodality::MultimodalElement;

use std::io::Cursor;
use image::imageops::FilterType;
use image::{ImageFormat, ImageReader};

pub struct ToolCat;


const CAT_MAX_IMAGES_CNT: usize = 1;

pub fn parse_skeleton_from_args(args: &HashMap<String, Value>) -> Result<bool, String> {
    Ok(match args.get("skeleton") {
        Some(Value::Bool(s)) => *s,
        Some(Value::String(s)) => {
            if s == "true" {
                true
            } else if s == "false" {
                false
            } else {
                return Err(format!("argument `skeleton` is not a bool: {:?}", s));
            }
        }
        Some(v) => return Err(format!("argument `skeleton` is not a bool: {:?}", v)),
        None => false
    })
}

#[async_trait]
impl Tool for ToolCat {
    fn as_any(&self) -> &dyn std::any::Any { self }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let mut corrections = false;
        let paths = match args.get("paths") {
            Some(Value::String(s)) => {
                let paths = s.split(",").map(|x|x.trim().to_string()).collect::<Vec<_>>();
                paths
            },
            Some(v) => return Err(format!("argument `paths` is not a string: {:?}", v)),
            None => return Err("Missing argument `paths`".to_string())
        };
        let symbols = match args.get("symbols") {
            Some(Value::String(s)) => {
                if s == "*" {
                    vec![]
                } else {
                    s.split(",")
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect::<Vec<_>>()
                }
            },
            Some(v) => return Err(format!("argument `symbols` is not a string: {:?}", v)),
            None => vec![],
        };
        let skeleton = parse_skeleton_from_args(args)?;
        ccx.lock().await.pp_skeleton = skeleton;

        let (filenames_present, symbols_not_found, not_found_messages, context_enums, multimodal) = paths_and_symbols_to_cat(ccx.clone(), paths, symbols).await;

        let mut content = "".to_string();
        if !filenames_present.is_empty() {
            content.push_str(&format!("Paths found:\n{}\n\n", filenames_present.join("\n")));
            if !symbols_not_found.is_empty() {
                content.push_str(&format!("Symbols not found in the {} files:\n{}\n\n", filenames_present.len(), symbols_not_found.join("\n")));
                corrections = true;
            }
        }
        if !not_found_messages.is_empty() {
            content.push_str(&format!("Problems:\n{}\n\n", not_found_messages.join("\n\n")));
            corrections = true;
        }

        let mut results = context_enums;
        let content = if multimodal.is_empty() {
            ChatContent::SimpleText(content)
        } else {
            ChatContent::Multimodal([
                vec![MultimodalElement { m_type: "text".to_string(), m_content: content }],
                multimodal
            ].concat())
        };

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content,
            tool_calls: None,
            tool_call_id: tool_call_id.clone(),
            ..Default::default()
        }));

        Ok((corrections, results))
    }
}

// todo: we can extract if from pipe, however PathBuf does not implement it
fn get_file_type(path: &PathBuf) -> String {
    let extension = path.extension().unwrap_or_default().to_string_lossy().to_string();
    if ["png", "svg", "jpeg"].contains(&extension.as_str()) {
        return format!("image/{extension}");
    }
    if ["jpg", "JPG", "JPEG"].contains(&extension.as_str()) {
        return "image/jpeg".to_string();
    }
    return "text".to_string();
}

async fn load_image(path: &String, f_type: &String) -> Result<MultimodalElement, String> {
    let extension = path.split(".").last().unwrap().to_string();
    let mut f_type = f_type.clone();

    let max_dimension = 800;
    let data = match f_type.as_str() {
        "image/png" | "image/jpeg" => {
            let reader = ImageReader::open(path).map_err(|_| format!("{} image read failed", path))?;
            let mut image = reader.decode().map_err(|_| format!("{} image decode failed", path))?;
            let scale_factor = max_dimension as f32 / std::cmp::max(image.width(), image.height()) as f32;
            if scale_factor < 1.0 {
                let (nwidth, nheight) = (scale_factor * image.width() as f32, scale_factor * image.height() as f32);
                image = image.resize(nwidth as u32, nheight as u32, FilterType::Lanczos3);
            }
            let mut data = Vec::new();
            image.write_to(&mut Cursor::new(&mut data), ImageFormat::Png).map_err(|_| format!("{} image encode failed", path))?;
            f_type = "image/png".to_string();
            Ok(data)
        },
        "image/svg" => {
            f_type = "image/png".to_string();
            let tree = {
                let mut opt = usvg::Options::default();
                opt.resources_dir = std::fs::canonicalize(&path)
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()));
                opt.fontdb_mut().load_system_fonts();

                let svg_data = std::fs::read(&path).unwrap();
                usvg::Tree::from_data(&svg_data, &opt).unwrap()
            };

            let mut pixmap_size = tree.size().to_int_size();
            let scale_factor = max_dimension as f32 / std::cmp::max(pixmap_size.width(), pixmap_size.height()) as f32;
            if scale_factor < 1.0 {
                let (nwidth, nheight) = (pixmap_size.width() as f32 * scale_factor, pixmap_size.height() as f32 * scale_factor);
                pixmap_size = tiny_skia::IntSize::from_wh(nwidth as u32, nheight as u32).unwrap();
            }
            let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();

            resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
            pixmap.encode_png().map_err(|_| format!("{} encode_png failed", path))
        },
        _ => Err(format!("Unsupported image format (extension): {}", extension)),
    }?;

    #[allow(deprecated)]
    let m_content = base64::encode(&data);

    MultimodalElement::new(
        f_type.clone(),
        m_content,
    )
}

pub async fn paths_and_symbols_to_cat(
    ccx: Arc<AMutex<AtCommandsContext>>,
    paths: Vec<String>,
    arg_symbols: Vec<String>,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<ContextEnum>, Vec<MultimodalElement>)
{
    let (gcx, top_n) = {
        let ccx_locked = ccx.lock().await;
        (ccx_locked.global_context.clone(), ccx_locked.top_n)
    };
    let ast_service_opt = gcx.read().await.ast_service.clone();

    let mut not_found_messages = vec![];
    let mut corrected_paths = vec![];

    for p in paths {
        // both not fuzzy
        let candidates_file = file_repair_candidates(gcx.clone(), &p, top_n, false).await;
        let candidates_dir = correct_to_nearest_dir_path(gcx.clone(), &p, false, top_n).await;

        if !candidates_file.is_empty() || candidates_dir.is_empty() {
            let file_path = match return_one_candidate_or_a_good_error(gcx.clone(), &p, &candidates_file, &get_project_dirs(gcx.clone()).await, false).await {
                Ok(f) => f,
                Err(e) => { not_found_messages.push(e); continue;}
            };
            corrected_paths.push(file_path);
        } else {
            let candidate = match return_one_candidate_or_a_good_error(gcx.clone(), &p, &candidates_dir, &get_project_dirs(gcx.clone()).await, true).await {
                Ok(f) => f,
                Err(e) => { not_found_messages.push(e); continue;}
            };
            let path = PathBuf::from(candidate);
            let indexing_everywhere = crate::files_blocklist::load_indexing_everywhere_if_needed(gcx.clone()).await;
            let files_in_dir = ls_files(&indexing_everywhere, &path, false).unwrap_or(vec![]);
            corrected_paths.extend(files_in_dir.into_iter().map(|x|x.to_string_lossy().to_string()));
        }
    }

    let unique_paths = corrected_paths.into_iter().collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();

    let mut context_enums = vec![];
    let mut symbols_found = HashSet::<String>::new();
    let mut symbols_not_found = vec![];
    let mut filenames_present = vec![];
    let mut multimodal: Vec<MultimodalElement> = vec![];

    if let Some(ast_service) = ast_service_opt {
        let ast_index = ast_service.lock().await.ast_index.clone();
        for p in unique_paths.iter() {
            let doc_syms = crate::ast::ast_db::doc_defs(ast_index.clone(), &p).await;
            // s.name() means the last part of the path
            // symbols.contains means exact match in comma-separated list
            let mut syms_def_in_this_file = vec![];
            for looking_for in arg_symbols.iter() {
                let colon_colon_looking_for = format!("::{}", looking_for.trim());
                for x in doc_syms.iter() {
                    if x.path().ends_with(colon_colon_looking_for.as_str()) {
                        syms_def_in_this_file.push(x.clone());
                    }
                }
                symbols_found.insert(looking_for.clone());
            }

            for sym in syms_def_in_this_file {
                let cf = ContextFile {
                    file_name: p.clone(),
                    file_content: "".to_string(),
                    line1: sym.full_line1(),
                    line2: sym.full_line2(),
                    symbols: vec![sym.path()],
                    gradient_type: -1,
                    usefulness: 100.0,
                };
                context_enums.push(ContextEnum::ContextFile(cf));
            }
        }
    }

    for looking_for in arg_symbols.iter() {
        if !symbols_found.contains(looking_for) {
            symbols_not_found.push(looking_for.clone());
        }
    }

    let filenames_got_symbols_for = context_enums.iter()
        .filter_map(|x| if let ContextEnum::ContextFile(cf) = x { Some(cf.file_name.clone()) } else { None })
        .collect::<Vec<_>>();

    let mut image_counter = 0;
    for p in unique_paths.iter().filter(|x|!filenames_got_symbols_for.contains(x)) {
        // don't have symbols for these, so we need to mention them as files, without a symbol, analog of @file
        let f_type = get_file_type(&PathBuf::from(p));

        if f_type.starts_with("image/") {
            filenames_present.push(p.clone());
            if image_counter == CAT_MAX_IMAGES_CNT {
                not_found_messages.push("Cat() shows only 1 image per call to avoid token overflow, call several cat() in parallel to see more images.".to_string());
            }
            image_counter += 1;
            if image_counter > CAT_MAX_IMAGES_CNT {
                continue
            }
            match load_image(p, &f_type).await {
                Ok(mm) => {
                    multimodal.push(mm);
                },
                Err(e) => { not_found_messages.push(format!("{}: {}", p, e)); }
            }
        } else {
            match get_file_text_from_memory_or_disk(gcx.clone(), &PathBuf::from(p)).await {
                Ok(text) => {
                    let cf = ContextFile {
                        file_name: p.clone(),
                        file_content: "".to_string(),
                        line1: 1,
                        line2: text.lines().count(),
                        symbols: vec![],
                        gradient_type: -1,
                        usefulness: 0.0,
                    };
                    context_enums.push(ContextEnum::ContextFile(cf));
                },
                Err(e) => {
                    not_found_messages.push(format!("{}: {}", p, e));
                }
            }
        }
    }
    for cf in context_enums.iter()
        .filter_map(|x| if let ContextEnum::ContextFile(cf) = x { Some(cf) } else { None }) {
        filenames_present.push(cf.file_name.clone());
    }
    (filenames_present, symbols_not_found, not_found_messages, context_enums, multimodal)
}
