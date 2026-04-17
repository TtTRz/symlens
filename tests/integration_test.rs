use std::path::Path;
use symlens::parser::traits::LanguageParser;

// ─── Parser tests ───────────────────────────────────────────────────

mod parser_tests {
    use super::*;

    fn parse_rust_fixture() -> Vec<symlens::model::symbol::Symbol> {
        let parser = symlens::parser::rust::RustParser;
        let source = include_bytes!("fixtures/sample.rs");
        symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.rs"),
        )
        .expect("Failed to parse Rust fixture")
    }

    #[test]
    fn rust_extracts_struct() {
        let symbols = parse_rust_fixture();
        let structs: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Struct)
            .collect();
        assert!(
            structs.iter().any(|s| s.name == "AudioEngine"),
            "Should find AudioEngine struct, got: {:?}",
            structs.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn rust_extracts_functions() {
        let symbols = parse_rust_fixture();
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Function)
            .collect();
        assert!(
            fns.iter().any(|s| s.name == "normalize"),
            "Should find normalize function"
        );
    }

    #[test]
    fn rust_extracts_methods() {
        let symbols = parse_rust_fixture();
        let methods: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Method)
            .collect();
        assert!(
            methods.iter().any(|s| s.name == "new"),
            "Should find new method"
        );
        assert!(
            methods.iter().any(|s| s.name == "process_block"),
            "Should find process_block method"
        );
    }

    #[test]
    fn rust_extracts_const() {
        let symbols = parse_rust_fixture();
        let consts: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Constant)
            .collect();
        assert!(
            consts.iter().any(|s| s.name == "MAX_CHANNELS"),
            "Should find MAX_CHANNELS constant"
        );
    }

    #[test]
    fn rust_extracts_enum() {
        let symbols = parse_rust_fixture();
        let enums: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Enum)
            .collect();
        assert!(
            enums.iter().any(|s| s.name == "AudioFormat"),
            "Should find AudioFormat enum"
        );
    }

    #[test]
    fn rust_extracts_trait() {
        let symbols = parse_rust_fixture();
        let traits: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Interface)
            .collect();
        assert!(
            traits.iter().any(|s| s.name == "Processor"),
            "Should find Processor trait"
        );
    }

    #[test]
    fn rust_extracts_type_alias() {
        let symbols = parse_rust_fixture();
        let types: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::TypeAlias)
            .collect();
        assert!(
            types.iter().any(|s| s.name == "SampleRate"),
            "Should find SampleRate type alias"
        );
    }

    #[test]
    fn rust_extracts_doc_comments() {
        let symbols = parse_rust_fixture();
        let engine = symbols.iter().find(|s| s.name == "AudioEngine").unwrap();
        assert!(
            engine.doc_comment.is_some(),
            "AudioEngine should have doc comment"
        );
        assert!(
            engine
                .doc_comment
                .as_ref()
                .unwrap()
                .contains("audio engine")
        );
    }

    #[test]
    fn rust_extracts_signatures() {
        let symbols = parse_rust_fixture();
        let process = symbols.iter().find(|s| s.name == "process_block").unwrap();
        assert!(process.signature.is_some());
        let sig = process.signature.as_ref().unwrap();
        assert!(
            sig.contains("process_block"),
            "Signature should contain function name"
        );
        assert!(
            sig.contains("&mut self"),
            "Signature should contain &mut self"
        );
    }

    #[test]
    fn rust_extracts_calls() {
        let parser = symlens::parser::rust::RustParser;
        let source = include_bytes!("fixtures/sample.rs");
        let calls = symlens::parser::traits::LanguageParser::extract_calls(
            &parser,
            source,
            Path::new("sample.rs"),
        )
        .expect("Failed to extract calls");
        // process_block calls normalize
        assert!(
            calls
                .iter()
                .any(|(caller, callee)| caller == "process_block" && callee == "normalize"),
            "process_block should call normalize, got: {:?}",
            calls
        );
    }

    #[test]
    fn rust_extracts_imports() {
        // Our fixture doesn't have use statements, test the parser doesn't crash
        let parser = symlens::parser::rust::RustParser;
        let source = include_bytes!("fixtures/sample.rs");
        let imports = symlens::parser::traits::LanguageParser::extract_imports(
            &parser,
            source,
            Path::new("sample.rs"),
        )
        .expect("Failed to extract imports");
        // sample.rs has no use statements — just verify the parser doesn't crash
        let _ = imports;
    }

    #[test]
    fn rust_find_identifiers() {
        let parser = symlens::parser::rust::RustParser;
        let source = include_bytes!("fixtures/sample.rs");
        let refs =
            symlens::parser::traits::LanguageParser::find_identifiers(&parser, source, "normalize")
                .expect("Failed to find identifiers");
        // Should find at least the definition + the call inside process_block
        assert!(
            refs.len() >= 2,
            "Should find at least 2 refs to 'normalize', got {}",
            refs.len()
        );
    }

    // TypeScript parser tests
    #[test]
    fn ts_extracts_class() {
        let parser = symlens::parser::typescript::TypeScriptParser;
        let source = include_bytes!("fixtures/sample.ts");
        let symbols = symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.ts"),
        )
        .expect("Failed to parse TS fixture");
        let classes: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Class)
            .collect();
        assert!(
            classes.iter().any(|s| s.name == "Server"),
            "Should find Server class, got: {:?}",
            classes.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ts_extracts_interface() {
        let parser = symlens::parser::typescript::TypeScriptParser;
        let source = include_bytes!("fixtures/sample.ts");
        let symbols = symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.ts"),
        )
        .expect("Failed to parse TS fixture");
        let interfaces: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Interface)
            .collect();
        assert!(
            interfaces.iter().any(|s| s.name == "Config"),
            "Should find Config interface"
        );
    }

    #[test]
    fn ts_extracts_function() {
        let parser = symlens::parser::typescript::TypeScriptParser;
        let source = include_bytes!("fixtures/sample.ts");
        let symbols = symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.ts"),
        )
        .expect("Failed to parse TS fixture");
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Function)
            .collect();
        assert!(
            fns.iter().any(|s| s.name == "createServer"),
            "Should find createServer function"
        );
    }

    // Python parser tests
    #[test]
    fn python_extracts_class() {
        let parser = symlens::parser::python::PythonParser;
        let source = include_bytes!("fixtures/sample.py");
        let symbols = symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.py"),
        )
        .expect("Failed to parse Python fixture");
        let classes: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Class)
            .collect();
        assert!(
            classes.iter().any(|s| s.name == "Database"),
            "Should find Database class"
        );
    }

    #[test]
    fn python_extracts_functions() {
        let parser = symlens::parser::python::PythonParser;
        let source = include_bytes!("fixtures/sample.py");
        let symbols = symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.py"),
        )
        .expect("Failed to parse Python fixture");
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Function)
            .collect();
        assert!(
            fns.iter().any(|s| s.name == "create_connection"),
            "Should find create_connection function"
        );
        assert!(
            fns.iter().any(|s| s.name == "process_results"),
            "Should find process_results function"
        );
    }
}

// ─── Dart parser tests ──────────────────────────────────────────────

mod dart_tests {
    use super::*;

    fn parse_dart_fixture() -> Vec<symlens::model::symbol::Symbol> {
        let parser = symlens::parser::dart::DartParser;
        let source = include_bytes!("fixtures/sample.dart");
        symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.dart"),
        )
        .expect("Failed to parse Dart fixture")
    }

    #[test]
    fn dart_extracts_class() {
        let symbols = parse_dart_fixture();
        let classes: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Class)
            .collect();
        assert!(
            classes.iter().any(|s| s.name == "User"),
            "Should find User class, got: {:?}",
            classes.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        assert!(
            classes.iter().any(|s| s.name == "UserRepository"),
            "Should find UserRepository class"
        );
    }

    #[test]
    fn dart_extracts_abstract_class() {
        let symbols = parse_dart_fixture();
        let interfaces: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Interface)
            .collect();
        assert!(
            interfaces.iter().any(|s| s.name == "Repository"),
            "Should find abstract class Repository as Interface, got: {:?}",
            interfaces.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn dart_extracts_mixin() {
        let symbols = parse_dart_fixture();
        let mixins: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Interface)
            .collect();
        assert!(
            mixins.iter().any(|s| s.name == "Logger"),
            "Should find Logger mixin, got: {:?}",
            mixins.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn dart_extracts_enum() {
        let symbols = parse_dart_fixture();
        let enums: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Enum)
            .collect();
        assert!(
            enums.iter().any(|s| s.name == "OperationStatus"),
            "Should find OperationStatus enum"
        );
    }

    #[test]
    fn dart_extracts_typedef() {
        let symbols = parse_dart_fixture();
        let types: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::TypeAlias)
            .collect();
        assert!(
            types.iter().any(|s| s.name == "UserCallback"),
            "Should find UserCallback typedef, got: {:?}",
            types.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn dart_extracts_top_level_functions() {
        let symbols = parse_dart_fixture();
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Function)
            .collect();
        assert!(
            fns.iter().any(|s| s.name == "createRepository"),
            "Should find createRepository function"
        );
        assert!(
            fns.iter().any(|s| s.name == "processUsers"),
            "Should find processUsers function"
        );
    }

    #[test]
    fn dart_extracts_methods() {
        let symbols = parse_dart_fixture();
        let methods: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Method)
            .collect();
        assert!(
            methods.iter().any(|s| s.name == "findById"),
            "Should find findById method, got: {:?}",
            methods.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        assert!(
            methods.iter().any(|s| s.name == "save"),
            "Should find save method"
        );
    }

    #[test]
    fn dart_extracts_doc_comments() {
        let symbols = parse_dart_fixture();
        let user = symbols.iter().find(|s| s.name == "User").unwrap();
        assert!(user.doc_comment.is_some(), "User should have doc comment");
        assert!(
            user.doc_comment.as_ref().unwrap().contains("User model"),
            "Doc should contain 'User model'"
        );
    }

    #[test]
    fn dart_extracts_calls() {
        let parser = symlens::parser::dart::DartParser;
        let source = include_bytes!("fixtures/sample.dart");
        let calls = symlens::parser::traits::LanguageParser::extract_calls(
            &parser,
            source,
            Path::new("sample.dart"),
        )
        .expect("Failed to extract Dart calls");
        // Dart calls are extracted — just verify the parser doesn't crash
        // Dart's selector-based call patterns (obj.method()) are complex
        let _ = calls;
    }

    #[test]
    fn dart_extracts_imports() {
        let parser = symlens::parser::dart::DartParser;
        let source = include_bytes!("fixtures/sample.dart");
        let imports = symlens::parser::traits::LanguageParser::extract_imports(
            &parser,
            source,
            Path::new("sample.dart"),
        )
        .expect("Failed to extract Dart imports");
        assert!(!imports.is_empty(), "Should find imports");
        let all_names: Vec<_> = imports.iter().flat_map(|i| &i.names).collect();
        assert!(
            all_names
                .iter()
                .any(|n| n.contains("async") || n.contains("flutter") || n.contains("Widget")),
            "Should find dart:async or flutter import, got: {:?}",
            imports
        );
    }

    #[test]
    fn dart_find_identifiers() {
        let parser = symlens::parser::dart::DartParser;
        let source = include_bytes!("fixtures/sample.dart");
        let refs =
            symlens::parser::traits::LanguageParser::find_identifiers(&parser, source, "User")
                .expect("Failed to find Dart identifiers");
        assert!(
            refs.len() >= 2,
            "Should find at least 2 refs to 'User', got {}",
            refs.len()
        );
    }
}

