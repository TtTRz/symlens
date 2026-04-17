use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Stable unique identifier for a symbol.
/// Format: "relative/path.rs::QualifiedName#kind"
/// Example: "src/audio/engine.rs::AudioEngine::process_block#method"
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SymbolId(pub String);

impl SymbolId {
    pub fn new(file_path: &str, qualified_name: &str, kind: &SymbolKind) -> Self {
        Self(format!(
            "{}::{}#{}",
            file_path,
            qualified_name,
            kind.as_str()
        ))
    }

    /// Create a workspace-aware SymbolId with an optional root_id prefix.
    /// When root_id is empty, falls back to the standard format (backward compatible).
    /// Workspace format: "[a1b2c3d4]path::Name#kind"
    /// Single-root format: "path::Name#kind"
    pub fn new_with_root(
        root_id: &str,
        file_path: &str,
        qualified_name: &str,
        kind: &SymbolKind,
    ) -> Self {
        if root_id.is_empty() {
            Self::new(file_path, qualified_name, kind)
        } else {
            Self(format!(
                "[{}]{}::{}#{}",
                root_id,
                file_path,
                qualified_name,
                kind.as_str()
            ))
        }
    }

    /// Returns the root_id portion ("a1b2c3d4") if a "[root_id]" prefix is present,
    /// otherwise returns "" (single-root mode).
    pub fn root_id(&self) -> &str {
        if self.0.starts_with('[') {
            self.0.find(']').map(|i| &self.0[1..i]).unwrap_or("")
        } else {
            ""
        }
    }

    /// Returns the file path portion (everything before the first "::" delimiter).
    /// Handles both "[root_id]path::Name#kind" and "path::Name#kind" formats.
    pub fn file(&self) -> &str {
        let start = self.content_start();
        let slice = &self.0[start..];
        slice.find("::").map(|i| &slice[..i]).unwrap_or(slice)
    }

    /// Returns the qualified name portion (between "::" and "#").
    /// Handles both "[root_id]path::Name#kind" and "path::Name#kind" formats.
    pub fn name(&self) -> &str {
        let start = self.content_start();
        let slice = &self.0[start..];
        let name_start = slice.find("::").map(|i| i + 2).unwrap_or(0);
        let name_end = slice.rfind('#').unwrap_or(slice.len());
        &slice[name_start..name_end]
    }

    /// Returns the kind portion (everything after "#").
    /// Handles both "[root_id]path::Name#kind" and "path::Name#kind" formats.
    pub fn kind_str(&self) -> &str {
        let start = self.content_start();
        let slice = &self.0[start..];
        slice.rfind('#').map(|i| &slice[i + 1..]).unwrap_or("")
    }

    /// Returns the byte offset where the actual content starts,
    /// skipping the "[root_id]" prefix if present.
    fn content_start(&self) -> usize {
        if self.0.starts_with('[') {
            self.0.find(']').map(|i| i + 1).unwrap_or(0)
        } else {
            0
        }
    }
}

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A code symbol — the minimal indexing unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    /// Simple name (e.g. "process_block")
    pub name: String,
    /// Qualified name including parent (e.g. "AudioEngine::process_block")
    pub qualified_name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Relative file path from project root
    pub file_path: PathBuf,
    /// Source location
    pub span: Span,
    /// Function/method signature (the declaration line)
    pub signature: Option<String>,
    /// Doc comment text
    pub doc_comment: Option<String>,
    /// Visibility
    pub visibility: Visibility,
    /// Parent symbol ID (method → struct, field → struct)
    pub parent: Option<SymbolId>,
    /// Child symbol IDs (struct → fields/methods)
    pub children: Vec<SymbolId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Class,
    Enum,
    EnumVariant,
    Interface, // Trait / Protocol / Interface
    Field,
    Constant,
    Variable,
    Module,
    TypeAlias,
    Macro,
    Import,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Class => "class",
            Self::Enum => "enum",
            Self::EnumVariant => "variant",
            Self::Interface => "interface",
            Self::Field => "field",
            Self::Constant => "constant",
            Self::Variable => "variable",
            Self::Module => "module",
            Self::TypeAlias => "type_alias",
            Self::Macro => "macro",
            Self::Import => "import",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "function" | "fn" => Some(Self::Function),
            "method" => Some(Self::Method),
            "struct" => Some(Self::Struct),
            "class" => Some(Self::Class),
            "enum" => Some(Self::Enum),
            "variant" | "enum_variant" => Some(Self::EnumVariant),
            "interface" | "trait" | "protocol" => Some(Self::Interface),
            "field" => Some(Self::Field),
            "constant" | "const" => Some(Self::Constant),
            "variable" | "var" | "let" => Some(Self::Variable),
            "module" | "mod" => Some(Self::Module),
            "type_alias" | "type" => Some(Self::TypeAlias),
            "macro" => Some(Self::Macro),
            "import" | "use" => Some(Self::Import),
            _ => None,
        }
    }
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Span {
    pub start_line: u32,
    pub end_line: u32,
    pub start_col: u32,
    pub end_col: u32,
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start_line == self.end_line {
            write!(f, "L{}", self.start_line)
        } else {
            write!(f, "L{}-{}", self.start_line, self.end_line)
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    /// pub(crate) / protected / package-private
    Internal,
}
