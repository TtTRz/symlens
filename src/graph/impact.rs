use crate::graph::call_graph::CallGraph;
use std::collections::{HashMap, HashSet};

/// Enhanced impact analysis result.
pub struct ImpactResult {
    pub target: String,
    pub direct_callers: Vec<String>,
    pub direct_callees: Vec<String>,
    pub transitive_callers: Vec<(String, usize)>, // (name, depth)
    pub transitive_callees: Vec<(String, usize)>, // (name, depth)
    /// Unique module names containing callers (blast radius spread)
    pub affected_modules: Vec<String>,
    /// Whether the target is part of a call cycle
    pub has_cycle: bool,
    /// Risk score 0.0-1.0 based on connectivity and reach
    pub risk_score: f32,
}

/// Analyze the impact of modifying a symbol.
pub fn analyze_impact(graph: &CallGraph, target: &str, max_depth: usize) -> ImpactResult {
    let direct_callers: Vec<String> = graph
        .callers(target)
        .iter()
        .map(|s| s.to_string())
        .collect();
    let direct_callees: Vec<String> = graph
        .callees(target)
        .iter()
        .map(|s| s.to_string())
        .collect();
    let transitive_callers = graph.transitive_callers(target, max_depth);
    let transitive_callees = compute_transitive_callees(graph, target, max_depth);

    // Affected modules: extract first segment from qualified names of all callers
    let mut module_set: HashSet<String> = HashSet::new();
    for (name, _) in &transitive_callers {
        if let Some(module) = extract_module(name) {
            module_set.insert(module);
        }
    }
    for name in &direct_callers {
        if let Some(module) = extract_module(name) {
            module_set.insert(module);
        }
    }
    let mut affected_modules: Vec<String> = module_set.into_iter().collect();
    affected_modules.sort();

    // Cycle detection
    let has_cycle = detect_cycle(graph, target);

    // Risk score: 0.0-1.0
    let total_dependents = direct_callers.len() + transitive_callers.len();
    let max_depth_reached = transitive_callers
        .iter()
        .map(|(_, d)| *d)
        .max()
        .unwrap_or(0);
    let module_spread = affected_modules.len();

    // Heuristic: log-scale to avoid saturation on large graphs
    let dependent_score = (total_dependents as f32 + 1.0).ln() / 6.0;
    let depth_score = if max_depth > 0 {
        max_depth_reached as f32 / max_depth as f32
    } else {
        0.0
    };
    let module_score = (module_spread as f32 + 1.0).ln() / 5.0;

    let risk_score = ((dependent_score + depth_score + module_score) / 3.0).min(1.0);

    ImpactResult {
        target: target.to_string(),
        direct_callers,
        direct_callees,
        transitive_callers,
        transitive_callees,
        affected_modules,
        has_cycle,
        risk_score,
    }
}

/// Compute transitive callees (BFS outward).
fn compute_transitive_callees(
    graph: &CallGraph,
    name: &str,
    max_depth: usize,
) -> Vec<(String, usize)> {
    let mut visited: HashMap<String, usize> = HashMap::new();
    let mut queue = vec![(name.to_string(), 0usize)];

    while let Some((current, depth)) = queue.pop() {
        if depth > max_depth || visited.contains_key(&current) {
            continue;
        }
        visited.insert(current.clone(), depth);

        for callee in graph.callees(&current) {
            if !visited.contains_key(callee) {
                queue.push((callee.to_string(), depth + 1));
            }
        }
    }

    visited.remove(name);
    let mut result: Vec<_> = visited.into_iter().collect();
    result.sort_by_key(|(_, d)| *d);
    result
}

/// Detect if target is part of a call cycle (A→B→...→A).
fn detect_cycle(graph: &CallGraph, target: &str) -> bool {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = graph
        .callees(target)
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    while let Some(current) = queue.pop() {
        if current == target {
            return true;
        }
        if !visited.insert(current.clone()) {
            continue;
        }
        for callee in graph.callees(&current) {
            if !visited.contains(callee) {
                queue.push(callee.to_string());
            }
        }
    }
    false
}

/// Extract module portion from a qualified name (first segment before "::").
fn extract_module(name: &str) -> Option<String> {
    name.split("::").next().map(|s| s.to_string())
}
