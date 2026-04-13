use crate::parser::c::CParser;
use crate::parser::cpp::CppParser;
use crate::parser::dart::DartParser;
use crate::parser::go::GoParser;
use crate::parser::kotlin::KotlinParser;
use crate::parser::python::PythonParser;
use crate::parser::rust::RustParser;
use crate::parser::swift::SwiftParser;
use crate::parser::traits::LanguageParser;
use crate::parser::typescript::TypeScriptParser;
use std::collections::HashMap;
use std::path::Path;

/// Maps file extensions to language parsers.
pub struct LanguageRegistry {
    parsers: Vec<Box<dyn LanguageParser>>,
    extension_map: HashMap<String, usize>,
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            parsers: Vec::new(),
            extension_map: HashMap::new(),
        };

        reg.register(Box::new(RustParser));
        reg.register(Box::new(TypeScriptParser));
        reg.register(Box::new(PythonParser));
        reg.register(Box::new(SwiftParser));
        reg.register(Box::new(GoParser));
        reg.register(Box::new(DartParser));
        reg.register(Box::new(KotlinParser));
        reg.register(Box::new(CParser));
        reg.register(Box::new(CppParser));

        reg
    }

    fn register(&mut self, parser: Box<dyn LanguageParser>) {
        let idx = self.parsers.len();
        for ext in parser.extensions() {
            self.extension_map.insert(ext.to_string(), idx);
        }
        self.parsers.push(parser);
    }

    /// Get the parser for a given file path, or None if unsupported.
    /// Fast path: static match for the 6 built-in languages.
    /// Indices match registration order in new(): 0=Rust, 1=TS, 2=Python, 3=Swift, 4=Go, 5=Dart.
    /// Falls through to HashMap lookup for any dynamically registered parsers.
    pub fn parser_for(&self, path: &Path) -> Option<&dyn LanguageParser> {
        let ext = path.extension()?.to_str()?;
        let idx = match ext {
            "rs" => 0,
            "ts" | "tsx" | "js" | "jsx" => 1,
            "py" => 2,
            "swift" => 3,
            "go" => 4,
            "dart" => 5,
            "kt" | "kts" => 6,
            "c" | "h" => 7,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" => 8,
            _ => {
                return self
                    .extension_map
                    .get(ext)
                    .map(|&i| self.parsers[i].as_ref());
            }
        };
        Some(self.parsers[idx].as_ref())
    }

    /// Check if a file extension is supported.
    pub fn is_supported(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| self.extension_map.contains_key(e))
            .unwrap_or(false)
    }
}
