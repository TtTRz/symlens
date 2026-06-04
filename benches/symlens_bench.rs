use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use symlens::graph::call_graph::CallGraph;
use symlens::graph::deps::DepsGraph;
use symlens::graph::impact;
use symlens::graph::path;
use symlens::model::project::ProjectIndex;
use symlens::parser::registry::LanguageRegistry;
use symlens::parser::traits::LanguageParser;

// ─── Shared fixture setup ───────────────────────────────────────────

struct BenchData {
    index: ProjectIndex,
    graph: CallGraph,
    edges: Vec<(String, String)>,
}

static BENCH_DATA: OnceLock<BenchData> = OnceLock::new();

fn bench_data() -> &'static BenchData {
    BENCH_DATA.get_or_init(|| {
        let root = std::env::current_dir().unwrap();
        let result = symlens::index::indexer::index_project(&root, 100_000).unwrap();
        let index = result.index;
        let edges: Vec<(String, String)> = index
            .file_call_edges
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();
        let graph = CallGraph::build(&edges);
        BenchData {
            index,
            graph,
            edges,
        }
    })
}

/// Find a deterministic node that has known callers (heavily connected).
fn hub_node(graph: &CallGraph) -> String {
    let candidates = ["index_project", "run", "extract_symbols", "search"];
    for name in &candidates {
        if graph.exact_index(name).is_some() {
            return name.to_string();
        }
    }
    graph.nodes.first().cloned().unwrap_or_default()
}

/// A second node guaranteed to differ from the hub, for path-finding.
fn peer_node(graph: &CallGraph, hub: &str) -> String {
    graph
        .nodes
        .iter()
        .find(|n| *n != hub && !n.is_empty())
        .cloned()
        .unwrap_or_else(|| "nonexistent".to_string())
}

// ─── 1. Parsing ─────────────────────────────────────────────────────

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    let rust_source = include_bytes!("../src/graph/call_graph.rs");
    let rust_parser = symlens::parser::rust::RustParser;
    group.bench_function("rust_file", |b| {
        b.iter(|| {
            rust_parser
                .extract_symbols(rust_source, Path::new("call_graph.rs"))
                .unwrap()
        })
    });

    let registry = LanguageRegistry::new();
    let ts_source = include_bytes!("../tests/fixtures/sample.ts");
    let py_source = include_bytes!("../tests/fixtures/sample.py");
    let go_source = include_bytes!("../tests/fixtures/sample.go");

    group.bench_function("typescript_file", |b| {
        let parser = registry.parser_for(Path::new("sample.ts")).unwrap();
        b.iter(|| {
            parser
                .extract_symbols(ts_source, Path::new("sample.ts"))
                .unwrap()
        })
    });
    group.bench_function("python_file", |b| {
        let parser = registry.parser_for(Path::new("sample.py")).unwrap();
        b.iter(|| {
            parser
                .extract_symbols(py_source, Path::new("sample.py"))
                .unwrap()
        })
    });
    group.bench_function("go_file", |b| {
        let parser = registry.parser_for(Path::new("sample.go")).unwrap();
        b.iter(|| {
            parser
                .extract_symbols(go_source, Path::new("sample.go"))
                .unwrap()
        })
    });

    group.bench_function("registry_lookup", |b| {
        b.iter(|| registry.parser_for(Path::new("src/main.rs")).is_some())
    });

    group.finish();
}

// ─── 2. Indexing ────────────────────────────────────────────────────

fn bench_index(c: &mut Criterion) {
    let root = std::env::current_dir().unwrap();

    c.bench_function("index_project_full", |b| {
        b.iter(|| symlens::index::indexer::index_project(&root, 100_000).unwrap())
    });
}

// ─── 3. Search (in-memory) ──────────────────────────────────────────

fn bench_search(c: &mut Criterion) {
    let data = bench_data();
    let mut group = c.benchmark_group("search");

    // Exact name match — best case
    group.bench_function("exact_name", |b| {
        b.iter(|| data.index.search("CallGraph", 20))
    });

    // Partial name — scans more symbols
    group.bench_function("partial_name", |b| {
        b.iter(|| data.index.search("extract", 20))
    });

    // Doc comment hit — triggers to_lowercase on every symbol
    group.bench_function("doc_content", |b| {
        b.iter(|| data.index.search("audio processing", 20))
    });

    // No results — worst case, scans all symbols
    group.bench_function("miss", |b| {
        b.iter(|| data.index.search("zzz_nonexistent_xyz_123", 20))
    });

    // Parametric: result set size impact
    for limit in [1, 10, 50, 200] {
        group.bench_with_input(BenchmarkId::new("limit", limit), &limit, |b, &lim| {
            b.iter(|| data.index.search("fn", lim))
        });
    }

    group.finish();
}

// ─── 4. Call graph queries ──────────────────────────────────────────

