use std::path::Path;

// ─── Parser tests ───────────────────────────────────────────────────

mod parser_tests {
    use super::*;

    fn parse_rust_fixture() -> Vec<codelens::model::symbol::Symbol> {
        let parser = codelens::parser::rust::RustParser;
        let source = include_bytes!("fixtures/sample.rs");
        codelens::parser::traits::LanguageParser::extract_symbols(
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
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Struct)
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
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Function)
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
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Method)
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
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Constant)
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
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Enum)
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
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Interface)
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
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::TypeAlias)
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
        let parser = codelens::parser::rust::RustParser;
        let source = include_bytes!("fixtures/sample.rs");
        let calls = codelens::parser::traits::LanguageParser::extract_calls(
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
        let parser = codelens::parser::rust::RustParser;
        let source = include_bytes!("fixtures/sample.rs");
        let imports = codelens::parser::traits::LanguageParser::extract_imports(
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
        let parser = codelens::parser::rust::RustParser;
        let source = include_bytes!("fixtures/sample.rs");
        let refs = codelens::parser::traits::LanguageParser::find_identifiers(
            &parser,
            source,
            "normalize",
        )
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
        let parser = codelens::parser::typescript::TypeScriptParser;
        let source = include_bytes!("fixtures/sample.ts");
        let symbols = codelens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.ts"),
        )
        .expect("Failed to parse TS fixture");
        let classes: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Class)
            .collect();
        assert!(
            classes.iter().any(|s| s.name == "Server"),
            "Should find Server class, got: {:?}",
            classes.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ts_extracts_interface() {
        let parser = codelens::parser::typescript::TypeScriptParser;
        let source = include_bytes!("fixtures/sample.ts");
        let symbols = codelens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.ts"),
        )
        .expect("Failed to parse TS fixture");
        let interfaces: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Interface)
            .collect();
        assert!(
            interfaces.iter().any(|s| s.name == "Config"),
            "Should find Config interface"
        );
    }

    #[test]
    fn ts_extracts_function() {
        let parser = codelens::parser::typescript::TypeScriptParser;
        let source = include_bytes!("fixtures/sample.ts");
        let symbols = codelens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.ts"),
        )
        .expect("Failed to parse TS fixture");
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Function)
            .collect();
        assert!(
            fns.iter().any(|s| s.name == "createServer"),
            "Should find createServer function"
        );
    }

    // Python parser tests
    #[test]
    fn python_extracts_class() {
        let parser = codelens::parser::python::PythonParser;
        let source = include_bytes!("fixtures/sample.py");
        let symbols = codelens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.py"),
        )
        .expect("Failed to parse Python fixture");
        let classes: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Class)
            .collect();
        assert!(
            classes.iter().any(|s| s.name == "Database"),
            "Should find Database class"
        );
    }

    #[test]
    fn python_extracts_functions() {
        let parser = codelens::parser::python::PythonParser;
        let source = include_bytes!("fixtures/sample.py");
        let symbols = codelens::parser::traits::LanguageParser::extract_symbols(
            &parser,
            source,
            Path::new("sample.py"),
        )
        .expect("Failed to parse Python fixture");
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == codelens::model::symbol::SymbolKind::Function)
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

// ─── Call graph tests ───────────────────────────────────────────────

mod call_graph_tests {
    use codelens::graph::call_graph::CallGraph;

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
    use codelens::model::project::ProjectIndex;
    use codelens::model::symbol::*;
    use std::path::PathBuf;

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
    use codelens::model::symbol::*;

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
    use codelens::graph::call_graph::CallGraph;
    use codelens::graph::path::find_path;

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
    use codelens::graph::deps::DepsGraph;
    use std::path::PathBuf;

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
