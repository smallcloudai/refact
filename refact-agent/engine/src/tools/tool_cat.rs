use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use serde_json::Value;
use itertools::Itertools; 

use tokio::sync::Mutex as AMutex;
use async_trait::async_trait;
use resvg::{tiny_skia, usvg};
use crate::at_commands::at_commands::AtCommandsContext;
use crate::at_commands::at_file::{file_repair_candidates, return_one_candidate_or_a_good_error};
use crate::tools::tools_description::{Tool, ToolDesc, ToolParam, ToolSource, ToolSourceType};
use crate::call_validation::{ChatMessage, ChatContent, ContextEnum, ContextFile};
use crate::files_correction::{canonical_path, correct_to_nearest_dir_path, get_project_dirs, preprocess_path_for_normalization};
use crate::files_in_workspace::{get_file_text_from_memory_or_disk, ls_files};
use crate::scratchpads::multimodality::MultimodalElement;

use std::io::Cursor;
use image::imageops::FilterType;
use image::{ImageFormat, ImageReader};

pub struct ToolCat {
    pub config_path: String,
}


const CAT_MAX_IMAGES_CNT: usize = 1;

fn parse_cat_args(args: &HashMap<String, Value>) -> Result<(Vec<String>, HashMap<String, Option<(usize, usize)>>, Vec<String>), String> {
    fn try_parse_line_range(s: &str) -> Result<Option<(usize, usize)>, String> {
        let s = s.trim();
        
        // Try parsing as a single number (like "10")
        if let Ok(n) = s.parse::<usize>() {
            return Ok(Some((n, n)));
        }
        
        // Try parsing as a range (like "10-20")
        if s.contains('-') {
            let parts = s.split('-').collect::<Vec<_>>();
            if parts.len() == 2 {
                if let Ok(start) = parts[0].trim().parse::<usize>() {
                    if let Ok(end) = parts[1].trim().parse::<usize>() {
                        if start > end {
                            return Err(format!("Start line ({}) cannot be greater than end line ({})", start, end));
                        }
                        return Ok(Some((start, end)));
                    }
                }
            }
        }
        
        Ok(None) // Not a line range - likely a Windows path
    }
    
    let raw_paths = match args.get("paths") {
        Some(Value::String(s)) => {
            s.split(",").map(|x|x.trim().to_string()).collect::<Vec<_>>()
        },
        Some(v) => return Err(format!("argument `paths` is not a string: {:?}", v)),
        None => return Err("Missing argument `paths`".to_string())
    };
    
    let mut paths = Vec::new();
    let mut path_line_ranges = HashMap::new();
    
    for path_str in raw_paths {
        let (file_path, range) = if let Some(colon_pos) = path_str.rfind(':') {
            match try_parse_line_range(&path_str[colon_pos+1..])? {
                Some((start, end)) => {
                    (path_str[..colon_pos].trim().to_string(), Some((start, end)))
                },
                None => (path_str, None),
            }
        } else {
            (path_str, None)
        };
        path_line_ranges.insert(file_path.clone(), range);
        paths.push(file_path);
    }
    
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
    
    Ok((paths, path_line_ranges, symbols))
}