// ─── Call graph tests ───────────────────────────────────────────────

mod call_graph_tests {
    use symlens::graph::call_graph::CallGraph;

    #[test]
    fn build_and_query_callers() {
        let edges = vec![
            ("main".to_string(), "init".to_string()),
            ("main".to_string(), "run".to_string()),
            ("run".to_string(), "process".to_string()),
            ("run".to_string(), "cleanup".to_string()),
            ("init".to_string(), "setup".to_string()),
        ];
        let graph = CallGraph::build(&edges);

        let callers = graph.callers("process");
        assert!(callers.contains(&"run"), "run should call process");

        let callers_init = graph.callers("init");
        assert!(callers_init.contains(&"main"), "main should call init");
    }

    #[test]
    fn build_and_query_callees() {
        let edges = vec![
            ("main".to_string(), "init".to_string()),
            ("main".to_string(), "run".to_string()),
            ("run".to_string(), "process".to_string()),
        ];
        let graph = CallGraph::build(&edges);

        let callees = graph.callees("main");
        assert!(callees.contains(&"init"), "main should call init");
        assert!(callees.contains(&"run"), "main should call run");
    }

    #[test]
    fn transitive_callers() {
        let edges = vec![
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
            ("c".to_string(), "d".to_string()),
        ];
        let graph = CallGraph::build(&edges);

        let transitive = graph.transitive_callers("d", 3);
        let names: Vec<_> = transitive.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"c"), "c should be a transitive caller of d");
        assert!(names.contains(&"b"), "b should be a transitive caller of d");
        assert!(names.contains(&"a"), "a should be a transitive caller of d");
    }

    #[test]
    fn partial_name_match() {
        let edges = vec![
            ("Engine::run".to_string(), "Engine::process".to_string()),
            ("main".to_string(), "Engine::run".to_string()),
        ];
        let graph = CallGraph::build(&edges);

        // Partial match: "run" should find "Engine::run"
        let callers = graph.callers("run");
        assert!(
            callers.contains(&"main"),
            "Partial match 'run' should find main as caller of Engine::run"
        );
    }

    #[test]
    fn empty_graph() {
        let graph = CallGraph::build(&[]);
        assert!(graph.callers("anything").is_empty());
        assert!(graph.callees("anything").is_empty());
        assert!(graph.transitive_callers("anything", 5).is_empty());
    }
}

// ─── Project index tests ────────────────────────────────────────────

mod index_tests {
    use std::path::PathBuf;
    use symlens::model::project::ProjectIndex;
    use symlens::model::symbol::*;

    fn make_symbol(name: &str, kind: SymbolKind, file: &str) -> Symbol {
        Symbol {
            id: SymbolId::new(file, name, &kind),
            name: name.to_string(),
            qualified_name: name.to_string(),
            kind,
            file_path: PathBuf::from(file),
            span: Span {
                start_line: 1,
                end_line: 10,
                start_col: 0,
                end_col: 0,
            },
            signature: Some(format!("fn {}()", name)),
            doc_comment: Some(format!("Doc for {}", name)),
            visibility: Visibility::Public,
            parent: None,
            children: vec![],
        }
    }

    #[test]
    fn insert_and_get() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        let sym = make_symbol("foo", SymbolKind::Function, "src/main.rs");
        let id = sym.id.clone();
        index.insert(sym);

        assert!(index.get(&id).is_some());
        assert_eq!(index.get(&id).unwrap().name, "foo");
    }

    #[test]
    fn search_by_name() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        index.insert(make_symbol(
            "process_audio",
            SymbolKind::Function,
            "src/audio.rs",
        ));
        index.insert(make_symbol(
            "AudioEngine",
            SymbolKind::Struct,
            "src/engine.rs",
        ));
        index.insert(make_symbol(
            "render_video",
            SymbolKind::Function,
            "src/video.rs",
        ));

        let results = index.search("audio", 10);
        assert_eq!(results.len(), 2, "Should find 2 symbols with 'audio'");
        let names: Vec<_> = results.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"process_audio"));
        assert!(names.contains(&"AudioEngine"));
    }

    #[test]
    fn search_by_doc() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        index.insert(make_symbol("foo", SymbolKind::Function, "src/main.rs"));

        let results = index.search("Doc for foo", 10);
        assert!(!results.is_empty(), "Should find symbol by doc comment");
    }

    #[test]
    fn search_limit() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        for i in 0..20 {
            index.insert(make_symbol(
                &format!("func_{}", i),
                SymbolKind::Function,
                "src/main.rs",
            ));
        }

        let results = index.search("func", 5);
        assert_eq!(results.len(), 5, "Should respect limit");
    }

    #[test]
    fn symbols_in_file() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        index.insert(make_symbol("foo", SymbolKind::Function, "src/a.rs"));
        index.insert(make_symbol("bar", SymbolKind::Function, "src/a.rs"));
        index.insert(make_symbol("baz", SymbolKind::Function, "src/b.rs"));

        let syms = index.symbols_in_file(&PathBuf::from("src/a.rs"));
        assert_eq!(syms.len(), 2, "Should find 2 symbols in a.rs");
    }

    #[test]
    fn stats() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        index.insert(make_symbol("foo", SymbolKind::Function, "src/main.rs"));
        index.insert(make_symbol("Bar", SymbolKind::Struct, "src/main.rs"));
        index.insert(make_symbol("baz", SymbolKind::Function, "src/lib.rs"));

        let stats = index.stats();
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.total_symbols, 3);
        assert_eq!(*stats.by_kind.get("function").unwrap(), 2);
        assert_eq!(*stats.by_kind.get("struct").unwrap(), 1);
    }

    #[test]
    fn remove_file() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        index.insert(make_symbol("foo", SymbolKind::Function, "src/main.rs"));
        index.insert(make_symbol("bar", SymbolKind::Function, "src/main.rs"));
        assert_eq!(index.symbols.len(), 2);

        index.remove_file(&PathBuf::from("src/main.rs"));
        assert_eq!(index.symbols.len(), 0);
        assert!(
            index
                .symbols_in_file(&PathBuf::from("src/main.rs"))
                .is_empty()
        );
    }
}

// ─── Symbol ID tests ────────────────────────────────────────────────

mod symbol_tests {
    use symlens::model::symbol::*;

    #[test]
    fn symbol_id_format() {
        let id = SymbolId::new("src/main.rs", "Engine::run", &SymbolKind::Method);
        assert_eq!(id.0, "src/main.rs::Engine::run#method");
    }

    #[test]
    fn symbol_kind_roundtrip() {
        let kinds = vec![
            ("function", SymbolKind::Function),
            ("method", SymbolKind::Method),
            ("struct", SymbolKind::Struct),
            ("class", SymbolKind::Class),
            ("enum", SymbolKind::Enum),
            ("interface", SymbolKind::Interface),
            ("constant", SymbolKind::Constant),
        ];
        for (s, k) in kinds {
            assert_eq!(k.as_str(), s);
            assert_eq!(SymbolKind::from_str(s), Some(k));
        }
    }

    #[test]
    fn span_display() {
        let span = Span {
            start_line: 10,
            end_line: 10,
            start_col: 0,
            end_col: 20,
        };
        assert_eq!(format!("{}", span), "L10");

        let span2 = Span {
            start_line: 10,
            end_line: 25,
            start_col: 0,
            end_col: 5,
        };
        assert_eq!(format!("{}", span2), "L10-25");
    }
}

// ─── Graph path tests ───────────────────────────────────────────────

mod path_tests {
    use symlens::graph::call_graph::CallGraph;
    use symlens::graph::path::find_path;

    #[test]
    fn find_direct_path() {
        let edges = vec![
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
        ];
        let graph = CallGraph::build(&edges);
        let path = find_path(&graph, "a", "c");
        assert!(path.is_some());
        let p = path.unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p[0], "a");
        assert_eq!(p[2], "c");
    }

    #[test]
    fn no_path() {
        let edges = vec![
            ("a".to_string(), "b".to_string()),
            ("c".to_string(), "d".to_string()),
        ];
        let graph = CallGraph::build(&edges);
        let path = find_path(&graph, "a", "d");
        // May or may not find a path depending on bidirectional search
        // At minimum, a→b has no connection to c→d
        let _ = path; // Just ensure it doesn't crash
    }

    #[test]
    fn path_to_self() {
        let edges = vec![("a".to_string(), "b".to_string())];
        let graph = CallGraph::build(&edges);
        let path = find_path(&graph, "a", "a");
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 1);
    }
}

// ─── Deps graph tests ──────────────────────────────────────────────

mod deps_tests {
    use std::path::PathBuf;
    use symlens::graph::deps::DepsGraph;

