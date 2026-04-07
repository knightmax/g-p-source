#[cfg(test)]
mod tests {
    use g_p_source::index::sled_store::SledStore;
    use g_p_source::index::store::SymbolStore;
    use g_p_source::parser::language_registry::SupportedLanguage;
    use g_p_source::parser::source_parser::SourceParser;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn pipeline_add_modify_delete() {
        let tmp = TempDir::new().unwrap();
        let store = SledStore::open(&tmp.path().join("db"), 10 * 1024 * 1024).unwrap();
        let mut parser = SourceParser::new().unwrap();

        // 1. Add a file
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        let file_path = src_dir.join("service.java");
        fs::write(
            &file_path,
            r#"
public class UserService {
    public void findById(int id) {}
}
"#,
        )
        .unwrap();

        let content = fs::read(&file_path).unwrap();
        let tree = parser
            .parse(&content, SupportedLanguage::Java, None)
            .unwrap();
        let symbols = parser.extract_symbols(&tree, &content, SupportedLanguage::Java, "src/service.java");
        store
            .upsert_file_symbols("src/service.java", &symbols)
            .unwrap();

        let results = store.symbols_in_file("src/service.java").unwrap();
        assert!(results.len() >= 1);

        // 2. Modify the file
        fs::write(
            &file_path,
            r#"
public class UserService {
    public void findById(int id) {}
    public void deleteById(int id) {}
}
"#,
        )
        .unwrap();

        let content2 = fs::read(&file_path).unwrap();
        let tree2 = parser
            .parse(&content2, SupportedLanguage::Java, None)
            .unwrap();
        let symbols2 = parser.extract_symbols(&tree2, &content2, SupportedLanguage::Java, "src/service.java");
        store
            .upsert_file_symbols("src/service.java", &symbols2)
            .unwrap();

        let results2 = store.symbols_in_file("src/service.java").unwrap();
        assert!(results2.len() > results.len());

        // 3. Delete the file
        fs::remove_file(&file_path).unwrap();
        store.remove_file("src/service.java").unwrap();

        let results3 = store.symbols_in_file("src/service.java").unwrap();
        assert!(results3.is_empty());
    }
}
