use similar::DiffableStr;
use tree_sitter::{Node, Query, QueryCapture, Range};

use crate::ast::treesitter::structs::{FunctionCallInfo, StaticInfo, StaticType, VariableInfo};

pub(crate) fn get_function_name(parent: Node, text: &str) -> String {
    for i in 0..parent.child_count() {
        if let Some(child) = parent.child(i) {
            let kind = child.kind();
            match kind {
                "identifier" => {
                    let name = text.slice(child.byte_range());
                    return name.to_string();
                }
                _ => {}
            }
        }
    }
    return "".to_string();
}

pub(crate) fn get_variable(captures: &[QueryCapture], query: &Query, code: &str) -> Option<VariableInfo> {
    let mut var = VariableInfo {
        name: "".to_string(),
        range: Range {
            start_byte: 0,
            end_byte: 0,
            start_point: Default::default(),
            end_point: Default::default(),
        },
        type_names: vec![],
        meta_path: None,
    };
    for capture in captures {
        let capture_name = &query.capture_names()[capture.index as usize];
        match capture_name.as_str() {
            "variable" => {
                var.range = capture.node.range()
            }
            "variable_name" => {
                let text = code.slice(capture.node.byte_range());
                var.name = text.to_string();
            }
            "variable_type" => {
                let text = code.slice(capture.node.byte_range());
                var.type_names.push(text.to_string());
            }
            &_ => {}
        }
    }
    if var.name.is_empty() {
        return None;
    }

    Some(var)
}

pub(crate) fn get_call(captures: &[QueryCapture], query: &Query, code: &str) -> Option<FunctionCallInfo> {
    let mut var = FunctionCallInfo {
        name: "".to_string(),
        range: Range {
            start_byte: 0,
            end_byte: 0,
            start_point: Default::default(),
            end_point: Default::default(),
        },
        caller_type_name: None,
        meta_path: None,
    };
    for capture in captures {
        let capture_name = &query.capture_names()[capture.index as usize];
        match capture_name.as_str() {
            "call" => {
                var.range = capture.node.range()
            }
            "call_name" => {
                let text = code.slice(capture.node.byte_range());
                var.name = text.to_string();
            }
            &_ => {}
        }
    }
    if var.name.is_empty() {
        return None;
    }
    Some(var)
}

pub(crate) fn get_static(captures: &[QueryCapture], query: &Query, code: &str) -> Option<StaticInfo> {
    let text = code.slice(captures[0].node.byte_range());
    for capture in captures {
        let capture_name = &query.capture_names()[capture.index as usize];
        return match capture_name.as_str() {
            "comment" => {
                Some(StaticInfo {
                    data: text.to_string(),
                    static_type: StaticType::Comment,
                    range: capture.node.range(),
                    meta_path: None,
                })
            }
            "string_literal" => {
                Some(StaticInfo {
                    data: text.to_string(),
                    static_type: StaticType::Literal,
                    range: capture.node.range(),
                    meta_path: None,
                })
            }
            &_ => {
                None
            }
        };
    }
    None
}