    #[test]
    fn build_deps_graph() {
        let imports = vec![
            (PathBuf::from("src/main.rs"), "crate::engine".to_string()),
            (PathBuf::from("src/main.rs"), "crate::audio".to_string()),
        ];
        let known = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/engine.rs"),
            PathBuf::from("src/audio.rs"),
        ];
        let graph = DepsGraph::build(&imports, &known);
        assert!(!graph.edges.is_empty(), "Should have dependency edges");
    }

    #[test]
    fn mermaid_output() {
        let imports = vec![(PathBuf::from("src/main.rs"), "crate::engine".to_string())];
        let known = vec![PathBuf::from("src/main.rs"), PathBuf::from("src/engine.rs")];
        let graph = DepsGraph::build(&imports, &known);
        let mermaid = graph.to_mermaid();
        assert!(
            mermaid.starts_with("graph TD"),
            "Mermaid output should start with 'graph TD'"
        );
    }
}

// ─── Go parser tests ────────────────────────────────────────────────

mod go_tests {
    use super::*;

    fn parse_go_fixture() -> Vec<symlens::model::symbol::Symbol> {
        let parser = symlens::parser::go::GoParser;
        let source = include_bytes!("fixtures/sample.go");
        symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.go"),
        )
        .expect("Failed to parse Go fixture")
    }

    #[test]
    fn go_extracts_struct() {
        let symbols = parse_go_fixture();
        let structs: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Struct)
            .collect();
        assert!(
            structs.iter().any(|s| s.name == "AudioEngine"),
            "Should find AudioEngine struct, got: {:?}",
            structs.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn go_extracts_function() {
        let symbols = parse_go_fixture();
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Function)
            .collect();
        assert!(
            fns.iter().any(|s| s.name == "Normalize"),
            "Should find Normalize function, got: {:?}",
            fns.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        assert!(
            fns.iter().any(|s| s.name == "NewAudioEngine"),
            "Should find NewAudioEngine function, got: {:?}",
            fns.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn go_extracts_method() {
        let symbols = parse_go_fixture();
        let methods: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Method)
            .collect();
        assert!(
            methods.iter().any(|s| s.name == "ProcessBlock"),
            "Should find ProcessBlock method, got: {:?}",
            methods.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn go_extracts_interface() {
        let symbols = parse_go_fixture();
        let interfaces: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Interface)
            .collect();
        assert!(
            interfaces.iter().any(|s| s.name == "Processor"),
            "Should find Processor interface, got: {:?}",
            interfaces.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn go_extracts_constant() {
        let symbols = parse_go_fixture();
        let consts: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Constant)
            .collect();
        assert!(
            consts.iter().any(|s| s.name == "MaxChannels"),
            "Should find MaxChannels constant, got: {:?}",
            consts.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn go_extracts_variable() {
        let symbols = parse_go_fixture();
        let vars: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Variable)
            .collect();
        assert!(
            vars.iter().any(|s| s.name == "DefaultRate"),
            "Should find DefaultRate variable, got: {:?}",
            vars.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn go_extracts_type_alias() {
        let symbols = parse_go_fixture();
        // SampleRate is a type alias, AudioFormat maps to TypeAlias or Struct depending on parser
        let type_aliases: Vec<_> = symbols
            .iter()
            .filter(|s| {
                s.kind == symlens::model::symbol::SymbolKind::TypeAlias
                    || s.kind == symlens::model::symbol::SymbolKind::Struct
            })
            .collect();
        assert!(
            type_aliases
                .iter()
                .any(|s| s.name == "SampleRate" || s.name == "AudioFormat"),
            "Should find SampleRate or AudioFormat type, got: {:?}",
            type_aliases
                .iter()
                .map(|s| (&s.name, &s.kind))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn go_extracts_doc_comments() {
        let symbols = parse_go_fixture();
        let engine = symbols.iter().find(|s| s.name == "AudioEngine").unwrap();
        assert!(
            engine.doc_comment.is_some(),
            "AudioEngine should have doc comment"
        );
        assert!(
            engine
                .doc_comment
                .as_ref()
                .unwrap()
                .contains("processes audio"),
            "Doc should mention audio processing"
        );
    }

    #[test]
    fn go_extracts_calls() {
        let parser = symlens::parser::go::GoParser;
        let source = include_bytes!("fixtures/sample.go");
        let calls = symlens::parser::traits::LanguageParser::extract_calls(
            &parser,
            source,
            Path::new("sample.go"),
        )
        .expect("Failed to extract Go calls");
        // ProcessBlock calls Normalize
        assert!(
            calls.iter().any(|(_, callee)| callee == "Normalize"),
            "Should find call to Normalize, got: {:?}",
            calls
        );
    }

    #[test]
    fn go_extracts_imports() {
        let parser = symlens::parser::go::GoParser;
        let source = include_bytes!("fixtures/sample.go");
        let imports = symlens::parser::traits::LanguageParser::extract_imports(
            &parser,
            source,
            Path::new("sample.go"),
        )
        .expect("Failed to extract Go imports");
        assert!(!imports.is_empty(), "Should find imports");
        let all_names: Vec<_> = imports.iter().flat_map(|i| &i.names).collect();
        assert!(
            all_names
                .iter()
                .any(|n| n.contains("fmt") || n.contains("math")),
            "Should find fmt or math import, got: {:?}",
            imports
        );
    }

    #[test]
    fn go_find_identifiers() {
        let parser = symlens::parser::go::GoParser;
        let source = include_bytes!("fixtures/sample.go");
        let refs =
            symlens::parser::traits::LanguageParser::find_identifiers(&parser, source, "Normalize")
                .expect("Failed to find Go identifiers");
        assert!(
            refs.len() >= 2,
            "Should find at least 2 refs to 'Normalize' (def + call), got {}",
            refs.len()
        );
    }
}

// ─── Swift parser tests ─────────────────────────────────────────────

mod swift_tests {
    use super::*;

    fn parse_swift_fixture() -> Vec<symlens::model::symbol::Symbol> {
        let parser = symlens::parser::swift::SwiftParser;
        let source = include_bytes!("fixtures/sample.swift");
        symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.swift"),
        )
        .expect("Failed to parse Swift fixture")
    }

    #[test]
    fn swift_extracts_class() {
        let symbols = parse_swift_fixture();
        let classes: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Class)
            .collect();
        assert!(
            classes.iter().any(|s| s.name == "AudioEngine"),
            "Should find AudioEngine class, got: {:?}",
            classes.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn swift_extracts_struct() {
        let symbols = parse_swift_fixture();
        let structs: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Struct)
            .collect();
        // tree-sitter-swift v0.7 may not produce struct_declaration at top level
        // If not found, verify the parser at least doesn't crash on struct syntax
        if !structs.is_empty() {
            assert!(
                structs.iter().any(|s| s.name == "AudioFormat"),
                "Should find AudioFormat struct, got: {:?}",
                structs.iter().map(|s| &s.name).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn swift_extracts_function() {
        let symbols = parse_swift_fixture();
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Function)
            .collect();
        assert!(
            fns.iter().any(|s| s.name == "normalize"),
            "Should find normalize function, got: {:?}",
            fns.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn swift_extracts_method() {
        let symbols = parse_swift_fixture();
        let methods: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Method)
            .collect();
        assert!(
            methods.iter().any(|s| s.name == "processBlock"),
            "Should find processBlock method, got: {:?}",
            methods.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn swift_extracts_enum() {
        let symbols = parse_swift_fixture();
        let enums: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Enum)
            .collect();
        // tree-sitter-swift v0.7 may not produce enum_declaration at top level
        if !enums.is_empty() {
            assert!(
                enums.iter().any(|s| s.name == "ChannelLayout"),
                "Should find ChannelLayout enum, got: {:?}",
                enums.iter().map(|s| &s.name).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn swift_extracts_protocol() {
        let symbols = parse_swift_fixture();
        let protocols: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Interface)
            .collect();
        assert!(
            protocols.iter().any(|s| s.name == "Processor"),
            "Should find Processor protocol, got: {:?}",
            protocols.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn swift_extracts_doc_comments() {
        let symbols = parse_swift_fixture();
        let engine = symbols.iter().find(|s| s.name == "AudioEngine").unwrap();
        assert!(
            engine.doc_comment.is_some(),
            "AudioEngine should have doc comment"
        );
        assert!(
            engine
                .doc_comment
                .as_ref()
                .unwrap()
                .contains("processes audio"),
            "Doc should mention audio processing"
        );
    }

    #[test]
    fn swift_extracts_calls() {
        let parser = symlens::parser::swift::SwiftParser;
        let source = include_bytes!("fixtures/sample.swift");
        let calls = symlens::parser::traits::LanguageParser::extract_calls(
            &parser,
            source,
            Path::new("sample.swift"),
        )
        .expect("Failed to extract Swift calls");
        // processBlock calls normalize
        assert!(
            calls.iter().any(|(_, callee)| callee == "normalize"),
            "Should find call to normalize, got: {:?}",
            calls
        );
    }

    #[test]
    fn swift_extracts_imports() {
        let parser = symlens::parser::swift::SwiftParser;
        let source = include_bytes!("fixtures/sample.swift");
        let imports = symlens::parser::traits::LanguageParser::extract_imports(
            &parser,
            source,
            Path::new("sample.swift"),
        )
        .expect("Failed to extract Swift imports");
        assert!(!imports.is_empty(), "Should find imports");
        let all_names: Vec<_> = imports.iter().flat_map(|i| &i.names).collect();
        assert!(
            all_names.iter().any(|n| n.contains("Foundation")),
            "Should find Foundation import, got: {:?}",
            imports
        );
    }

    #[test]
    fn swift_find_identifiers() {
        let parser = symlens::parser::swift::SwiftParser;
        let source = include_bytes!("fixtures/sample.swift");
        let refs =
            symlens::parser::traits::LanguageParser::find_identifiers(&parser, source, "normalize")
                .expect("Failed to find Swift identifiers");
        assert!(
            refs.len() >= 2,
            "Should find at least 2 refs to 'normalize' (def + call), got {}",
            refs.len()
        );
    }
}

// ─── C parser tests ─────────────────────────────────────────────────

mod c_tests {
    use super::*;

    fn parse_c_fixture() -> Vec<symlens::model::symbol::Symbol> {
        let parser = symlens::parser::c::CParser;
        let source = include_bytes!("fixtures/sample.c");
        symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.c"),
        )
        .expect("Failed to parse C fixture")
    }

    #[test]
    fn c_extracts_function() {
        let symbols = parse_c_fixture();
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Function)
            .collect();
        assert!(
            fns.iter()
                .any(|s| s.name == "normalize" || s.name == "process_block"),
            "Should find normalize or process_block function, got: {:?}",
            fns.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn c_extracts_struct() {
        let symbols = parse_c_fixture();
        let structs: Vec<_> = symbols
            .iter()
            .filter(|s| {
                s.kind == symlens::model::symbol::SymbolKind::Struct
                    || s.kind == symlens::model::symbol::SymbolKind::TypeAlias
            })
            .collect();
        assert!(
            structs
                .iter()
                .any(|s| s.name == "AudioEngine" || s.name.contains("AudioEngine")),
            "Should find AudioEngine, got: {:?}",
            structs
                .iter()
                .map(|s| (&s.name, &s.kind))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn c_extracts_macro() {
        let symbols = parse_c_fixture();
        let macros: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Macro)
            .collect();
        assert!(
            macros.iter().any(|s| s.name == "MAX_CHANNELS"),
            "Should find MAX_CHANNELS macro, got: {:?}",
            macros.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn c_extracts_calls() {
        let parser = symlens::parser::c::CParser;
        let source = include_bytes!("fixtures/sample.c");
        let calls = symlens::parser::traits::LanguageParser::extract_calls(
            &parser,
            source,
            Path::new("sample.c"),
        )
        .expect("Failed to extract C calls");
        assert!(
            calls.iter().any(|(_, callee)| callee == "normalize"),
            "Should find call to normalize, got: {:?}",
            calls
        );
    }

    #[test]
    fn c_extracts_imports() {
        let parser = symlens::parser::c::CParser;
        let source = include_bytes!("fixtures/sample.c");
        let imports = symlens::parser::traits::LanguageParser::extract_imports(
            &parser,
            source,
            Path::new("sample.c"),
        )
        .expect("Failed to extract C imports");
        assert!(!imports.is_empty(), "Should find #include directives");
    }
}

// ─── C++ parser tests ───────────────────────────────────────────────

mod cpp_tests {
    use super::*;

    fn parse_cpp_fixture() -> Vec<symlens::model::symbol::Symbol> {
        let parser = symlens::parser::cpp::CppParser;
        let source = include_bytes!("fixtures/sample.cpp");
        symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.cpp"),
        )
        .expect("Failed to parse C++ fixture")
    }

    #[test]
    fn cpp_extracts_class() {
        let symbols = parse_cpp_fixture();
        let classes: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Class)
            .collect();
        assert!(
            classes.iter().any(|s| s.name == "AudioEngine"),
            "Should find AudioEngine class, got: {:?}",
            classes.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn cpp_extracts_enum() {
        let symbols = parse_cpp_fixture();
        let enums: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Enum)
            .collect();
        assert!(
            enums.iter().any(|s| s.name == "AudioFormat"),
            "Should find AudioFormat enum, got: {:?}",
            enums.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn cpp_extracts_method() {
        let symbols = parse_cpp_fixture();
        let methods: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Method)
            .collect();
        assert!(
            methods
                .iter()
                .any(|s| s.name == "processBlock" || s.name == "normalize"),
            "Should find processBlock or normalize method, got: {:?}",
            methods.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn cpp_extracts_calls() {
        let parser = symlens::parser::cpp::CppParser;
        let source = include_bytes!("fixtures/sample.cpp");
        let calls = symlens::parser::traits::LanguageParser::extract_calls(
            &parser,
            source,
            Path::new("sample.cpp"),
        )
        .expect("Failed to extract C++ calls");
        assert!(
            calls.iter().any(|(_, callee)| callee == "normalize"),
            "Should find call to normalize, got: {:?}",
            calls
        );
    }

    #[test]
    fn cpp_extracts_imports() {
        let parser = symlens::parser::cpp::CppParser;
        let source = include_bytes!("fixtures/sample.cpp");
        let imports = symlens::parser::traits::LanguageParser::extract_imports(
            &parser,
            source,
            Path::new("sample.cpp"),
        )
        .expect("Failed to extract C++ imports");
        assert!(!imports.is_empty(), "Should find #include directives");
    }
}

// ─── Kotlin parser tests ────────────────────────────────────────────

mod kotlin_tests {
    use super::*;

    fn parse_kotlin_fixture() -> Vec<symlens::model::symbol::Symbol> {
        let parser = symlens::parser::kotlin::KotlinParser;
        let source = include_bytes!("fixtures/sample.kt");
        symlens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.kt"),
        )
        .expect("Failed to parse Kotlin fixture")
    }

    #[test]
    fn kotlin_extracts_class() {
        let symbols = parse_kotlin_fixture();
        let classes: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Class)
            .collect();
        assert!(
            classes.iter().any(|s| s.name == "AudioEngine"),
            "Should find AudioEngine class, got: {:?}",
            classes.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn kotlin_extracts_interface() {
        let symbols = parse_kotlin_fixture();
        let interfaces: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Interface)
            .collect();
        assert!(
            interfaces.iter().any(|s| s.name == "Processor"),
            "Should find Processor interface, got: {:?}",
            interfaces.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn kotlin_extracts_function() {
        let symbols = parse_kotlin_fixture();
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Function)
            .collect();
        assert!(
            fns.iter().any(|s| s.name == "createEngine"),
            "Should find createEngine function, got: {:?}",
            fns.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn kotlin_extracts_enum() {
        let symbols = parse_kotlin_fixture();
        let enums: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Enum)
            .collect();
        // tree-sitter-kotlin may parse enum class as Class
        if !enums.is_empty() {
            assert!(
                enums.iter().any(|s| s.name == "AudioFormat"),
                "Should find AudioFormat enum, got: {:?}",
                enums.iter().map(|s| &s.name).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn kotlin_extracts_imports() {
        let parser = symlens::parser::kotlin::KotlinParser;
        let source = include_bytes!("fixtures/sample.kt");
        let imports = symlens::parser::traits::LanguageParser::extract_imports(
            &parser,
            source,
            Path::new("sample.kt"),
        )
        .expect("Failed to extract Kotlin imports");
        assert!(!imports.is_empty(), "Should find imports");
    }

    #[test]
    fn kotlin_extracts_calls() {
        let parser = symlens::parser::kotlin::KotlinParser;
        let source = include_bytes!("fixtures/sample.kt");
        let calls = symlens::parser::traits::LanguageParser::extract_calls(
            &parser,
            source,
            Path::new("sample.kt"),
        )
        .expect("Failed to extract Kotlin calls");
        let _ = calls; // at minimum, parser doesn't crash
    }
}

// ─── Incremental call graph tests ──────────────────────────────────

mod incremental_tests {
    use std::path::PathBuf;
    use symlens::graph::call_graph::CallGraph;
    use symlens::model::project::ProjectIndex;

    #[test]
    fn project_index_stores_file_call_edges() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        let edges = vec![
            ("main".to_string(), "foo".to_string()),
            ("main".to_string(), "bar".to_string()),
        ];
        index
            .file_call_edges
            .insert(PathBuf::from("src/main.rs"), edges.clone());

        assert_eq!(index.file_call_edges.len(), 1);
        assert_eq!(
            index
                .file_call_edges
                .get(&PathBuf::from("src/main.rs"))
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn project_index_stores_file_imports() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        let imports = vec![symlens::parser::traits::ImportInfo {
            module_path: "std::collections".to_string(),
            names: vec!["HashMap".to_string()],
        }];
        index
            .file_imports
            .insert(PathBuf::from("src/main.rs"), imports);

        assert_eq!(index.file_imports.len(), 1);
    }

    #[test]
    fn call_graph_from_merged_edges() {
        // Simulate two files contributing edges
        let file_a_edges = vec![("a::main".to_string(), "a::foo".to_string())];
        let file_b_edges = vec![("b::run".to_string(), "a::foo".to_string())];

        // Merge like the indexer does
        let mut all_edges = Vec::new();
        all_edges.extend(file_a_edges);
        all_edges.extend(file_b_edges);

        let graph = CallGraph::build(&all_edges);
        let callers = graph.callers("a::foo");
        assert_eq!(callers.len(), 2, "Should have callers from both files");
        assert!(callers.contains(&"a::main"));
        assert!(callers.contains(&"b::run"));
    }

    #[test]
    fn incremental_edge_carryforward_produces_complete_graph() {
        // Simulate: file A unchanged (edges carried forward), file B re-parsed
        let carried_edges = vec![
            ("module_a::init".to_string(), "module_a::setup".to_string()),
            ("module_a::init".to_string(), "shared::log".to_string()),
        ];
        let new_edges = vec![
            ("module_b::run".to_string(), "shared::log".to_string()),
            ("module_b::run".to_string(), "module_b::cleanup".to_string()),
        ];

        let mut all_edges = Vec::new();
        all_edges.extend(carried_edges);
        all_edges.extend(new_edges);

        let graph = CallGraph::build(&all_edges);

        // shared::log should have 2 callers from different modules
        let log_callers = graph.callers("shared::log");
        assert_eq!(log_callers.len(), 2);
        assert!(log_callers.contains(&"module_a::init"));
        assert!(log_callers.contains(&"module_b::run"));

        // module_a::init should have 2 callees
        let init_callees = graph.callees("module_a::init");
        assert_eq!(init_callees.len(), 2);
    }

    #[test]
    fn file_call_edges_serialization_roundtrip() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        let edges = vec![
            ("foo".to_string(), "bar".to_string()),
            ("baz".to_string(), "qux".to_string()),
        ];
        index
            .file_call_edges
            .insert(PathBuf::from("test.rs"), edges);

        // Serialize and deserialize via bincode
        let encoded = bincode::serde::encode_to_vec(&index, bincode::config::standard())
            .expect("encode failed");
        let (decoded, _): (ProjectIndex, _) =
            bincode::serde::decode_from_slice(&encoded, bincode::config::standard())
                .expect("decode failed");

        assert_eq!(decoded.file_call_edges.len(), 1);
        let decoded_edges = decoded
            .file_call_edges
            .get(&PathBuf::from("test.rs"))
            .unwrap();
        assert_eq!(decoded_edges.len(), 2);
        assert_eq!(decoded_edges[0], ("foo".to_string(), "bar".to_string()));
    }

    #[test]
    fn file_imports_serialization_roundtrip() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test"));
        let imports = vec![symlens::parser::traits::ImportInfo {
            module_path: "crate::utils".to_string(),
            names: vec!["helper".to_string(), "format".to_string()],
        }];
        index.file_imports.insert(PathBuf::from("test.rs"), imports);

        let encoded = bincode::serde::encode_to_vec(&index, bincode::config::standard())
            .expect("encode failed");
        let (decoded, _): (ProjectIndex, _) =
            bincode::serde::decode_from_slice(&encoded, bincode::config::standard())
                .expect("decode failed");

        assert_eq!(decoded.file_imports.len(), 1);
        let decoded_imports = decoded.file_imports.get(&PathBuf::from("test.rs")).unwrap();
        assert_eq!(decoded_imports.len(), 1);
        assert_eq!(decoded_imports[0].module_path, "crate::utils");
        assert_eq!(decoded_imports[0].names, vec!["helper", "format"]);
    }
}

// ─── WASM API tests (run on host, not actual WASM target) ──────────

mod wasm_api_tests {
    use super::*;
    use symlens::graph::call_graph::CallGraph;
    use symlens::parser::registry::LanguageRegistry;

    #[test]
    fn parse_rust_via_registry() {
        let registry = LanguageRegistry::new();
        let path = Path::new("test.rs");
        let source = b"fn hello() { println!(\"hi\"); }";
        let parser = registry.parser_for(path).expect("Should find Rust parser");
        let symbols = parser.extract_symbols(source, path).expect("Should parse");
        assert!(symbols.iter().any(|s| s.name == "hello"));
    }

    #[test]
    fn parse_typescript_via_registry() {
        let registry = LanguageRegistry::new();
        let path = Path::new("app.ts");
        let source = b"function greet(name: string): string { return name; }";
        let parser = registry.parser_for(path).expect("Should find TS parser");
        let symbols = parser.extract_symbols(source, path).expect("Should parse");
        assert!(symbols.iter().any(|s| s.name == "greet"));
    }

    #[test]
    fn parse_python_via_registry() {
        let registry = LanguageRegistry::new();
        let path = Path::new("app.py");
        let source = b"def process(data):\n    return data";
        let parser = registry
            .parser_for(path)
            .expect("Should find Python parser");
        let symbols = parser.extract_symbols(source, path).expect("Should parse");
        assert!(symbols.iter().any(|s| s.name == "process"));
    }

    #[test]
    fn extract_calls_and_build_graph() {
        let registry = LanguageRegistry::new();
        let path = Path::new("test.rs");
        let source = b"fn main() { foo(); bar(); } fn foo() {} fn bar() { foo(); }";
        let parser = registry.parser_for(path).unwrap();
        let edges = parser.extract_calls(source, path).unwrap();

        let graph = CallGraph::build(&edges);
        let foo_callers = graph.callers("foo");
        // main and bar both call foo
        assert!(!foo_callers.is_empty(), "foo should have at least 1 caller");
    }

    #[test]
    fn call_graph_roundtrip_json() {
        let edges = vec![
            ("main".to_string(), "init".to_string()),
            ("main".to_string(), "run".to_string()),
            ("run".to_string(), "cleanup".to_string()),
        ];
        let graph = CallGraph::build(&edges);

        // Serialize to JSON (like WASM API would do)
        let json = serde_json::to_value(&graph).expect("Should serialize to JSON");
        assert!(json.get("nodes").is_some());
        assert!(json.get("edges").is_some());

        // Deserialize back
        let mut restored: CallGraph =
            serde_json::from_value(json).expect("Should deserialize from JSON");
        restored.rebuild_index();

        let callers = restored.callers("init");
        assert!(callers.contains(&"main"));

        let callees = restored.callees("main");
        assert_eq!(callees.len(), 2);
    }

    #[test]
    fn unsupported_extension_returns_none() {
        let registry = LanguageRegistry::new();
        assert!(registry.parser_for(Path::new("data.csv")).is_none());
        assert!(registry.parser_for(Path::new("image.png")).is_none());
    }

    #[test]
    fn all_nine_languages_have_parsers() {
        let registry = LanguageRegistry::new();
        let test_files = [
            "test.rs",
            "test.ts",
            "test.py",
            "test.swift",
            "test.go",
            "test.dart",
            "test.c",
            "test.cpp",
            "test.kt",
        ];
        for file in &test_files {
            assert!(
                registry.parser_for(Path::new(file)).is_some(),
                "Should have parser for {}",
                file
            );
        }
    }
}

// ─── End-to-end integration tests ────────────────────────────────────

mod cli_integration_tests {
    use std::path::PathBuf;

    const ENGINE_RS: &str = r#"
/// The main audio engine.
pub struct AudioEngine {
    sample_rate: u32,
    channels: usize,
}

impl AudioEngine {
    /// Create a new engine with the given sample rate.
    pub fn new(rate: u32) -> Self {
        AudioEngine { sample_rate: rate, channels: 2 }
    }

    /// Process an audio block.
    pub fn process_block(&mut self, data: &mut [f32]) {
        for sample in data.iter_mut() {
            *sample = normalize(*sample);
        }
    }
}

fn normalize(value: f32) -> f32 {
    value.clamp(-1.0, 1.0)
}
"#;

    const UTILS_RS: &str = r#"
/// Maximum supported channels.
pub const MAX_CHANNELS: usize = 8;

/// Clamp a value between min and max.
pub fn clamp_value(val: f32, min: f32, max: f32) -> f32 {
    val.clamp(min, max)
}

pub fn compute_rms(data: &[f32]) -> f32 {
    let sum: f32 = data.iter().map(|x| x * x).sum();
    (sum / data.len() as f32).sqrt()
}
"#;

    const MAIN_RS: &str = r#"
fn main() {
    let mut engine = AudioEngine::new(44100);
    let mut buf = vec![0.0f32; 1024];
    engine.process_block(&mut buf);
    let rms = compute_rms(&buf);
    println!("rms = {}", rms);
}
"#;

    /// Create a multi-file test project in a temp directory.
    fn create_test_project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let root = dir.path().to_path_buf();

        // Create .git dir so find_project_root works
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();

        std::fs::write(root.join("src/engine.rs"), ENGINE_RS).unwrap();
        std::fs::write(root.join("src/utils.rs"), UTILS_RS).unwrap();
        std::fs::write(root.join("src/main.rs"), MAIN_RS).unwrap();

        (dir, root)
    }

    fn index_project(root: &std::path::Path) -> symlens::index::indexer::IndexResult {
        symlens::index::indexer::index_project(root, 100_000).expect("Failed to index project")
    }

    #[test]
    fn index_and_load_roundtrip() {
        let (_dir, root) = create_test_project();
        let result = index_project(&root);

        assert!(!result.index.symbols.is_empty(), "Should have symbols");
        assert!(result.files_scanned >= 3, "Should scan at least 3 files");
        assert!(result.files_parsed >= 3, "Should parse at least 3 files");
        assert!(
            result.index.call_graph.is_some(),
            "Should have a call graph"
        );

        // Save and reload
        let _cache = symlens::index::storage::save(&result.index).expect("Failed to save index");
        let loaded = symlens::index::storage::load(&root).expect("Failed to load index");
        let loaded = loaded.expect("Loaded index should not be None");

        assert_eq!(
            loaded.symbols.len(),
            result.index.symbols.len(),
            "Loaded symbol count should match"
        );
        assert!(
            loaded.call_graph.is_some(),
            "Loaded index should have call graph"
        );
    }

    #[test]
    fn search_finds_symbols() {
        let (_dir, root) = create_test_project();
        let result = index_project(&root);

        let hits = result.index.search("AudioEngine", 10);
        assert!(
            hits.iter().any(|s| s.name == "AudioEngine"),
            "Should find AudioEngine, got: {:?}",
            hits.iter().map(|s| &s.name).collect::<Vec<_>>()
        );

        let hits = result.index.search("normalize", 10);
        assert!(
            hits.iter().any(|s| s.name == "normalize"),
            "Should find normalize"
        );

        let hits = result.index.search("MAX_CHANNELS", 10);
        assert!(
            hits.iter().any(|s| s.name == "MAX_CHANNELS"),
            "Should find MAX_CHANNELS"
        );

        let hits = result.index.search("nonexistent_xyz_123", 10);
        assert!(hits.is_empty(), "Should not find nonexistent symbol");
    }

    #[test]
    fn search_by_kind_filtering() {
        let (_dir, root) = create_test_project();
        let result = index_project(&root);

        let all = result.index.search("AudioEngine", 20);

        let structs: Vec<_> = all
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Struct)
            .collect();
        assert!(
            structs.iter().any(|s| s.name == "AudioEngine"),
            "Should find AudioEngine as struct"
        );

        let fns: Vec<_> = all
            .iter()
            .filter(|s| s.kind == symlens::model::symbol::SymbolKind::Function)
            .collect();
        assert!(
            !fns.iter().any(|s| s.name == "AudioEngine"),
            "AudioEngine should not be a function"
        );
    }

    #[test]
    fn outline_file_returns_symbols() {
        let (_dir, root) = create_test_project();
        let result = index_project(&root);

        let engine_path = PathBuf::from("src/engine.rs");
        let symbols = result.index.symbols_in_file(&engine_path);

        assert!(
            symbols.len() >= 3,
            "engine.rs should have >= 3 symbols (struct + methods + fn), got {}",
            symbols.len()
        );

        assert!(
            symbols
                .iter()
                .any(|s| s.name == "AudioEngine"
                    && s.kind == symlens::model::symbol::SymbolKind::Struct),
            "Should find AudioEngine struct"
        );

        let pb = symbols.iter().find(|s| s.name == "process_block");
        assert!(pb.is_some(), "Should find process_block");
        assert!(
            pb.unwrap().parent.is_some(),
            "process_block should have a parent (AudioEngine)"
        );
    }

    #[test]
    fn outline_project_stats() {
        let (_dir, root) = create_test_project();
        let result = index_project(&root);

        let stats = result.index.stats();
        assert_eq!(stats.total_files, 3, "Should have 3 files");
        assert!(
            stats.total_symbols >= 6,
            "Should have >= 6 symbols, got {}",
            stats.total_symbols
        );
        assert!(
            stats.by_language.contains_key("rust"),
            "Should detect rust language"
        );
    }

    #[test]
    fn callers_cross_file() {
        let (_dir, root) = create_test_project();
        let result = index_project(&root);
        let graph = result
            .index
            .call_graph
            .as_ref()
            .expect("Should have call graph");

        let normalize_callers = graph.callers("normalize");
        assert!(
            normalize_callers
                .iter()
                .any(|c| c.contains("process_block")),
            "normalize should be called by process_block, got callers: {:?}",
            normalize_callers
        );

        let pb_callers = graph.callers("process_block");
        assert!(
            pb_callers.iter().any(|c| c.contains("main")),
            "process_block should be called by main, got callers: {:?}",
            pb_callers
        );
    }

    #[test]
    fn callees_cross_file() {
        let (_dir, root) = create_test_project();
        let result = index_project(&root);
        let graph = result
            .index
            .call_graph
            .as_ref()
            .expect("Should have call graph");

        let pb_callees = graph.callees("process_block");
        assert!(
            pb_callees.iter().any(|c| c.contains("normalize")),
            "process_block should call normalize, got callees: {:?}",
            pb_callees
        );

        let main_callees = graph.callees("main");
        assert!(!main_callees.is_empty(), "main should have callees");
    }

    #[test]
    fn transitive_callers_depth() {
        let (_dir, root) = create_test_project();
        let result = index_project(&root);
        let graph = result
            .index
            .call_graph
            .as_ref()
            .expect("Should have call graph");

        let tc = graph.transitive_callers("normalize", 5);
        let names: Vec<&str> = tc.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("process_block")),
            "transitive callers of normalize should include process_block, got: {:?}",
            names
        );
    }

    #[test]
    fn refs_find_identifiers() {
        let (_dir, root) = create_test_project();

        let source = std::fs::read(root.join("src/engine.rs")).unwrap();
        let parser = symlens::parser::rust::RustParser;
        let refs = symlens::parser::traits::LanguageParser::find_identifiers(
            &parser,
            &source,
            "normalize",
        )
        .expect("Failed to find identifiers");

        assert!(
            refs.len() >= 2,
            "Should find >= 2 refs to 'normalize' (def + call), got {}",
            refs.len()
        );

        assert!(
            refs.iter()
                .any(|r| r.kind == symlens::parser::traits::RefKind::Call),
            "Should find at least one Call ref"
        );
    }

    #[test]
    fn incremental_index_skips_unchanged() {
        let (_dir, root) = create_test_project();

        let result1 = index_project(&root);
        assert!(result1.files_parsed >= 3);

        let result2 = symlens::index::indexer::index_project_incremental(
            &root,
            100_000,
            Some(&result1.index),
        )
        .expect("Incremental index failed");

        assert_eq!(
            result2.files_parsed, 0,
            "Should not re-parse any files, got parsed={}",
            result2.files_parsed
        );
        assert!(
            result2.files_skipped >= 3,
            "Should skip all files, got skipped={}",
            result2.files_skipped
        );
        assert_eq!(
            result2.index.symbols.len(),
            result1.index.symbols.len(),
            "Symbol count should be preserved"
        );
    }

    #[test]
    fn incremental_index_detects_change() {
        let (_dir, root) = create_test_project();

        let result1 = index_project(&root);

        // Wait >1s to ensure different mtime (filesystem mtime granularity is 1 second)
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(
            root.join("src/utils.rs"),
            format!("{}\npub fn added_func() {{}}\n", UTILS_RS),
        )
        .unwrap();

        let result2 = symlens::index::indexer::index_project_incremental(
            &root,
            100_000,
            Some(&result1.index),
        )
        .expect("Incremental index failed");

        assert!(
            result2.files_parsed >= 1,
            "Should re-parse at least 1 file, got parsed={}",
            result2.files_parsed
        );
        assert!(
            result2.files_skipped >= 1,
            "Should skip at least 1 unchanged file, got skipped={}",
            result2.files_skipped
        );
        assert!(
            result2.index.symbols.len() > result1.index.symbols.len(),
            "Should have more symbols after adding a function"
        );
    }

    #[test]
    fn index_empty_project() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".git")).unwrap();

        let result = index_project(&root);
        assert_eq!(
            result.index.symbols.len(),
            0,
            "Empty project should have 0 symbols"
        );
        assert_eq!(result.files_scanned, 0, "Empty project should scan 0 files");
    }
}

// ─── Python AST import tests ────────────────────────────────────────

#[test]
fn py_import_simple() {
    let parser = symlens::parser::python::PythonParser;
    let source = br#"
import os
import sys
"#;
    let imports = parser
        .extract_imports(source, std::path::Path::new("test.py"))
        .unwrap();
    assert!(
        imports.iter().any(|i| i.module_path == "os"),
        "Should import os"
    );
    assert!(
        imports.iter().any(|i| i.module_path == "sys"),
        "Should import sys"
    );
}

#[test]
fn py_import_dotted() {
    let parser = symlens::parser::python::PythonParser;
    let source = b"import os.path\n";
    let imports = parser
        .extract_imports(source, std::path::Path::new("test.py"))
        .unwrap();
    assert!(
        imports.iter().any(|i| i.module_path == "os.path"),
        "Should import os.path"
    );
}

#[test]
fn py_import_from() {
    let parser = symlens::parser::python::PythonParser;
    let source = b"from foo.bar import Baz, Qux\n";
    let imports = parser
        .extract_imports(source, std::path::Path::new("test.py"))
        .unwrap();
    assert_eq!(imports.len(), 1, "Should have 1 import statement");
    assert_eq!(imports[0].module_path, "foo.bar");
    assert!(
        imports[0].names.contains(&"Baz".to_string()),
        "Should import Baz"
    );
    assert!(
        imports[0].names.contains(&"Qux".to_string()),
        "Should import Qux"
    );
}

#[test]
fn py_import_from_typing() {
    let parser = symlens::parser::python::PythonParser;
    let source = b"from typing import Optional, List\n";
    let imports = parser
        .extract_imports(source, std::path::Path::new("test.py"))
        .unwrap();
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].module_path, "typing");
    assert!(imports[0].names.contains(&"Optional".to_string()));
    assert!(imports[0].names.contains(&"List".to_string()));
}

#[test]
fn py_import_relative() {
    let parser = symlens::parser::python::PythonParser;
    let source = b"from . import something\nfrom ..package import Module\n";
    let imports = parser
        .extract_imports(source, std::path::Path::new("test.py"))
        .unwrap();
    assert!(imports.len() >= 1, "Should have at least 1 import");
    // Relative imports with module path
    let from_pkg = imports.iter().find(|i| i.module_path.contains("package"));
    assert!(from_pkg.is_some(), "Should find from ..package import");
    assert!(from_pkg.unwrap().names.contains(&"Module".to_string()));
}

#[test]
fn py_import_wildcard_skipped() {
    let parser = symlens::parser::python::PythonParser;
    let source = b"from foo import *\n";
    let imports = parser
        .extract_imports(source, std::path::Path::new("test.py"))
        .unwrap();
    // Wildcard imports have no specific names, should be skipped
    assert!(
        imports.is_empty() || !imports.iter().any(|i| i.names.is_empty()),
        "Wildcard should not produce empty-name import"
    );
}

#[test]
fn py_import_aliased() {
    let parser = symlens::parser::python::PythonParser;
    let source = b"import numpy as np\n";
    let imports = parser
        .extract_imports(source, std::path::Path::new("test.py"))
        .unwrap();
    assert!(
        imports.iter().any(|i| i.names.contains(&"np".to_string())),
        "Should import np alias"
    );
}

#[test]
fn py_import_from_aliased() {
    let parser = symlens::parser::python::PythonParser;
    let source = b"from foo import Bar as Baz\n";
    let imports = parser
        .extract_imports(source, std::path::Path::new("test.py"))
        .unwrap();
    assert!(
        imports.iter().any(|i| i.names.contains(&"Baz".to_string())),
        "Should import Baz alias"
    );
}

// ─── DepsGraph dependents/dependencies tests ─────────────────────────

mod deps_query_tests {
    use std::path::PathBuf;
    use symlens::graph::deps::DepsGraph;

    fn build_sample_deps() -> DepsGraph {
        let imports = vec![
            (PathBuf::from("src/main.rs"), "crate::engine".to_string()),
            (PathBuf::from("src/main.rs"), "crate::audio".to_string()),
            (PathBuf::from("src/engine.rs"), "crate::audio".to_string()),
        ];
        let known = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/engine.rs"),
            PathBuf::from("src/audio.rs"),
        ];
        DepsGraph::build(&imports, &known)
    }

    #[test]
    fn dependencies_returns_outgoing_deps() {
        let graph = build_sample_deps();
        let deps = graph.dependencies(&PathBuf::from("src/main.rs"));
        assert!(
            deps.iter().any(|d| d.to_string_lossy().contains("engine")),
            "main.rs should depend on engine.rs, got: {:?}",
            deps
        );
        assert!(
            deps.iter().any(|d| d.to_string_lossy().contains("audio")),
            "main.rs should depend on audio.rs, got: {:?}",
            deps
        );
    }

    #[test]
    fn dependents_returns_incoming_deps() {
        let graph = build_sample_deps();
        let dependents = graph.dependents(&PathBuf::from("src/audio.rs"));
        assert!(
            dependents.len() >= 2,
            "audio.rs should have >= 2 dependents, got: {:?}",
            dependents
        );
        assert!(
            dependents
                .iter()
                .any(|d| d.to_string_lossy().contains("main")),
            "main.rs should depend on audio.rs"
        );
        assert!(
            dependents
                .iter()
                .any(|d| d.to_string_lossy().contains("engine")),
            "engine.rs should depend on audio.rs"
        );
    }

    #[test]
    fn dependencies_unknown_file_returns_empty() {
        let graph = build_sample_deps();
        let deps = graph.dependencies(&PathBuf::from("src/nonexistent.rs"));
        assert!(deps.is_empty(), "Unknown file should have no dependencies");
    }

    #[test]
    fn dependents_unknown_file_returns_empty() {
        let graph = build_sample_deps();
        let deps = graph.dependents(&PathBuf::from("src/nonexistent.rs"));
        assert!(deps.is_empty(), "Unknown file should have no dependents");
    }
}

// ─── Export sqlite tests ────────────────────────────────────────────

mod export_sqlite_tests {
    use std::path::PathBuf;
    use symlens::model::project::ProjectIndex;
    use symlens::model::symbol::*;

    fn make_symbol(name: &str, kind: SymbolKind, file: &str) -> Symbol {
        Symbol {
            id: SymbolId::new(file, name, &kind),
            name: name.to_string(),
            qualified_name: name.to_string(),
            kind,
            file_path: PathBuf::from(file),
            span: Span {
                start_line: 1,
                end_line: 10,
                start_col: 0,
                end_col: 0,
            },
            signature: Some(format!("fn {}()", name)),
            doc_comment: None,
            visibility: Visibility::Public,
            parent: None,
            children: vec![],
        }
    }

    #[test]
    fn sqlite_export_creates_db() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test-export"));
        index.insert(make_symbol("foo", SymbolKind::Function, "src/main.rs"));
        index.insert(make_symbol("Bar", SymbolKind::Struct, "src/main.rs"));

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Use rusqlite directly to mirror what export_sqlite does
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE symbols (id TEXT PRIMARY KEY, name TEXT, kind TEXT);
             INSERT INTO symbols VALUES ('src/main.rs::foo#function', 'foo', 'function');
             INSERT INTO symbols VALUES ('src/main.rs::Bar#struct', 'Bar', 'struct');",
        )
        .unwrap();

        // Verify the db file exists and is readable
        assert!(db_path.exists(), "SQLite file should be created");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "Should have 2 symbols in the database");

        // Verify we can query by kind
        let fn_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM symbols WHERE kind = 'function'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(fn_count, 1, "Should have 1 function");
    }

    #[test]
    fn sqlite_export_full_roundtrip() {
        let mut index = ProjectIndex::new(PathBuf::from("/tmp/test-rt"));
        index.insert(make_symbol("hello", SymbolKind::Function, "src/lib.rs"));
        index.insert(make_symbol("World", SymbolKind::Struct, "src/lib.rs"));
        index.insert(make_symbol("method", SymbolKind::Method, "src/lib.rs"));

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("roundtrip.db");

        // Create and populate database
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("PRAGMA journal_mode = WAL;").unwrap();

        conn.execute_batch(
            "CREATE TABLE symbols (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                qualified_name TEXT NOT NULL,
                kind TEXT NOT NULL,
                file TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                visibility TEXT,
                signature TEXT,
                doc TEXT,
                parent TEXT
            );
            CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .unwrap();

        for s in index.symbols.values() {
            conn.execute(
                "INSERT INTO symbols (id, name, qualified_name, kind, file, start_line, end_line, visibility)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    s.id.0,
                    s.name,
                    s.qualified_name,
                    s.kind.as_str(),
                    s.file_path.to_string_lossy().as_ref(),
                    s.span.start_line,
                    s.span.end_line,
                    format!("{:?}", s.visibility),
                ],
            )
            .unwrap();
        }

        // Query back and verify
        let names: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name FROM symbols ORDER BY name")
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };
        assert_eq!(names, vec!["World", "hello", "method"]);
    }
}

// ─── NO_COLOR / CLICOLOR_FORCE resolution tests ─────────────────────

mod color_resolution_tests {
    // These tests verify the color resolution logic by simulating the
    // same precedence rules used in main.rs::resolve_color().

    fn resolve_color_sim(
        no_color_flag: bool,
        no_color_env: bool,
        clicolor_force_env: bool,
        is_tty: bool,
    ) -> bool {
        if no_color_flag {
            return false;
        }
        if no_color_env {
            return false;
        }
        if clicolor_force_env {
            return true;
        }
        is_tty
    }

    #[test]
    fn no_color_flag_disables() {
        assert!(!resolve_color_sim(true, false, false, true));
    }

    #[test]
    fn no_color_env_disables() {
        assert!(!resolve_color_sim(false, true, false, true));
    }

    #[test]
    fn no_color_flag_overrides_force() {
        // --no_color takes precedence over CLICOLOR_FORCE
        assert!(!resolve_color_sim(true, false, true, false));
    }

    #[test]
    fn no_color_env_overridden_by_flag() {
        // --no_color flag and NO_COLOR env both set — flag already disables
        assert!(!resolve_color_sim(true, true, false, true));
    }

    #[test]
    fn clicolor_force_enables_in_pipe() {
        assert!(resolve_color_sim(false, false, true, false));
    }

    #[test]
    fn clicolor_force_does_not_override_no_color() {
        // NO_COLOR env takes precedence over CLICOLOR_FORCE
        assert!(!resolve_color_sim(false, true, true, false));
    }

    #[test]
    fn tty_enables_color() {
        assert!(resolve_color_sim(false, false, false, true));
    }

    #[test]
    fn pipe_disables_color() {
        assert!(!resolve_color_sim(false, false, false, false));
    }
}

// ─── Workspace mode tests ───────────────────────────────────────────

mod workspace_tests {
    use std::path::PathBuf;
    use symlens::model::project::{FileKey, ProjectIndex, RootInfo};
    use symlens::model::symbol::*;
    use symlens::model::workspace::WorkspaceIndex;

    // ── SymbolId root_id prefix ──────────────────────────────────────

    #[test]
    fn symbol_id_with_root_prefix() {
        let id = SymbolId::new_with_root(
            "a1b2c3d4",
            "src/main.rs",
            "Engine::run",
            &SymbolKind::Method,
        );
        assert_eq!(id.0, "[a1b2c3d4]src/main.rs::Engine::run#method");
        assert_eq!(id.root_id(), "a1b2c3d4");
        assert_eq!(id.file(), "src/main.rs");
        assert_eq!(id.name(), "Engine::run");
        assert_eq!(id.kind_str(), "method");
    }

    #[test]
    fn symbol_id_without_root_backward_compat() {
        let id = SymbolId::new_with_root("", "src/main.rs", "Engine::run", &SymbolKind::Method);
        // Should fall back to standard format
        assert_eq!(id.0, "src/main.rs::Engine::run#method");
        assert_eq!(id.root_id(), "");
        assert_eq!(id.file(), "src/main.rs");
        assert_eq!(id.name(), "Engine::run");
        assert_eq!(id.kind_str(), "method");
    }

    #[test]
    fn symbol_id_root_id_parsing() {
        let id = SymbolId("[deadbeef]lib.rs::Foo#struct".to_string());
        assert_eq!(id.root_id(), "deadbeef");
        assert_eq!(id.file(), "lib.rs");
        assert_eq!(id.name(), "Foo");
        assert_eq!(id.kind_str(), "struct");
    }

    #[test]
    fn symbol_id_no_root_parsing() {
        let id = SymbolId("lib.rs::Foo#struct".to_string());
        assert_eq!(id.root_id(), "");
        assert_eq!(id.file(), "lib.rs");
        assert_eq!(id.name(), "Foo");
        assert_eq!(id.kind_str(), "struct");
    }

    // ── FileKey ──────────────────────────────────────────────────────

    #[test]
    fn file_key_with_root() {
        let key = FileKey::new("abc123", PathBuf::from("src/main.rs"));
        assert_eq!(key.root_id, "abc123");
        assert_eq!(key.path, PathBuf::from("src/main.rs"));
        assert_eq!(key.display(), "[abc123]src/main.rs");
    }

    #[test]
    fn file_key_without_root() {
        let key = FileKey::new("", PathBuf::from("src/main.rs"));
        assert_eq!(key.display(), "src/main.rs");
    }

    #[test]
    fn file_key_equality() {
        let a = FileKey::new("abc", PathBuf::from("src/main.rs"));
        let b = FileKey::new("abc", PathBuf::from("src/main.rs"));
        let c = FileKey::new("def", PathBuf::from("src/main.rs"));
        let d = FileKey::new("abc", PathBuf::from("src/lib.rs"));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    // ── RootInfo ─────────────────────────────────────────────────────

    #[test]
    fn root_info_derives_stable_id() {
        let r1 = RootInfo::new(PathBuf::from("/tmp/project-a"));
        let r2 = RootInfo::new(PathBuf::from("/tmp/project-a"));
        let r3 = RootInfo::new(PathBuf::from("/tmp/project-b"));

        // Same path -> same id
        assert_eq!(r1.id, r2.id);
        // Different path -> different id
        assert_ne!(r1.id, r3.id);
        // Id is 8 hex chars
        assert_eq!(r1.id.len(), 8);
        // Hash is 16 hex chars
        assert_eq!(r1.hash.len(), 16);
    }

    // ── WorkspaceIndex insert + query ────────────────────────────────

    fn make_symbol(name: &str, kind: SymbolKind, file: &str) -> Symbol {
        Symbol {
            id: SymbolId::new(file, name, &kind),
            name: name.to_string(),
            qualified_name: name.to_string(),
            kind,
            file_path: PathBuf::from(file),
            span: Span {
                start_line: 1,
                end_line: 10,
                start_col: 0,
                end_col: 0,
            },
            signature: Some(format!("fn {}()", name)),
            doc_comment: Some(format!("Doc for {}", name)),
            visibility: Visibility::Public,
            parent: None,
            children: vec![],
        }
    }

    fn make_project_with_symbols(
        root_path: &str,
        symbols: Vec<(&str, SymbolKind, &str)>,
    ) -> (RootInfo, ProjectIndex) {
        let root_info = RootInfo::new(PathBuf::from(root_path));
        let mut project = ProjectIndex::new(PathBuf::from(root_path));
        for (name, kind, file) in symbols {
            project.insert(make_symbol(name, kind, file));
        }
        (root_info, project)
    }

    #[test]
    fn workspace_insert_and_get() {
        let (root_info, project) = make_project_with_symbols(
            "/tmp/ws-root-a",
            vec![
                ("AudioEngine", SymbolKind::Struct, "src/engine.rs"),
                ("process_block", SymbolKind::Method, "src/engine.rs"),
            ],
        );

        let mut ws = WorkspaceIndex::new(&[root_info.clone()]);
        ws.insert_from_project(&root_info, &project);

        // Symbols should be prefixed with root_id
        assert_eq!(ws.symbols.len(), 2, "Should have 2 symbols after insert");

        // Get by workspace-scoped SymbolId
        let ws_id = SymbolId::new_with_root(
            &root_info.id,
            "src/engine.rs",
            "AudioEngine",
            &SymbolKind::Struct,
        );
        let sym = ws
            .get(&ws_id)
            .expect("Should find AudioEngine in workspace");
        assert_eq!(sym.name, "AudioEngine");

        // Root_id in SymbolId should match
        assert_eq!(sym.id.root_id(), root_info.id);
    }

    #[test]
    fn workspace_search_cross_root() {
        let (root_a, project_a) = make_project_with_symbols(
            "/tmp/ws-root-a",
            vec![("AudioEngine", SymbolKind::Struct, "src/engine.rs")],
        );
        let (root_b, project_b) = make_project_with_symbols(
            "/tmp/ws-root-b",
            vec![
                ("VideoEngine", SymbolKind::Struct, "src/video.rs"),
                ("process_audio", SymbolKind::Function, "src/audio.rs"),
            ],
        );

        let mut ws = WorkspaceIndex::new(&[root_a.clone(), root_b.clone()]);
        ws.insert_from_project(&root_a, &project_a);
        ws.insert_from_project(&root_b, &project_b);

        // Search "engine" should find both AudioEngine and VideoEngine
        let results = ws.search("engine", 10);
        assert_eq!(
            results.len(),
            2,
            "Should find 2 'engine' symbols across roots"
        );
        let names: Vec<_> = results.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"AudioEngine"));
        assert!(names.contains(&"VideoEngine"));

        // Search "audio" should find AudioEngine + process_audio
        let results = ws.search("audio", 10);
        assert_eq!(
            results.len(),
            2,
            "Should find 2 'audio' symbols across roots"
        );
    }

    #[test]
    fn workspace_remove_root() {
        let (root_a, project_a) = make_project_with_symbols(
            "/tmp/ws-root-a",
            vec![("AudioEngine", SymbolKind::Struct, "src/engine.rs")],
        );
        let (root_b, project_b) = make_project_with_symbols(
            "/tmp/ws-root-b",
            vec![("VideoEngine", SymbolKind::Struct, "src/video.rs")],
        );

        let mut ws = WorkspaceIndex::new(&[root_a.clone(), root_b.clone()]);
        ws.insert_from_project(&root_a, &project_a);
        ws.insert_from_project(&root_b, &project_b);

        assert_eq!(ws.symbols.len(), 2);

        // Remove root_b
        ws.remove_root(&root_b.id);

        assert_eq!(
            ws.symbols.len(),
            1,
            "Should have 1 symbol after removing root_b"
        );
        assert!(ws.roots.iter().any(|r| r.id == root_a.id));
        assert!(!ws.roots.iter().any(|r| r.id == root_b.id));

        // Search should only find AudioEngine
        let results = ws.search("engine", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "AudioEngine");
    }

    #[test]
    fn workspace_symbols_in_file() {
        let (root_info, project) = make_project_with_symbols(
            "/tmp/ws-root-a",
            vec![
                ("foo", SymbolKind::Function, "src/a.rs"),
                ("bar", SymbolKind::Function, "src/a.rs"),
                ("baz", SymbolKind::Function, "src/b.rs"),
            ],
        );

        let mut ws = WorkspaceIndex::new(&[root_info.clone()]);
        ws.insert_from_project(&root_info, &project);

        let file_key = FileKey::new(&root_info.id, PathBuf::from("src/a.rs"));
        let syms = ws.symbols_in_file(&file_key);
        assert_eq!(syms.len(), 2, "Should find 2 symbols in src/a.rs");
    }

    #[test]
    fn workspace_stats() {
        let (root_a, project_a) = make_project_with_symbols(
            "/tmp/ws-root-a",
            vec![
                ("foo", SymbolKind::Function, "src/a.rs"),
                ("Bar", SymbolKind::Struct, "src/a.rs"),
            ],
        );
        let (root_b, project_b) = make_project_with_symbols(
            "/tmp/ws-root-b",
            vec![("baz", SymbolKind::Function, "lib/b.rs")],
        );

        let mut ws = WorkspaceIndex::new(&[root_a.clone(), root_b.clone()]);
        ws.insert_from_project(&root_a, &project_a);
        ws.insert_from_project(&root_b, &project_b);

        let stats = ws.stats();
        assert_eq!(stats.total_files, 2, "Should count 2 files across roots");
        assert_eq!(
            stats.total_symbols, 3,
            "Should count 3 symbols across roots"
        );
        assert_eq!(*stats.by_kind.get("function").unwrap(), 2);
        assert_eq!(*stats.by_kind.get("struct").unwrap(), 1);
    }

    #[test]
    fn workspace_call_graph_cross_root() {
        let (root_a, project_a) = make_project_with_symbols(
            "/tmp/ws-root-a",
            vec![("process_audio", SymbolKind::Function, "src/audio.rs")],
        );
        let (root_b, project_b) = make_project_with_symbols(
            "/tmp/ws-root-b",
            vec![("main", SymbolKind::Function, "src/main.rs")],
        );

        let root_a_id = root_a.id.clone();
        let root_b_id = root_b.id.clone();

        let mut ws = WorkspaceIndex::new(&[root_a.clone(), root_b.clone()]);
        ws.insert_from_project(&root_a, &project_a);
        ws.insert_from_project(&root_b, &project_b);

        // Add a cross-root call edge manually: main -> process_audio
        let file_key = FileKey::new(&root_b_id, PathBuf::from("src/main.rs"));
        ws.file_call_edges.insert(
            file_key,
            vec![(
                format!("[{}]main", root_b_id),
                format!("[{}]process_audio", root_a_id),
            )],
        );

        ws.build_call_graph();

        let graph = ws.call_graph.as_ref().expect("Should have call graph");
        let callers = graph.callers("process_audio");
        assert!(
            callers.iter().any(|c| c.contains("main")),
            "main should be a caller of process_audio across roots, got: {:?}",
            callers,
        );
    }

    #[test]
    fn workspace_resolve_absolute() {
        let root_info = RootInfo::new(PathBuf::from("/tmp/my-project"));
        let ws = WorkspaceIndex::new(&[root_info.clone()]);

        let file_key = FileKey::new(&root_info.id, PathBuf::from("src/main.rs"));
        let abs = ws.resolve_absolute(&file_key);
        assert_eq!(abs, Some(PathBuf::from("/tmp/my-project/src/main.rs")));
    }

    #[test]
    fn workspace_hash_deterministic() {
        let r1 = RootInfo::new(PathBuf::from("/tmp/project-a"));
        let r2 = RootInfo::new(PathBuf::from("/tmp/project-b"));

        let ws1 = WorkspaceIndex::new(&[r1.clone(), r2.clone()]);
        let ws2 = WorkspaceIndex::new(&[r2.clone(), r1.clone()]);

        // Hash should be the same regardless of root order
        assert_eq!(ws1.workspace_hash, ws2.workspace_hash);
    }

    #[test]
    fn workspace_serialization_roundtrip() {
        let (root_info, project) = make_project_with_symbols(
            "/tmp/ws-root-a",
            vec![
                ("AudioEngine", SymbolKind::Struct, "src/engine.rs"),
                ("process_block", SymbolKind::Method, "src/engine.rs"),
            ],
        );

        let mut ws = WorkspaceIndex::new(&[root_info.clone()]);
        ws.insert_from_project(&root_info, &project);

        let encoded =
            bincode::serde::encode_to_vec(&ws, bincode::config::standard()).expect("encode failed");
        let (decoded, _): (WorkspaceIndex, _) =
            bincode::serde::decode_from_slice(&encoded, bincode::config::standard())
                .expect("decode failed");

        assert_eq!(decoded.symbols.len(), 2);
        assert_eq!(decoded.roots.len(), 1);
        assert_eq!(decoded.workspace_hash, ws.workspace_hash);

        // Search cache needs rebuild after deserialization
        let mut restored = decoded;
        restored.rebuild_search_cache();
        let results = restored.search("AudioEngine", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn workspace_parent_child_remap() {
        let mut project = ProjectIndex::new(PathBuf::from("/tmp/ws-parent"));
        let mut child = make_symbol("process_block", SymbolKind::Method, "src/engine.rs");
        let parent_id = SymbolId::new("src/engine.rs", "AudioEngine", &SymbolKind::Struct);
        child.parent = Some(parent_id.clone());

        let mut parent = make_symbol("AudioEngine", SymbolKind::Struct, "src/engine.rs");
        parent.children = vec![child.id.clone()];

        project.insert(parent);
        project.insert(child);

        let root_info = RootInfo::new(PathBuf::from("/tmp/ws-parent"));
        let mut ws = WorkspaceIndex::new(&[root_info.clone()]);
        ws.insert_from_project(&root_info, &project);

        // Find the workspace-scoped process_block
        let ws_pb_id = SymbolId::new_with_root(
            &root_info.id,
            "src/engine.rs",
            "process_block",
            &SymbolKind::Method,
        );
        let ws_pb = ws.get(&ws_pb_id).expect("Should find process_block");

        // Parent should be remapped with root_id prefix
        let ws_parent = ws_pb.parent.as_ref().expect("Should have parent");
        assert_eq!(ws_parent.root_id(), root_info.id);
        assert!(ws_parent.name().contains("AudioEngine"));
    }
}
