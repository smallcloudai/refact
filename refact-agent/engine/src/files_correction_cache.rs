use std::collections::{HashMap, HashSet};
use std::path::{PathBuf};

struct TrieNode {
    children: HashMap<usize, TrieNode>,
    count: usize,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            count: 0,
        }
    }
}

pub struct PathTrie {
    root: TrieNode,
    component_to_index: HashMap<String, usize>,
    index_to_component: HashMap<usize, String>,
    // TODO: do we need to store unique paths really?
    pub unique_paths: HashSet<String>,
}

impl PathTrie {
    pub fn new() -> Self {
        PathTrie {
            root: TrieNode::new(),
            component_to_index: HashMap::new(),
            index_to_component: HashMap::new(),
            unique_paths: HashSet::new(),
        }
    }

    pub fn build(paths: &Vec<PathBuf>) -> Self {
        let mut root = TrieNode::new();
        let mut component_to_index = HashMap::new();
        let mut index_to_component = HashMap::new();

        for path in paths.iter() {
            let components: Vec<String> = path
                .components()
                .map(|comp| comp.as_os_str().to_string_lossy().to_string())
                .collect();

            let mut node = &mut root;
            for i in (0..components.len()).rev() {
                let component = &components[i];
                let index = if let Some(index) = component_to_index.get(component) {
                    *index
                } else {
                    let index = component_to_index.len();
                    component_to_index.insert(component.clone(), index);
                    index_to_component.insert(index, component.clone());
                    index
                };
                node = node.children.entry(index).or_insert_with(TrieNode::new);
                node.count += 1;
            }
        }

        let mut unique_paths = HashSet::new();
        let mut stack = Vec::new();
        stack.push((&root, vec![]));
        while let Some((node, components)) = stack.pop() {
            if node.count == 1 {
                let mut matched_path = PathBuf::new();
                for component in components.iter().rev() {
                    matched_path.push(component);
                }
                unique_paths.insert(matched_path.to_string_lossy().to_string());
            } else {
                for (index, child) in &node.children {
                    let mut child_components = components.clone();
                    let component = index_to_component.get(index).unwrap();
                    child_components.push(component);
                    stack.push((child, child_components));
                }
            }
        }

        PathTrie { root, component_to_index, index_to_component, unique_paths }
    }

    fn _search_node(&self, path: &PathBuf) -> &TrieNode {
        let components: Vec<String> = path
            .components()
            .map(|comp| comp.as_os_str().to_string_lossy().to_string())
            .collect();
        let mut current = &self.root;
        for component in components.iter().rev() {
            if let Some(index) = self.component_to_index.get(component) {
                if let Some(node) = current.children.get(index) {
                    current = node;
                } else {
                    return current
                }
            } else {
                return current
            }
        }
        current
    }

    fn count_matches(&self, path: &PathBuf) -> usize {
        let node = self._search_node(path);
        node.count
    }

    pub fn find_matches(&self, path: &PathBuf) -> Vec<PathBuf> {
        let root_node = self._search_node(path);
        let mut result = vec![];

        if root_node.count == 0 {
            return result
        }

        let mut stack = Vec::new();
        stack.push((root_node, vec![]));
        while let Some((node, components)) = stack.pop() {
            if node.children.is_empty() {
                let mut matched_path = PathBuf::new();
                for component in components.iter().rev() {
                    matched_path.push(component);
                }
                matched_path.push(path);
                result.push(matched_path);
            } else {
                for (index, child) in &node.children {
                    let mut child_components = components.clone();
                    let component = self.index_to_component.get(index).unwrap();
                    child_components.push(component);
                    stack.push((child, child_components));
                }
            }
        }
        result
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