use crate::graph::call_graph::CallGraph;
use std::collections::{HashMap, HashSet, VecDeque};

/// Find the shortest path between two symbols in the call graph.
pub fn find_path(graph: &CallGraph, from: &str, to: &str) -> Option<Vec<String>> {
    // BFS from "from" following outgoing edges (callees)
    let mut visited: HashSet<String> = HashSet::new();
    let mut parent: HashMap<String, String> = HashMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    // Find actual node names (partial match)
    let from_name = find_node(graph, from)?;
    let to_name = find_node(graph, to)?;

    queue.push_back(from_name.clone());
    visited.insert(from_name.clone());

    while let Some(current) = queue.pop_front() {
        if current == to_name {
            // Reconstruct path
            let mut path = vec![to_name.clone()];
            let mut node = &to_name;
            while let Some(p) = parent.get(node) {
                path.push(p.clone());
                node = p;
            }
            path.reverse();
            return Some(path);
        }

        for callee in graph.callees(&current) {
            let callee_s = callee.to_string();
            if !visited.contains(&callee_s) {
                visited.insert(callee_s.clone());
                parent.insert(callee_s.clone(), current.clone());
                queue.push_back(callee_s);
            }
        }

        // Also search callers (bidirectional for finding connections)
        for caller in graph.callers(&current) {
            let caller_s = caller.to_string();
            if !visited.contains(&caller_s) {
                visited.insert(caller_s.clone());
                parent.insert(caller_s.clone(), current.clone());
                queue.push_back(caller_s);
            }
        }
    }

    None
}

fn find_node(graph: &CallGraph, partial: &str) -> Option<String> {
    // Exact match first
    if graph.nodes.contains(&partial.to_string()) {
        return Some(partial.to_string());
    }
    // Partial match (ends with)
    graph
        .nodes
        .iter()
        .find(|n| n.ends_with(partial) || n.contains(partial))
        .cloned()
}