#[async_trait]
impl Tool for ToolCat {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    fn tool_description(&self) -> ToolDesc {
        ToolDesc {
            name: "cat".to_string(),
            agentic: false,
            display_name: "Cat".to_string(),
            source: ToolSource {
                source_type: ToolSourceType::Builtin,
                config_path: self.config_path.clone(),
            },
            experimental: false,
            description: "Like cat in console, but better: it can read multiple files and images. Prefer to open full files.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "paths".to_string(),
                    description: "Comma separated file names or directories: dir1/file1.ext,dir3/dir4.".to_string(),
                    param_type: "string".to_string(),
                },
                ToolParam {
                    name: "output_limit".to_string(),
                    description: "Optional. Max lines to show (default: uses smart compression). Use higher values like '500' or 'all' to see more output.".to_string(),
                    param_type: "string".to_string(),
                },
            ],
            parameters_required: vec!["paths".to_string()],
        }
    }

    async fn tool_execute(
        &mut self,
        ccx: Arc<AMutex<AtCommandsContext>>,
        tool_call_id: &String,
        args: &HashMap<String, Value>
    ) -> Result<(bool, Vec<ContextEnum>), String> {
        let mut corrections = false;
        let (paths, path_line_ranges, symbols) = parse_cat_args(args)?;
        let output_limit = match args.get("output_limit") {
            Some(Value::String(s)) => s.to_lowercase(),
            _ => "".to_string(),
        };
        let no_compression = output_limit == "all" || output_limit == "full";
        
        let (filenames_present, symbols_not_found, not_found_messages, context_enums, multimodal) = 
            paths_and_symbols_to_cat_with_path_ranges(ccx.clone(), paths, path_line_ranges, symbols).await;

        let mut content = "".to_string();
        if !filenames_present.is_empty() {
            content.push_str(&format!("Paths found:\n{}\n\n", filenames_present.iter().unique().cloned().collect::<Vec<_>>().join("\n")));
            if !symbols_not_found.is_empty() {
                content.push_str(&format!("Symbols not found in the {} files:\n{}\n\n", filenames_present.len(), symbols_not_found.join("\n")));
                corrections = true;
            }
        }
        if !not_found_messages.is_empty() {
            content.push_str(&format!("Problems:\n{}\n\n", not_found_messages.join("\n\n")));
            corrections = true;
        }

        // When output_limit="all", mark ContextFiles to skip postprocessing compression
        let mut results: Vec<ContextEnum> = if no_compression {
            context_enums.into_iter().map(|ctx| {
                if let ContextEnum::ContextFile(mut cf) = ctx {
                    cf.skip_pp = true;
                    ContextEnum::ContextFile(cf)
                } else {
                    ctx
                }
            }).collect()
        } else {
            context_enums
        };
        
        let chat_content = if multimodal.is_empty() {
            ChatContent::SimpleText(content)
        } else {
            ChatContent::Multimodal([
                vec![MultimodalElement { m_type: "text".to_string(), m_content: content }],
                multimodal
            ].concat())
        };

        results.push(ContextEnum::ChatMessage(ChatMessage {
            role: "tool".to_string(),
            content: chat_content,
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

pub async fn paths_and_symbols_to_cat_with_path_ranges(
    ccx: Arc<AMutex<AtCommandsContext>>,
    paths: Vec<String>,
    path_line_ranges: HashMap<String, Option<(usize, usize)>>,
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
    let mut corrected_path_to_original = HashMap::new();

    for p in paths {
        let path = if PathBuf::from(&p).is_absolute() {
            canonical_path(p).to_string_lossy().to_string()
        } else {
            preprocess_path_for_normalization(p)
        };

        // both not fuzzy
        let candidates_file = file_repair_candidates(gcx.clone(), &path, top_n, false).await;
        let candidates_dir = correct_to_nearest_dir_path(gcx.clone(), &path, false, top_n).await;

        if !candidates_file.is_empty() || candidates_dir.is_empty() {
            let file_path = match return_one_candidate_or_a_good_error(gcx.clone(), &path, &candidates_file, &get_project_dirs(gcx.clone()).await, false).await {
                Ok(f) => f,
                Err(e) => { not_found_messages.push(e); continue;}
            };
            corrected_paths.push(file_path.clone());
            corrected_path_to_original.insert(file_path, path.clone());
        } else {
            let candidate = match return_one_candidate_or_a_good_error(gcx.clone(), &path, &candidates_dir, &get_project_dirs(gcx.clone()).await, true).await {
                Ok(f) => f,
                Err(e) => { not_found_messages.push(e); continue;}
            };
            let path_buf = PathBuf::from(candidate);
            let indexing_everywhere = crate::files_blocklist::reload_indexing_everywhere_if_needed(gcx.clone()).await;
            let files_in_dir = ls_files(&indexing_everywhere, &path_buf, false).unwrap_or(vec![]);
            for file in files_in_dir {
                let file_str = file.to_string_lossy().to_string();
                corrected_paths.push(file_str.clone());
                corrected_path_to_original.insert(file_str, path.clone());
            }
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
            let original_path = corrected_path_to_original.get(p).unwrap_or(p);
            let line_range = path_line_ranges.get(original_path).cloned().flatten();
            
            let doc_syms = crate::ast::ast_db::doc_defs(ast_index.clone(), &p);
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
                let sym_start = sym.full_line1();
                let sym_end = sym.full_line2();

                // If line range is specified, check overlap
                let (start_line, end_line) = match line_range {
                    Some((start, end)) => {
                        // If symbol doesn't overlap with requested line range, skip it
                        if end < sym_start || start > sym_end {
                            // Symbol is completely outside requested range
                            continue;
                        }
                        // Show the intersection of symbol range and requested range
                        (start.max(sym_start), end.min(sym_end))
                    },
                    None => (sym_start, sym_end)
                };
                
                let cf = ContextFile {
                    file_name: p.clone(),
                    file_content: "".to_string(),
                    line1: start_line,
                    line2: end_line,
                    symbols: vec![sym.path()],
                    gradient_type: 5,
                    usefulness: 100.0,
                    skip_pp: false,
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
        let original_path = corrected_path_to_original.get(p).unwrap_or(p);
        let line_range = path_line_ranges.get(original_path).cloned().flatten();
        
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
                    let total_lines = text.lines().count();
                    let (start_line, end_line) = match line_range {
                        Some((start, end)) => {
                            let start = start.max(1);
                            let end = end.min(total_lines);
                            if start > end {
                                not_found_messages.push(format!(
                                    "Requested line range {}-{} is outside file bounds (file has {} lines)", 
                                    start, end, total_lines
                                ));
                                (1, total_lines)
                            } else {
                                (start, end)
                            }
                        },
                        None => (1, total_lines)
                    };
                    
                    let cf = ContextFile {
                        file_name: p.clone(),
                        file_content: "".to_string(),
                        line1: start_line,
                        line2: end_line,
                        symbols: vec![],
                        gradient_type: 5,
                        usefulness: 100.0,
                        skip_pp: false,
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
