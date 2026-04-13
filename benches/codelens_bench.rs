use criterion::{Criterion, criterion_group, criterion_main};
use std::path::Path;

/// Benchmark parsing a single Rust file with tree-sitter.
fn bench_parse_rust_file(c: &mut Criterion) {
    let source = include_bytes!("../src/graph/call_graph.rs");
    let parser = codelens::parser::rust::RustParser;

    c.bench_function("parse_rust_file", |b| {
        b.iter(|| {
            codelens::parser::traits::LanguageParser::extract_symbols(
                &parser,
                source,
                Path::new("call_graph.rs"),
            )
            .unwrap()
        })
    });
}

/// Benchmark indexing the codelens project itself.
fn bench_index_project(c: &mut Criterion) {
    let root = std::env::current_dir().unwrap();

    c.bench_function("index_project", |b| {
        b.iter(|| codelens::index::indexer::index_project(&root, 100_000).unwrap())
    });
}

/// Benchmark ProjectIndex::search() with pre-cached lowercase.
fn bench_search(c: &mut Criterion) {
    let root = std::env::current_dir().unwrap();
    let result = codelens::index::indexer::index_project(&root, 100_000).unwrap();
    let index = result.index;

    c.bench_function("search_CallGraph", |b| {
        b.iter(|| index.search("CallGraph", 20))
    });

    c.bench_function("search_extract", |b| b.iter(|| index.search("extract", 20)));
}

/// Benchmark CallGraph::callers() with cached DiGraph.
fn bench_callers(c: &mut Criterion) {
    let root = std::env::current_dir().unwrap();
    let result = codelens::index::indexer::index_project(&root, 100_000).unwrap();
    let graph = result.index.call_graph.unwrap();

    // Find a node that exists in the graph
    let target = graph.nodes.first().cloned().unwrap_or_default();

    c.bench_function("callers", |b| b.iter(|| graph.callers(&target)));

    c.bench_function("callees", |b| b.iter(|| graph.callees(&target)));
}

/// Benchmark transitive_callers with cached DiGraph.
fn bench_transitive_callers(c: &mut Criterion) {
    let root = std::env::current_dir().unwrap();
    let result = codelens::index::indexer::index_project(&root, 100_000).unwrap();
    let graph = result.index.call_graph.unwrap();

    let target = graph.nodes.first().cloned().unwrap_or_default();

    c.bench_function("transitive_callers_depth3", |b| {
        b.iter(|| graph.transitive_callers(&target, 3))
    });
}

/// Benchmark bidirectional BFS path finding.
fn bench_find_path(c: &mut Criterion) {
    let root = std::env::current_dir().unwrap();
    let result = codelens::index::indexer::index_project(&root, 100_000).unwrap();
    let graph = result.index.call_graph.unwrap();

    // Pick two nodes that are likely connected
    let from = graph.nodes.first().cloned().unwrap_or_default();
    let to = graph.nodes.last().cloned().unwrap_or_default();

    c.bench_function("find_path", |b| {
        b.iter(|| codelens::graph::path::find_path(&graph, &from, &to))
    });
}

criterion_group!(
    benches,
    bench_parse_rust_file,
    bench_index_project,
    bench_search,
    bench_callers,
    bench_transitive_callers,
    bench_find_path,
);
criterion_main!(benches);
