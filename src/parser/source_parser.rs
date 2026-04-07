use super::language_registry::{LanguageRegistry, SupportedLanguage};
use super::queries;
use super::symbol::{Symbol, SymbolKind, Visibility};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor, Tree};

pub struct SourceParser {
    parsers: std::collections::HashMap<SupportedLanguage, Parser>,
    registry: LanguageRegistry,
}

impl SourceParser {
    pub fn new() -> anyhow::Result<Self> {
        let registry = LanguageRegistry::new();
        let mut parsers = std::collections::HashMap::new();

        for lang in [
            SupportedLanguage::Java,
            SupportedLanguage::TypeScript,
            SupportedLanguage::Tsx,
            SupportedLanguage::Python,
            SupportedLanguage::Rust,
            SupportedLanguage::CSharp,
        ] {
            let mut parser = Parser::new();
            if let Some(ts_lang) = registry.get_ts_language(lang) {
                parser.set_language(&ts_lang)?;
            }
            parsers.insert(lang, parser);
        }

        Ok(Self { parsers, registry })
    }

    pub fn language_for_extension(&self, ext: &str) -> Option<SupportedLanguage> {
        self.registry.language_for_extension(ext)
    }

    pub fn parse(
        &mut self,
        source: &[u8],
        lang: SupportedLanguage,
        old_tree: Option<&Tree>,
    ) -> Option<Tree> {
        let parser = self.parsers.get_mut(&lang)?;
        parser.parse(source, old_tree)
    }

    pub fn extract_symbols(
        &self,
        tree: &Tree,
        source: &[u8],
        lang: SupportedLanguage,
        file_path: &str,
    ) -> Vec<Symbol> {
        let ts_lang = match self.registry.get_ts_language(lang) {
            Some(l) => l,
            None => return Vec::new(),
        };

        let query_src = match lang {
            SupportedLanguage::Java => queries::JAVA_QUERY,
            SupportedLanguage::TypeScript | SupportedLanguage::Tsx => queries::TYPESCRIPT_QUERY,
            SupportedLanguage::Python => queries::PYTHON_QUERY,
            SupportedLanguage::Rust => queries::RUST_QUERY,
            SupportedLanguage::CSharp => queries::CSHARP_QUERY,
        };

        let query = match Query::new(&ts_lang, query_src) {
            Ok(q) => q,
            Err(e) => {
                tracing::warn!(lang = ?lang, error = %e, "failed to compile query");
                return Vec::new();
            }
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source);

        let mut symbols = Vec::new();
        let capture_names = query.capture_names();

        loop {
            matches.advance();
            let m = match matches.get() {
                Some(m) => m,
                None => break,
            };
            let mut name_text = String::new();
            let mut def_node = None;
            let mut def_kind_str = "";

            for cap in m.captures {
                let cap_name: &str = &capture_names[cap.index as usize];
                if cap_name == "name" {
                    name_text =
                        cap.node.utf8_text(source).unwrap_or_default().to_string();
                } else if cap_name.starts_with("definition.") {
                    def_kind_str = &cap_name["definition.".len()..];
                    def_node = Some(cap.node);
                }
            }

            if let Some(node) = def_node {
                if name_text.is_empty() {
                    continue;
                }

                let kind = match def_kind_str {
                    "class" => SymbolKind::Class,
                    "interface" => SymbolKind::Interface,
                    "struct" => SymbolKind::Struct,
                    "enum" => SymbolKind::Enum,
                    "function" => SymbolKind::Function,
                    "method" => SymbolKind::Method,
                    "trait" => SymbolKind::Trait,
                    "module" => SymbolKind::Module,
                    "namespace" => SymbolKind::Namespace,
                    "import" => SymbolKind::Import,
                    "type_alias" => SymbolKind::TypeAlias,
                    "constant" => SymbolKind::Constant,
                    _ => continue,
                };

                let start = node.start_position();
                let end = node.end_position();

                // Detect visibility from source text before the node
                let visibility = detect_visibility(source, node.start_byte());

                symbols.push(Symbol {
                    name: name_text.clone(),
                    qualified_name: name_text.clone(), // simplified; pipeline enriches this
                    kind,
                    file: file_path.to_string(),
                    start_line: start.row as u32 + 1,
                    start_col: start.column as u32 + 1,
                    end_line: end.row as u32 + 1,
                    end_col: end.column as u32 + 1,
                    parent: None, // enriched later by the pipeline
                    visibility,
                });
            }
        }

        symbols
    }
}

fn detect_visibility(source: &[u8], start_byte: usize) -> Visibility {
    // Look backwards from the node start for visibility keywords
    let lookback = if start_byte > 30 { 30 } else { start_byte };
    let prefix = std::str::from_utf8(&source[start_byte - lookback..start_byte]).unwrap_or("");
    if prefix.contains("public") || prefix.contains("pub ") || prefix.contains("export") {
        Visibility::Public
    } else if prefix.contains("protected") {
        Visibility::Protected
    } else if prefix.contains("internal") {
        Visibility::Internal
    } else {
        Visibility::Private
    }
}
