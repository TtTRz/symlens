use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A call graph built from extracted call edges.
#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraph {
    /// Node names (qualified symbol names)
    pub nodes: Vec<String>,
    /// Edges: (from_index, to_index)
    pub edges: Vec<(usize, usize)>,
    /// Name → node index
    #[serde(skip)]
    name_to_idx: HashMap<String, usize>,
}

impl CallGraph {
    /// Build a call graph from call edges.
    pub fn build(edges: &[(String, String)]) -> Self {
        let mut name_to_idx: HashMap<String, usize> = HashMap::new();
        let mut nodes: Vec<String> = Vec::new();

        let mut get_or_insert = |name: &str| -> usize {
            if let Some(&idx) = name_to_idx.get(name) {
                idx
            } else {
                let idx = nodes.len();
                nodes.push(name.to_string());
                name_to_idx.insert(name.to_string(), idx);
                idx
            }
        };

        let graph_edges: Vec<(usize, usize)> = edges
            .iter()
            .map(|(caller, callee)| {
                let from = get_or_insert(caller);
                let to = get_or_insert(callee);
                (from, to)
            })
            .collect();

        CallGraph {
            nodes,
            edges: graph_edges,
            name_to_idx,
        }
    }

    /// Rebuild the name_to_idx after deserialization.
    pub fn rebuild_index(&mut self) {
        self.name_to_idx.clear();
        for (i, name) in self.nodes.iter().enumerate() {
            self.name_to_idx.insert(name.clone(), i);
        }
    }

    /// Get all call edges as (caller, callee) pairs.
    pub fn all_edges(&self) -> Vec<(String, String)> {
        self.edges
            .iter()
            .map(|&(from, to)| (self.nodes[from].clone(), self.nodes[to].clone()))
            .collect()
    }

    /// Get the petgraph DiGraph representation.
    fn to_digraph(&self) -> DiGraph<&str, ()> {
        let mut graph = DiGraph::new();
        let node_indices: Vec<NodeIndex> = self
            .nodes
            .iter()
            .map(|name| graph.add_node(name.as_str()))
            .collect();

        for &(from, to) in &self.edges {
            if from < node_indices.len() && to < node_indices.len() {
                graph.add_edge(node_indices[from], node_indices[to], ());
            }
        }
        graph
    }

    /// Find direct callers of a symbol.
    pub fn callers(&self, name: &str) -> Vec<&str> {
        let graph = self.to_digraph();
        let Some(&idx) = self.name_to_idx.get(name) else {
            // Try partial match
            return self.callers_partial(name);
        };

        let node_indices: Vec<NodeIndex> = (0..self.nodes.len()).map(NodeIndex::new).collect();

        graph
            .neighbors_directed(node_indices[idx], Direction::Incoming)
            .map(|ni| self.nodes[ni.index()].as_str())
            .collect()
    }

    /// Find direct callees of a symbol.
    pub fn callees(&self, name: &str) -> Vec<&str> {
        let graph = self.to_digraph();
        let Some(&idx) = self.name_to_idx.get(name) else {
            return self.callees_partial(name);
        };

        let node_indices: Vec<NodeIndex> = (0..self.nodes.len()).map(NodeIndex::new).collect();

        graph
            .neighbors_directed(node_indices[idx], Direction::Outgoing)
            .map(|ni| self.nodes[ni.index()].as_str())
            .collect()
    }

    /// Find callers by partial name match (e.g. "process_block" matches "AudioEngine::process_block").
    fn callers_partial(&self, partial: &str) -> Vec<&str> {
        // Find nodes whose name ends with the partial
        let targets: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.ends_with(partial) || n == &partial)
            .map(|(i, _)| i)
            .collect();

        let mut result = Vec::new();
        for &(from, to) in &self.edges {
            if targets.contains(&to) {
                result.push(self.nodes[from].as_str());
            }
        }
        result.sort();
        result.dedup();
        result
    }

    fn callees_partial(&self, partial: &str) -> Vec<&str> {
        let sources: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.ends_with(partial) || n == &partial)
            .map(|(i, _)| i)
            .collect();

        let mut result = Vec::new();
        for &(from, to) in &self.edges {
            if sources.contains(&from) {
                result.push(self.nodes[to].as_str());
            }
        }
        result.sort();
        result.dedup();
        result
    }

    /// Find transitive callers (up to max_depth).
    pub fn transitive_callers(&self, name: &str, max_depth: usize) -> Vec<(String, usize)> {
        let mut visited = HashMap::new();
        let mut queue = vec![(name.to_string(), 0usize)];

        while let Some((current, depth)) = queue.pop() {
            if depth > max_depth || visited.contains_key(&current) {
                continue;
            }
            visited.insert(current.clone(), depth);

            for caller in self.callers(&current) {
                if !visited.contains_key(caller) {
                    queue.push((caller.to_string(), depth + 1));
                }
            }
        }

        visited.remove(name);
        let mut result: Vec<_> = visited.into_iter().collect();
        result.sort_by_key(|(_, d)| *d);
        result
    }
}
