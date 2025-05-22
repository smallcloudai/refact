use std::collections::{HashMap, HashSet};
use std::path::{PathBuf};

struct TrieNode {
    children: HashMap<usize, TrieNode>,
    count: usize,
    is_root: bool,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            count: 0,
            is_root: false,
        }
    }
}

pub struct PathTrie {
    root: TrieNode,
    index_to_component: HashMap<usize, String>,
}

fn shortest_root_path(path: &PathBuf, root_paths: &Vec<PathBuf>) -> PathBuf {
    for root_path in root_paths.iter() {
        match path.strip_prefix(&root_path) {
            Ok(_) => return root_path.clone(),
            Err(_) => continue,
        }
    }
    PathBuf::new()
}

pub struct ShortPathsIter<'a> {
    trie: &'a PathTrie,
    stack: Vec<(
        &'a TrieNode,
        HashSet<usize>,
        String,
    )>,
}

impl<'a> Iterator for ShortPathsIter<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, indices_to_process, _)) = self.stack.last_mut() {
            // 1. if node is_root or have no children we should end with the node and return path
            if node.is_root || node.children.is_empty() {
                let mut path = PathBuf::new();
                for (_, _, component) in self.stack.iter().rev() {
                    if !component.is_empty() {
                        path.push(component);
                    }
                }
                self.stack.pop();
                return Some(path.to_string_lossy().to_string());
            }
            // 2. go deeper or end with the node
            if let Some(index) = indices_to_process.iter().next().cloned() {
                indices_to_process.remove(&index);
                let child = node.children.get(&index).unwrap();
                let component = if child.is_root {
                    String::new()  // we don't want to add root_path component
                } else {
                    self.trie.index_to_component.get(&index).unwrap().clone()
                };
                self.stack.push((
                    child,
                    child.children.keys().cloned().collect::<HashSet<usize>>(),
                    component,
                ));
            } else {
                self.stack.pop();
            }
        }
        None
    }
}

impl PathTrie {
    pub fn new() -> Self {
        PathTrie {
            root: TrieNode::new(),
            index_to_component: HashMap::new(),
        }
    }

    pub fn build(paths: &Vec<PathBuf>, root_paths: &Vec<PathBuf>) -> Self {
        let mut root = TrieNode::new();
        let mut component_to_index = HashMap::new();
        let mut index_to_component = HashMap::new();

        // NOTE: root paths should be sorted with shortest at front
        let mut sorted_root_paths = root_paths.clone();
        sorted_root_paths.sort_by(|a, b| {
            let component_count_a = a.components().count();
            let component_count_b = b.components().count();
            match component_count_a.cmp(&component_count_b) {
                std::cmp::Ordering::Equal => {
                    a.cmp(b)
                },
                other => other
            }
        });

        for path in paths.iter() {
            // 1. find shortest root for given path (can be empty)
            let root_path = shortest_root_path(path, &sorted_root_paths);
            let root_path_components = root_path.components().count();

            let components: Vec<String> = path
                .components()
                .map(|comp| comp.as_os_str().to_string_lossy().to_string())
                .collect();

            // 2. iteratively insert components of the path
            // (can be root_path instead of single component)
            let mut node = &mut root;
            for i in (0..components.len()).rev() {
                let is_root = root_path_components == i + 1;
                let component = if is_root {
                    &root_path.to_string_lossy().to_string()
                } else {
                    &components[i]
                };
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
                node.is_root = is_root;
                if is_root {
                    node.is_root = is_root;
                    break;
                }
            }
        }

        PathTrie { root, index_to_component }
    }

    fn _search_for_nodes(&self, path: &PathBuf) -> Vec<(&TrieNode, PathBuf)> {
        let mut nodes = vec![];
        let mut components: Vec<String> = path
            .components()
            .map(|comp| comp.as_os_str().to_string_lossy().to_string())
            .collect();

        let mut current = &self.root;
        loop {
            // 1. collect root_component_postfix and pop next component
            let mut components_prefix = PathBuf::new();
            for component in components.iter() {
                components_prefix.push(component.clone());
            }
            let component = components.pop().unwrap();

            // 2. iterate over all children: match root ones and find next
            let mut is_next_found = false;
            for (index, child) in current.children.iter() {
                if let Some(child_component) = self.index_to_component.get(index) {
                    if child.is_root {
                        let mut root_path = PathBuf::from(child_component);
                        if !root_path.ends_with(&components_prefix) {
                            continue;
                        }
                        match path.strip_prefix(&components_prefix) {
                            Ok(root_relative_path) => {
                                root_path.push(root_relative_path);
                                nodes.push((child, root_path));
                            },
                            Err(_) => continue,  // should not happen, but anyway
                        };
                    } else if *child_component == component {
                        is_next_found = true;
                        current = child;
                    }
                }
            }

            // 3. give up if we can't find next node
            if !is_next_found {
                break;
            }

            // 4. if no components we're break
            if components.is_empty() {
                nodes.push((current, path.clone()));
                break;
            }
        }

        nodes
    }

    #[allow(dead_code)]
    fn count_matches(&self, path: &PathBuf) -> usize {
        let mut counter = 0;
        for (node, _) in self._search_for_nodes(path) {
            counter += node.count;
        }
        counter
    }

    pub fn find_matches(&self, path: &PathBuf) -> Vec<PathBuf> {
        let mut result = vec![];
        for (root_node, relative_path) in self._search_for_nodes(path) {
            if root_node.is_root {
                result.push(relative_path);
                continue;
            }
            let mut stack = Vec::new();
            stack.push((root_node, vec![]));
            while let Some((node, components)) = stack.pop() {
                if node.children.is_empty() {
                    let mut matched_path = PathBuf::new();
                    for index in components.iter().rev() {
                        let component = self.index_to_component.get(index).unwrap();
                        matched_path.push(component);
                    }
                    matched_path.push(path);
                    result.push(matched_path);
                } else {
                    for (index, child) in &node.children {
                        let mut child_components = components.clone();
                        child_components.push(*index);
                        stack.push((child, child_components));
                    }
                }
            }
        }
        result
    }

    // Unique shortest possible postfix of the path
    #[allow(dead_code)]
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

    // Short path postfix (relative to shortest root_path)
    pub fn short_path(&self, path: &PathBuf) -> Option<PathBuf> {
        let nodes = self._search_for_nodes(path);
        if nodes.len() == 1 && nodes[0].0.count == 1 {
            let mut node;
            let mut relative_path;
            (node, relative_path) = nodes[0].clone();
            while !node.is_root && !node.children.is_empty() {
                let index;
                (index, node) = node.children.iter().last().unwrap();
                let mut child_relative_path = if node.is_root {
                    PathBuf::new()
                } else {
                    PathBuf::from(self.index_to_component.get(index).unwrap().clone())
                };
                child_relative_path.push(relative_path.clone());
                relative_path = child_relative_path;
            }
            Some(relative_path)
        } else {
            None
        }
    }

    // Iterate over all paths, returns short version (relative to shortest root_path)
    pub fn short_paths_iter(&self) -> ShortPathsIter<'_> {
        ShortPathsIter {
            trie: self,
            stack: vec![(
                &self.root,
                self.root.children.keys().cloned().collect::<HashSet<usize>>(),
                String::new(),
            )],
        }
    }

    pub fn len(&self) -> usize {
        let mut count = 0;
        for (_, child) in &self.root.children {
            count += child.count;
        }
        count
    }
}
