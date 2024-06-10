use tree_sitter::Node;
use uuid::Uuid;
use similar::DiffableStr;

use crate::ast::treesitter::ast_instance_structs::{AstSymbolFields, AstSymbolInstanceArc};

pub(crate) fn get_guid() -> Uuid {
    Uuid::new_v4()
}

pub(crate) fn get_children_guids(parent_guid: &Uuid, children: &Vec<AstSymbolInstanceArc>) -> Vec<Uuid> {
    let mut result = Vec::new();
    for child in children {
        let child_ref = child.read();
        if let Some(child_guid) = child_ref.parent_guid() {
            if child_guid == parent_guid {
                result.push(child_ref.guid().clone());
            }
        }
    }
    result
}


pub(crate) struct CandidateInfo<'a> {
    pub ast_fields: AstSymbolFields,
    pub node: Node<'a>,
    pub parent_guid: Uuid,
}


pub fn class_shortened_version(
    output: &mut String,
    indent: usize,
    the_class: &Node,
    code: &str,
) {
    for i in 0..the_class.child_count() {
        let something_about_class = the_class.child(i).unwrap();
        match something_about_class.kind() {
            "interface_body" | "class_body" => {
                for j in 0..something_about_class.child_count() {
                    let something_inside_body = something_about_class.child(j).unwrap();
                    match something_inside_body.kind() {
                        "field_declaration" | "method_declaration" => {
                            let mut is_private: bool = false;
                            if let Some(modifiers) = something_inside_body.child_by_field_name("modifiers") {
                                for i in 0..modifiers.child_count() {
                                    let child = modifiers.child(i).unwrap();
                                    if child.kind() == "private" {
                                        is_private = true;
                                    }
                                }
                            }
                            if is_private {
                                continue;
                            }
                            for i in 0..something_inside_body.child_count() {
                                let child = something_inside_body.child(i).unwrap();
                                if child.kind() == "block" {
                                    output.push_str("{ ... }");
                                } else {
                                    output_append_node_text(output, indent + 2, &child, code);
                                }
                                // tracing::info!("{}child {:?}", "    ".repeat(indent + 2), child.kind());
                            }
                            output_append_newline(output);
                        }
                        "{" => {
                            output_append_node_text(output, indent + 1, &something_inside_body, code);
                            output_append_newline(output);
                        }
                        "}" => {
                            output_append_node_text(output, indent, &something_inside_body, code);
                            output_append_newline(output);
                        }
                        _ => {
                            // tracing::info!("{}skipping something_inside_body {:?}", "    ".repeat(indent + 1), something_inside_body.kind());
                        }
                    }
                }
            }
            "line_comment" | "block_comment" => {
            }
            _ => {
                // tracing::info!("{}using something_about_class {:?} {}-{}", "    ".repeat(indent), something_about_class.kind(), something_about_class.range().start_point.row, something_about_class.range().end_point.row);
                output_append_node_text(output, indent, &something_about_class, code);
            }
        }
    }
}

fn output_append_node_text(
    output: &mut String,
    indent: usize,
    node: &Node,
    code: &str
) {
    let need_indent = output.is_empty() || output.ends_with('\n');
    if need_indent {
        let tabs = "    ".repeat(indent);
        output.push_str(&tabs);
    }
    let name = if let Some(name_node) = node.child_by_field_name("name") {
        code.slice(name_node.byte_range()).to_string()
    } else {
        "".to_string()
    };
    tracing::info!("{}node {:?} name={:?} takes range {}-{}", "    ".repeat(indent), node.kind(), name, node.start_position(), node.end_position());
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();
    let r = &code[start_byte..end_byte];
    output.push_str(r);
    output.push(' ');
}

fn output_append_newline(
    output: &mut String,
) {
    let last_space = output.len() > 0 && output.ends_with(" ");
    if last_space {
        output.pop();
    }
    output.push_str("\n");
}
