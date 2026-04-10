use crate::model::symbol::Symbol;
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

/// Each language implements this trait to extract symbols from source code.
pub trait LanguageParser: Send + Sync {
    /// File extensions this parser handles (e.g. ["rs"])
    fn extensions(&self) -> &[&str];

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
}

/// Import information extracted from source.
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// Module path (e.g. "crate::audio::engine", "audio.engine")
    #[allow(dead_code)]
    pub module_path: String,
    /// Imported names (e.g. ["AudioEngine", "AudioError"])
    pub names: Vec<String>,
}
