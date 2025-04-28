use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::path::{PathBuf};

struct TrieNode {
    children: HashMap<String, TrieNode>,
    indices: Vec<usize>,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            indices: Vec::new(),
        }
    }
}

pub struct PathTrie {
    // TODO: do we need to store paths here?
    pub paths: Vec<PathBuf>,
    pub unique_paths: HashSet<String>,
    root: TrieNode,
}

impl PathTrie {
    pub fn new() -> Self {
        PathTrie {
            paths: vec![],
            unique_paths: HashSet::new(),
            root: TrieNode::new(),
        }
    }

    pub fn build(paths: &Vec<PathBuf>) -> Self {
        let mut root = TrieNode::new();

        for (index, path) in paths.iter().enumerate() {
            let components: Vec<String> = path
                .components()
                .map(|comp| comp.as_os_str().to_string_lossy().to_string())
                .collect();

            let mut node = &mut root;
            for i in (0..components.len()).rev() {
                let component = &components[i];
                node = node.children.entry(component.clone()).or_insert_with(TrieNode::new);
                node.indices.push(index);
            }
        }

        let mut index_to_components = HashMap::new();
        let mut nodes_to_process = VecDeque::new();

        let root_component = String::new();
        nodes_to_process.push_back((&root_component, &root));
        while let Some((component, node)) = nodes_to_process.pop_front() {
            for index in &node.indices {
                let components = index_to_components.entry(index).or_insert_with(Vec::new);
                components.push(component);
            }
            if node.indices.len() == 1 {
                continue;
            }
            for child in node.children.iter() {
                nodes_to_process.push_back(child);
            }
        }

        let unique_paths: HashSet<String> = index_to_components.iter()
            .map(|(_index, components)|
                components.iter().rev().fold(
                    PathBuf::new(), |mut path, c| {
                        path.push(c); path
                    })
            )
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        PathTrie { paths: paths.clone(), unique_paths, root }
    }

    pub fn find_matches(&self, path: &PathBuf) -> Vec<PathBuf> {
        let components: Vec<String> = path
            .components()
            .map(|comp| comp.as_os_str().to_string_lossy().to_string())
            .collect();
        let mut current = &self.root;
        for component in components.iter().rev() {
            match current.children.get(component) {
                Some(node) => current = node,
                None => return Vec::new(),
            }
        }
        current.indices.iter()
            .map(|&index| self.paths[index].clone())
            .collect()
    }

    pub fn shortest_path(&self, path: &PathBuf) -> Option<PathBuf> {
        let components: Vec<String> = path
            .components()
            .map(|comp| comp.as_os_str().to_string_lossy().to_string())
            .collect();

        for i in (0..components.len()).rev() {
            let mut partial_path = PathBuf::new();
            for j in i..components.len() {
                partial_path.push(&components[j]);
            }
            let matches = self.find_matches(&partial_path);
            if matches.len() == 1 {
                return Some(partial_path);
            }
        }

        None
    }
}