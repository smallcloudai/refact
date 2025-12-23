use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use petgraph::graph::{DiGraph, NodeIndex};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KnowledgeFrontmatter {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub filenames: Vec<String>,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub superseded_by: Option<String>,
    #[serde(default)]
    pub deprecated_at: Option<String>,
    #[serde(default)]
    pub review_after: Option<String>,
}

impl KnowledgeFrontmatter {
    pub fn parse(content: &str) -> (Self, usize) {
        if !content.starts_with("---") {
            return (Self::default(), 0);
        }

        let rest = &content[3..];
        let end_marker = rest.find("\n---");
        let Some(end_idx) = end_marker else {
            return (Self::default(), 0);
        };

        let yaml_content = &rest[..end_idx];
        let mut end_offset = 3 + end_idx + 4;
        if content.len() > end_offset && content.as_bytes().get(end_offset) == Some(&b'\n') {
            end_offset += 1;
        }

        match serde_yaml::from_str::<KnowledgeFrontmatter>(yaml_content) {
            Ok(fm) => (fm, end_offset),
            Err(_) => (Self::default(), 0),
        }
    }

    pub fn to_yaml(&self) -> String {
        let mut lines = vec!["---".to_string()];

        if let Some(id) = &self.id {
            lines.push(format!("id: \"{}\"", id));
        }
        if let Some(title) = &self.title {
            lines.push(format!("title: \"{}\"", title.replace('"', "\\\"")));
        }
        if let Some(kind) = &self.kind {
            lines.push(format!("kind: {}", kind));
        }
        if let Some(created) = &self.created {
            lines.push(format!("created: {}", created));
        }
        if let Some(updated) = &self.updated {
            lines.push(format!("updated: {}", updated));
        }
        if let Some(review_after) = &self.review_after {
            lines.push(format!("review_after: {}", review_after));
        }
        if let Some(status) = &self.status {
            lines.push(format!("status: {}", status));
        }
        if !self.tags.is_empty() {
            let tags_str = self.tags.iter()
                .map(|t| format!("\"{}\"", t))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("tags: [{}]", tags_str));
        }
        if !self.filenames.is_empty() {
            let files_str = self.filenames.iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("filenames: [{}]", files_str));
        }
        if !self.links.is_empty() {
            let links_str = self.links.iter()
                .map(|l| format!("\"{}\"", l))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("links: [{}]", links_str));
        }
        if let Some(superseded_by) = &self.superseded_by {
            lines.push(format!("superseded_by: \"{}\"", superseded_by));
        }
        if let Some(deprecated_at) = &self.deprecated_at {
            lines.push(format!("deprecated_at: {}", deprecated_at));
        }

        lines.push("---".to_string());
        lines.join("\n")
    }

    pub fn is_active(&self) -> bool {
        self.status.as_deref().unwrap_or("active") == "active"
    }

    pub fn is_deprecated(&self) -> bool {
        self.status.as_deref() == Some("deprecated")
    }

    pub fn is_archived(&self) -> bool {
        self.status.as_deref() == Some("archived")
    }

    pub fn kind_or_default(&self) -> &str {
        self.kind.as_deref().unwrap_or(if self.filenames.is_empty() { "domain" } else { "code" })
    }
}

