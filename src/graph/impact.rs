use crate::graph::call_graph::CallGraph;

/// Impact analysis result.
pub struct ImpactResult {
    pub target: String,
    pub direct_callers: Vec<String>,
    pub direct_callees: Vec<String>,
    pub transitive_callers: Vec<(String, usize)>, // (name, depth)
}

/// Analyze the impact of modifying a symbol.
pub fn analyze_impact(graph: &CallGraph, target: &str, max_depth: usize) -> ImpactResult {
    let direct_callers: Vec<String> = graph.callers(target).iter().map(|s| s.to_string()).collect();
    let direct_callees: Vec<String> = graph.callees(target).iter().map(|s| s.to_string()).collect();
    let transitive_callers = graph.transitive_callers(target, max_depth);

    ImpactResult {
        target: target.to_string(),
        direct_callers,
        direct_callees,
        transitive_callers,
    }
}