fn bench_call_graph(c: &mut Criterion) {
    let data = bench_data();
    let target = hub_node(&data.graph);
    let peer = peer_node(&data.graph, &target);
    let mut group = c.benchmark_group("call_graph");

    group.bench_function("callers_exact", |b| b.iter(|| data.graph.callers(&target)));

    group.bench_function("callees_exact", |b| b.iter(|| data.graph.callees(&target)));

    let short_name = target.rsplit("::").next().unwrap_or(&target);
    group.bench_function("callers_partial", |b| {
        b.iter(|| data.graph.callers(short_name))
    });

    for depth in [1, 3, 5] {
        group.bench_with_input(
            BenchmarkId::new("transitive_callers", depth),
            &depth,
            |b, &d| b.iter(|| data.graph.transitive_callers(&target, d)),
        );
    }

    group.bench_function("find_path", |b| {
        b.iter(|| path::find_path(&data.graph, &target, &peer))
    });

    group.bench_function("impact_analysis", |b| {
        b.iter(|| impact::analyze_impact(&data.graph, &target, 3))
    });

    group.bench_function("build_graph", |b| b.iter(|| CallGraph::build(&data.edges)));

    group.finish();
}

// ─── 5. Dependency graph & cycles ───────────────────────────────────

fn bench_deps(c: &mut Criterion) {
    let mut group = c.benchmark_group("deps");

    // Small DAG — no cycles
    let small_dag = DepsGraph {
        edges: std::collections::BTreeMap::from([
            (
                PathBuf::from("a.rs"),
                std::collections::BTreeSet::from([PathBuf::from("b.rs")]),
            ),
            (
                PathBuf::from("b.rs"),
                std::collections::BTreeSet::from([PathBuf::from("c.rs")]),
            ),
            (
                PathBuf::from("d.rs"),
                std::collections::BTreeSet::from([PathBuf::from("e.rs")]),
            ),
        ]),
    };
    group.bench_function("has_cycle_dag", |b| {
        b.iter(|| small_dag.has_cycle_from(&PathBuf::from("a.rs")))
    });

    // Cycle present
    let cyclic = DepsGraph {
        edges: std::collections::BTreeMap::from([
            (
                PathBuf::from("a.rs"),
                std::collections::BTreeSet::from([PathBuf::from("b.rs")]),
            ),
            (
                PathBuf::from("b.rs"),
                std::collections::BTreeSet::from([PathBuf::from("c.rs")]),
            ),
            (
                PathBuf::from("c.rs"),
                std::collections::BTreeSet::from([PathBuf::from("a.rs")]),
            ),
        ]),
    };
    group.bench_function("has_cycle_cyclic", |b| {
        b.iter(|| cyclic.has_cycle_from(&PathBuf::from("a.rs")))
    });

    // Larger graph for detect_cycles
    let mut large_edges: std::collections::BTreeMap<PathBuf, std::collections::BTreeSet<PathBuf>> =
        std::collections::BTreeMap::new();
    for i in 0..100 {
        let from = PathBuf::from(format!("mod_{:03}.rs", i));
        let to = PathBuf::from(format!("mod_{:03}.rs", (i + 1) % 100));
        large_edges.entry(from).or_default().insert(to);
    }
    let large_dag = DepsGraph { edges: large_edges };
    group.bench_function("detect_cycles_100_nodes", |b| {
        b.iter(|| large_dag.detect_cycles())
    });

    group.finish();
}

// ─── 6. Refs (AST identifier scan) ──────────────────────────────────

fn bench_refs(c: &mut Criterion) {
    let registry = LanguageRegistry::new();
    let source = include_bytes!("../src/graph/call_graph.rs");
    let path = Path::new("call_graph.rs");
    let parser = registry.parser_for(path).unwrap();

    let mut group = c.benchmark_group("refs");

    group.bench_function("find_identifiers_frequent", |b| {
        b.iter(|| parser.find_identifiers(source, "graph").unwrap())
    });

    group.bench_function("find_identifiers_rare", |b| {
        b.iter(|| parser.find_identifiers(source, "BFS").unwrap())
    });

    group.bench_function("find_identifiers_miss", |b| {
        b.iter(|| parser.find_identifiers(source, "zzz_nonexistent").unwrap())
    });

    group.finish();
}

// ─── 7. Serialization roundtrip ─────────────────────────────────────

fn bench_serialization(c: &mut Criterion) {
    let data = bench_data();
    let mut group = c.benchmark_group("serde");

    group.bench_function("bincode_encode", |b| {
        b.iter(|| bincode::serde::encode_to_vec(&data.index, bincode::config::standard()).unwrap())
    });

    let encoded = bincode::serde::encode_to_vec(&data.index, bincode::config::standard()).unwrap();

    group.bench_function("bincode_decode", |b| {
        b.iter(|| {
            let (decoded, _): (ProjectIndex, _) =
                bincode::serde::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
            decoded
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_index,
    bench_search,
    bench_call_graph,
    bench_deps,
    bench_refs,
    bench_serialization,
);
criterion_main!(benches);
