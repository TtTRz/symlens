use crate::graph::call_graph::CallGraph;
use std::collections::{HashMap, VecDeque};

/// Find the shortest path between two symbols in the call graph.
/// Uses bidirectional BFS: forward from source (callees) and backward from target (callers).
pub fn find_path(graph: &CallGraph, from: &str, to: &str) -> Option<Vec<String>> {
    let from_name = find_node(graph, from)?;
    let to_name = find_node(graph, to)?;

    if from_name == to_name {
        return Some(vec![from_name]);
    }

    // Forward BFS state (source → callees + callers)
    let mut fwd_visited: HashMap<String, Option<String>> = HashMap::new();
    let mut fwd_queue: VecDeque<String> = VecDeque::new();
    fwd_visited.insert(from_name.clone(), None);
    fwd_queue.push_back(from_name);

    // Backward BFS state (target → callees + callers)
    let mut bwd_visited: HashMap<String, Option<String>> = HashMap::new();
    let mut bwd_queue: VecDeque<String> = VecDeque::new();
    bwd_visited.insert(to_name.clone(), None);
    bwd_queue.push_back(to_name);

    loop {
        // Expand forward frontier
        if let Some(meeting) = bfs_step(&mut fwd_queue, &mut fwd_visited, &bwd_visited, graph) {
            return Some(reconstruct_bidir(&fwd_visited, &bwd_visited, &meeting));
        }
        // Expand backward frontier
        if let Some(meeting) = bfs_step(&mut bwd_queue, &mut bwd_visited, &fwd_visited, graph) {
            return Some(reconstruct_bidir(&fwd_visited, &bwd_visited, &meeting));
        }
        // Both frontiers exhausted
        if fwd_queue.is_empty() && bwd_queue.is_empty() {
            return None;
        }
    }
}

/// Expand one level of BFS. Returns Some(meeting_node) if the other side is reached.
fn bfs_step(
    queue: &mut VecDeque<String>,
    visited: &mut HashMap<String, Option<String>>,
    other_visited: &HashMap<String, Option<String>>,
    graph: &CallGraph,
) -> Option<String> {
    let level_size = queue.len();
    for _ in 0..level_size {
        let Some(current) = queue.pop_front() else {
            break;
        };
        // Explore both callees and callers (undirected connectivity)
        let neighbors: Vec<&str> = graph
            .callees(&current)
            .into_iter()
            .chain(graph.callers(&current))
            .collect();
        for next in neighbors {
            let next_s = next.to_string();
            if !visited.contains_key(&next_s) {
                visited.insert(next_s.clone(), Some(current.clone()));
                if other_visited.contains_key(&next_s) {
                    return Some(next_s);
                }
                queue.push_back(next_s);
            }
        }
    }
    None
}

/// Reconstruct the full path from forward + backward parent maps through the meeting point.
fn reconstruct_bidir(
    fwd: &HashMap<String, Option<String>>,
    bwd: &HashMap<String, Option<String>>,
    meeting: &str,
) -> Vec<String> {
    // Forward path: source → meeting
    let mut forward_path = vec![meeting.to_string()];
    let mut node = meeting.to_string();
    while let Some(Some(parent)) = fwd.get(&node) {
        forward_path.push(parent.clone());
        node = parent.clone();
    }
    forward_path.reverse();

    // Backward path: meeting → target
    let mut node = meeting.to_string();
    while let Some(Some(parent)) = bwd.get(&node) {
        forward_path.push(parent.clone());
        node = parent.clone();
    }
    forward_path
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
