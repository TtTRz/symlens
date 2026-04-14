use crate::model::symbol::Symbol;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A call edge: (caller_qualified_name, callee_name)
pub type CallEdge = (String, String);

/// An identifier reference found in source code.
#[derive(Debug)]
pub struct IdentifierRef {
    pub line: u32,
    pub context: String,
    pub kind: RefKind,
}

/// Classification of a reference based on AST context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    /// Function/method call: foo(), self.foo(), Struct::foo()
    Call,
    /// Type annotation: x: Foo, -> Foo, Vec<Foo>
    TypeRef,
    /// Import/use statement: use crate::Foo
    Import,
    /// Field access: self.foo (without call)
    FieldAccess,
    /// Constructor: Foo::new(), Foo { ... }
    Constructor,
    /// Definition site itself
    Definition,
    /// Could not classify
    Unknown,
}

/// All extractable data from a single parse pass.
#[derive(Default)]
pub struct ParsedOutput {
    pub symbols: Vec<Symbol>,
    pub call_edges: Vec<CallEdge>,
    pub imports: Vec<ImportInfo>,
}

/// Each language implements this trait to extract symbols from source code.
pub trait LanguageParser: Send + Sync {
    /// File extensions this parser handles (e.g. ["rs"])
    fn extensions(&self) -> &[&str];

    /// Return the tree-sitter Language for this parser.
    /// Used by `extract_all()` to parse once and reuse the tree.
    fn language(&self) -> tree_sitter::Language;

    /// Parse source code and extract symbols.
    fn extract_symbols(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<Symbol>>;

    /// Extract call edges from source (caller_name, callee_name).
    fn extract_calls(&self, source: &[u8], file_path: &Path) -> anyhow::Result<Vec<CallEdge>> {
        let _ = (source, file_path);
        Ok(vec![])
    }

    /// Find all identifier references matching a name.
    fn find_identifiers(
        &self,
        source: &[u8],
        target_name: &str,
    ) -> anyhow::Result<Vec<IdentifierRef>> {
        let _ = (source, target_name);
        Ok(vec![])
    }

    /// Extract import/use statements.
    fn extract_imports(&self, source: &[u8], _file_path: &Path) -> anyhow::Result<Vec<ImportInfo>> {
        let _ = source;
        Ok(vec![])
    }

    /// Extract symbols from a pre-parsed tree.
    /// Override this to benefit from single-parse optimization via `extract_all()`.
    fn extract_symbols_from_tree(
        &self,
        _tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> anyhow::Result<Vec<Symbol>> {
        // Default: fall back to re-parsing
        self.extract_symbols(source, file_path)
    }

    /// Extract call edges from a pre-parsed tree.
    fn extract_calls_from_tree(
        &self,
        _tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> anyhow::Result<Vec<CallEdge>> {
        let _ = source;
        self.extract_calls(source, file_path)
    }

    /// Extract imports from a pre-parsed tree.
    fn extract_imports_from_tree(
        &self,
        _tree: &tree_sitter::Tree,
        source: &[u8],
        file_path: &Path,
    ) -> anyhow::Result<Vec<ImportInfo>> {
        let _ = source;
        self.extract_imports(source, file_path)
    }

    /// Parse once and extract all data (symbols, calls, imports).
    /// Uses `language()` to create the tree, then calls `_from_tree` variants.
    fn extract_all(&self, source: &[u8], file_path: &Path) -> anyhow::Result<ParsedOutput> {
        let tree = crate::parser::helpers::parse_source(self.language(), source, file_path)?;

        let symbols = self.extract_symbols_from_tree(&tree, source, file_path)?;
        let call_edges = self.extract_calls_from_tree(&tree, source, file_path)?;
        let imports = self.extract_imports_from_tree(&tree, source, file_path)?;

        Ok(ParsedOutput {
            symbols,
            call_edges,
            imports,
        })
    }
}

/// Import information extracted from source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    /// Module path (e.g. "crate::audio::engine", "audio.engine")
    #[allow(dead_code)]
    pub module_path: String,
    /// Imported names (e.g. ["AudioEngine", "AudioError"])
    pub names: Vec<String>,
}