#[derive(Debug, Clone)]
pub struct KnowledgeDoc {
    pub path: PathBuf,
    pub frontmatter: KnowledgeFrontmatter,
    pub content: String,
    pub entities: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum KgNode {
    Doc { id: String },
    Tag,
    FileRef { exists: bool },
    Entity,
}

#[derive(Debug, Clone)]
pub enum KgEdge {
    TaggedWith,
    ReferencesFile,
    LinksTo,
    Mentions,
    SupersededBy,
}

pub struct KnowledgeGraph {
    pub graph: DiGraph<KgNode, KgEdge>,
    pub doc_index: HashMap<String, NodeIndex>,
    pub doc_path_index: HashMap<PathBuf, NodeIndex>,
    pub tag_index: HashMap<String, NodeIndex>,
    pub file_index: HashMap<String, NodeIndex>,
    pub entity_index: HashMap<String, NodeIndex>,
    pub docs: HashMap<String, KnowledgeDoc>,
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            doc_index: HashMap::new(),
            doc_path_index: HashMap::new(),
            tag_index: HashMap::new(),
            file_index: HashMap::new(),
            entity_index: HashMap::new(),
            docs: HashMap::new(),
        }
    }

    pub fn get_or_create_tag(&mut self, name: &str) -> NodeIndex {
        let normalized = name.to_lowercase().trim().to_string();
        if let Some(&idx) = self.tag_index.get(&normalized) {
            return idx;
        }
        let idx = self.graph.add_node(KgNode::Tag);
        self.tag_index.insert(normalized, idx);
        idx
    }

    pub fn get_or_create_file(&mut self, path: &str, exists: bool) -> NodeIndex {
        if let Some(&idx) = self.file_index.get(path) {
            return idx;
        }
        let idx = self.graph.add_node(KgNode::FileRef { exists });
        self.file_index.insert(path.to_string(), idx);
        idx
    }

    pub fn get_or_create_entity(&mut self, name: &str) -> NodeIndex {
        if let Some(&idx) = self.entity_index.get(name) {
            return idx;
        }
        let idx = self.graph.add_node(KgNode::Entity);
        self.entity_index.insert(name.to_string(), idx);
        idx
    }

    pub fn add_doc(&mut self, doc: KnowledgeDoc) -> NodeIndex {
        let id = doc.frontmatter.id.clone().unwrap_or_else(|| doc.path.to_string_lossy().to_string());
        let path = doc.path.clone();

        let doc_idx = self.graph.add_node(KgNode::Doc { id: id.clone() });
        self.doc_index.insert(id.clone(), doc_idx);
        self.doc_path_index.insert(path, doc_idx);

        for tag in &doc.frontmatter.tags {
            let tag_idx = self.get_or_create_tag(tag);
            self.graph.add_edge(doc_idx, tag_idx, KgEdge::TaggedWith);
        }

        for filename in &doc.frontmatter.filenames {
            let file_idx = self.get_or_create_file(filename, true);
            self.graph.add_edge(doc_idx, file_idx, KgEdge::ReferencesFile);
        }

        for entity in &doc.entities {
            let entity_idx = self.get_or_create_entity(entity);
            self.graph.add_edge(doc_idx, entity_idx, KgEdge::Mentions);
        }

        self.docs.insert(id, doc);
        doc_idx
    }

    pub fn link_docs(&mut self) {
        let links: Vec<(String, String)> = self.docs.iter()
            .flat_map(|(id, doc)| {
                doc.frontmatter.links.iter().map(|link| (id.clone(), link.clone())).collect::<Vec<_>>()
            })
            .collect();

        for (from_id, to_id) in links {
            if let (Some(&from_idx), Some(&to_idx)) = (self.doc_index.get(&from_id), self.doc_index.get(&to_id)) {
                self.graph.add_edge(from_idx, to_idx, KgEdge::LinksTo);
            }
        }

        let supersedes: Vec<(String, String)> = self.docs.iter()
            .filter_map(|(id, doc)| {
                doc.frontmatter.superseded_by.as_ref().map(|s| (id.clone(), s.clone()))
            })
            .collect();

        for (old_id, new_id) in supersedes {
            if let (Some(&old_idx), Some(&new_idx)) = (self.doc_index.get(&old_id), self.doc_index.get(&new_id)) {
                self.graph.add_edge(old_idx, new_idx, KgEdge::SupersededBy);
            }
        }
    }

    pub fn get_doc_by_id(&self, id: &str) -> Option<&KnowledgeDoc> {
        self.docs.get(id)
    }

    pub fn get_doc_by_path(&self, path: &PathBuf) -> Option<&KnowledgeDoc> {
        self.doc_path_index.get(path)
            .and_then(|idx| {
                if let Some(KgNode::Doc { id, .. }) = self.graph.node_weight(*idx) {
                    self.docs.get(id)
                } else {
                    None
                }
            })
    }

    pub fn active_docs(&self) -> impl Iterator<Item = &KnowledgeDoc> {
        self.docs.values().filter(|d| d.frontmatter.is_active())
    }

    pub fn docs_with_tag(&self, tag: &str) -> HashSet<String> {
        let normalized = tag.to_lowercase();
        let Some(&tag_idx) = self.tag_index.get(&normalized) else {
            return HashSet::new();
        };

        self.graph.neighbors_directed(tag_idx, petgraph::Direction::Incoming)
            .filter_map(|idx| {
                if let Some(KgNode::Doc { id, .. }) = self.graph.node_weight(idx) {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn docs_referencing_file(&self, file_path: &str) -> HashSet<String> {
        let Some(&file_idx) = self.file_index.get(file_path) else {
            return HashSet::new();
        };

        self.graph.neighbors_directed(file_idx, petgraph::Direction::Incoming)
            .filter_map(|idx| {
                if let Some(KgNode::Doc { id, .. }) = self.graph.node_weight(idx) {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn docs_mentioning_entity(&self, entity: &str) -> HashSet<String> {
        let Some(&entity_idx) = self.entity_index.get(entity) else {
            return HashSet::new();
        };

        self.graph.neighbors_directed(entity_idx, petgraph::Direction::Incoming)
            .filter_map(|idx| {
                if let Some(KgNode::Doc { id, .. }) = self.graph.node_weight(idx) {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}
