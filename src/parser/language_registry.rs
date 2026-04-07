use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedLanguage {
    Java,
    TypeScript,
    Tsx,
    Python,
    Rust,
    CSharp,
}

pub struct LanguageRegistry {
    ext_map: HashMap<String, SupportedLanguage>,
    languages: HashMap<SupportedLanguage, tree_sitter::Language>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        let mut ext_map = HashMap::new();
        ext_map.insert("java".into(), SupportedLanguage::Java);
        ext_map.insert("ts".into(), SupportedLanguage::TypeScript);
        ext_map.insert("tsx".into(), SupportedLanguage::Tsx);
        ext_map.insert("py".into(), SupportedLanguage::Python);
        ext_map.insert("rs".into(), SupportedLanguage::Rust);
        ext_map.insert("cs".into(), SupportedLanguage::CSharp);

        let mut languages = HashMap::new();
        languages.insert(SupportedLanguage::Java, tree_sitter_java::LANGUAGE.into());
        languages.insert(
            SupportedLanguage::TypeScript,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        );
        languages.insert(
            SupportedLanguage::Tsx,
            tree_sitter_typescript::LANGUAGE_TSX.into(),
        );
        languages.insert(
            SupportedLanguage::Python,
            tree_sitter_python::LANGUAGE.into(),
        );
        languages.insert(SupportedLanguage::Rust, tree_sitter_rust::LANGUAGE.into());
        languages.insert(
            SupportedLanguage::CSharp,
            tree_sitter_c_sharp::LANGUAGE.into(),
        );

        Self { ext_map, languages }
    }

    pub fn language_for_extension(&self, ext: &str) -> Option<SupportedLanguage> {
        self.ext_map.get(ext).copied()
    }

    pub fn get_ts_language(&self, lang: SupportedLanguage) -> Option<tree_sitter::Language> {
        self.languages.get(&lang).cloned()
    }
}
