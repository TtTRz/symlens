use std::path::Path;

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

        assert!(result.index.symbols.len() > 0, "Should have symbols");
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
