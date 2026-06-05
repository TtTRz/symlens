use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// A call graph built from extracted call edges.
#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraph {
    /// Node names (qualified symbol names)
    pub nodes: Vec<String>,
    /// Edges: (from_index, to_index)
    pub edges: Vec<(usize, usize)>,
    /// Name → node index
    name_to_idx: HashMap<String, usize>,
    /// Short name (last segment after `::`) → Vec<node indices>
    short_name_idx: HashMap<String, Vec<usize>>,
    /// Cached petgraph DiGraph (built once, reused for all queries)
    #[serde(skip)]
    digraph: Option<DiGraph<usize, ()>>,
    /// NodeIndex parallel to `nodes` vec (built with digraph)
    #[serde(skip)]
    node_indices: Vec<NodeIndex>,
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

        let mut graph_edges: Vec<(usize, usize)> = edges
            .iter()
            .map(|(caller, callee)| {
                let from = get_or_insert(caller);
                let to = get_or_insert(callee);
                (from, to)
            })
            .collect();

        // Deduplicate edges to avoid redundant graph traversals
        graph_edges.sort_unstable();
        graph_edges.dedup();

        let (digraph, node_indices) = Self::build_digraph(&nodes, &graph_edges);
        let short_name_idx = Self::build_short_name_idx(&nodes);

        CallGraph {
            nodes,
            edges: graph_edges,
            name_to_idx,
            short_name_idx,
            digraph: Some(digraph),
            node_indices,
        }
    }

    /// Build the petgraph DiGraph and NodeIndex mapping from nodes and edges.
    fn build_digraph(
        nodes: &[String],
        edges: &[(usize, usize)],
    ) -> (DiGraph<usize, ()>, Vec<NodeIndex>) {
        let mut g = DiGraph::with_capacity(nodes.len(), edges.len());
        let indices: Vec<NodeIndex> = (0..nodes.len()).map(|i| g.add_node(i)).collect();
        for &(from, to) in edges {
            if from < indices.len() && to < indices.len() {
                g.add_edge(indices[from], indices[to], ());
            }
        }
        (g, indices)
    }

    /// Build short name → Vec<node index> index for fast partial matching.
    /// The short name is the last segment after `::` (or the full name if no `::`).
    fn build_short_name_idx(nodes: &[String]) -> HashMap<String, Vec<usize>> {
        let mut idx: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, name) in nodes.iter().enumerate() {
            let short = name.rsplit("::").next().unwrap_or(name);
            idx.entry(short.to_string()).or_default().push(i);
        }
        idx
    }

    /// Find node indices matching a partial name using the short_name_idx.
    /// Returns matching indices for suffix/short-name lookups.
    pub fn find_nodes_partial(&self, partial: &str) -> Vec<usize> {
        // Fast path: exact short name match
        if let Some(indices) = self.short_name_idx.get(partial) {
            return indices.clone();
        }
        // Fallback: linear scan for substring / suffix match
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.ends_with(partial) || n.contains(partial))
            .map(|(i, _)| i)
            .collect()
    }

    /// Rebuild the name_to_idx and cached digraph after deserialization.
    pub fn rebuild_index(&mut self) {
        self.name_to_idx.clear();
        for (i, name) in self.nodes.iter().enumerate() {
            self.name_to_idx.insert(name.clone(), i);
        }
        self.short_name_idx = Self::build_short_name_idx(&self.nodes);
        self.rebuild_digraph();
    }

    /// Rebuild only the petgraph DiGraph from nodes and edges.
    /// Used after deserialization of v2+ indices where name_to_idx and
    /// short_name_idx are already serialized.
    pub fn rebuild_digraph(&mut self) {
        let (digraph, node_indices) = Self::build_digraph(&self.nodes, &self.edges);
        self.digraph = Some(digraph);
        self.node_indices = node_indices;
    }

    /// Look up exact node index by qualified name.
    pub fn exact_index(&self, name: &str) -> Option<usize> {
        self.name_to_idx.get(name).copied()
    }

    /// Returns true if the name index has been populated.
    pub fn has_name_index(&self) -> bool {
        !self.name_to_idx.is_empty()
    }

    /// Get all call edges as (from_index, to_index) pairs.
    /// Use `self.nodes[idx]` to resolve names.
    pub fn all_edges(&self) -> &[(usize, usize)] {
        &self.edges
    }

    /// Find direct callers of a symbol.
    pub fn callers(&self, name: &str) -> Vec<&str> {
        let Some(&idx) = self.name_to_idx.get(name) else {
            return self.callers_partial(name);
        };
        let Some(ref graph) = self.digraph else {
            return self.callers_partial(name);
        };
        graph
            .neighbors_directed(self.node_indices[idx], Direction::Incoming)
            .map(|ni| self.nodes[graph[ni]].as_str())
            .collect()
    }

    /// Find direct callees of a symbol.
    pub fn callees(&self, name: &str) -> Vec<&str> {
        let Some(&idx) = self.name_to_idx.get(name) else {
            return self.callees_partial(name);
        };
        let Some(ref graph) = self.digraph else {
            return self.callees_partial(name);
        };
        graph
            .neighbors_directed(self.node_indices[idx], Direction::Outgoing)
            .map(|ni| self.nodes[graph[ni]].as_str())
            .collect()
    }

    /// Find callers by partial name match (e.g. "process_block" matches "AudioEngine::process_block").
    fn callers_partial(&self, partial: &str) -> Vec<&str> {
        let targets: HashSet<usize> = self.find_nodes_partial(partial).into_iter().collect();

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
        let sources: HashSet<usize> = self.find_nodes_partial(partial).into_iter().collect();

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

    /// Find transitive callers (up to max_depth) using cached digraph indices.
    pub fn transitive_callers(&self, name: &str, max_depth: usize) -> Vec<(String, usize)> {
        let Some(&start_idx) = self.name_to_idx.get(name) else {
            return vec![];
        };
        let Some(ref graph) = self.digraph else {
            return vec![];
        };

        let mut visited: HashMap<usize, usize> = HashMap::new();
        let mut queue = VecDeque::from([(start_idx, 0usize)]);

        while let Some((current, depth)) = queue.pop_front() {
            if depth > max_depth || visited.contains_key(&current) {
                continue;
            }
            visited.insert(current, depth);

            if current < self.node_indices.len() {
                for neighbor in
                    graph.neighbors_directed(self.node_indices[current], Direction::Incoming)
                {
                    let ni = graph[neighbor];
                    if !visited.contains_key(&ni) {
                        queue.push_back((ni, depth + 1));
                    }
                }
            }
        }

        visited.remove(&start_idx);
        let mut result: Vec<_> = visited
            .into_iter()
            .map(|(idx, d)| (self.nodes[idx].clone(), d))
            .collect();
        result.sort_by_key(|(_, d)| *d);
        result
    }
}
