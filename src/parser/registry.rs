use crate::parser::traits::LanguageParser;
use crate::parser::rust::RustParser;
use crate::parser::typescript::TypeScriptParser;
use crate::parser::python::PythonParser;
use crate::parser::swift::SwiftParser;
use crate::parser::go::GoParser;
use std::collections::HashMap;
use std::path::Path;

/// Maps file extensions to language parsers.
pub struct LanguageRegistry {
    parsers: Vec<Box<dyn LanguageParser>>,
    extension_map: HashMap<String, usize>,
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
    pub fn parser_for(&self, path: &Path) -> Option<&dyn LanguageParser> {
        let ext = path.extension()?.to_str()?;
        let idx = self.extension_map.get(ext)?;
        Some(self.parsers[*idx].as_ref())
    }

    /// Check if a file extension is supported.
    pub fn is_supported(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| self.extension_map.contains_key(e))
            .unwrap_or(false)
    }
}
