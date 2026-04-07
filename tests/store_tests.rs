#[cfg(test)]
mod tests {
    use g_p_source::index::sled_store::SledStore;
    use g_p_source::index::store::SymbolStore;
    use g_p_source::index::types::FileMetadata;
    use g_p_source::parser::symbol::{Symbol, SymbolKind, Visibility};
    use tempfile::TempDir;

    fn make_store() -> (SledStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = SledStore::open(dir.path(), 10 * 1024 * 1024).unwrap();
        (store, dir)
    }

    fn make_symbol(name: &str, kind: SymbolKind, file: &str) -> Symbol {
        Symbol {
            name: name.to_string(),
            qualified_name: format!("{}.{}", file, name),
            kind,
            file: file.to_string(),
            start_line: 1,
            start_col: 1,
            end_line: 10,
            end_col: 1,
            parent: None,
            visibility: Visibility::Public,
        }
    }

    #[test]
    fn upsert_and_locate() {
        let (store, _dir) = make_store();
        let sym = make_symbol("UserService", SymbolKind::Class, "src/service.java");
        store.upsert_file_symbols("src/service.java", &[sym]).unwrap();

        let results = store.locate("src/service.java.UserService").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, "class");
    }

    #[test]
    fn remove_file_clears_symbols() {
        let (store, _dir) = make_store();
        let sym = make_symbol("Config", SymbolKind::Struct, "src/config.rs");
        store.upsert_file_symbols("src/config.rs", &[sym]).unwrap();

        store.remove_file("src/config.rs").unwrap();

        let results = store.locate("src/config.rs.Config").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn symbols_in_file() {
        let (store, _dir) = make_store();
        let symbols = vec![
            make_symbol("Foo", SymbolKind::Class, "src/foo.java"),
            make_symbol("bar", SymbolKind::Method, "src/foo.java"),
        ];
        store.upsert_file_symbols("src/foo.java", &symbols).unwrap();

        let results = store.symbols_in_file("src/foo.java").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn symbols_by_kind() {
        let (store, _dir) = make_store();
        let symbols = vec![
            make_symbol("A", SymbolKind::Class, "a.java"),
            make_symbol("B", SymbolKind::Class, "b.java"),
            make_symbol("f", SymbolKind::Function, "c.rs"),
        ];
        store.upsert_file_symbols("a.java", &[symbols[0].clone()]).unwrap();
        store.upsert_file_symbols("b.java", &[symbols[1].clone()]).unwrap();
        store.upsert_file_symbols("c.rs", &[symbols[2].clone()]).unwrap();

        let classes = store.symbols_by_kind("class").unwrap();
        assert_eq!(classes.len(), 2);
    }

    #[test]
    fn file_metadata_roundtrip() {
        let (store, _dir) = make_store();
        let meta = FileMetadata {
            mtime: 1234567890,
            hash: vec![1, 2, 3, 4],
            symbol_count: 42,
        };
        store.set_file_meta("src/main.rs", &meta).unwrap();

        let loaded = store.get_file_meta("src/main.rs").unwrap().unwrap();
        assert_eq!(loaded.mtime, 1234567890);
        assert_eq!(loaded.hash, vec![1, 2, 3, 4]);
        assert_eq!(loaded.symbol_count, 42);
    }

    #[test]
    fn upsert_replaces_old_symbols() {
        let (store, _dir) = make_store();
        let sym1 = make_symbol("Old", SymbolKind::Class, "src/lib.rs");
        store.upsert_file_symbols("src/lib.rs", &[sym1]).unwrap();

        let sym2 = make_symbol("New", SymbolKind::Struct, "src/lib.rs");
        store.upsert_file_symbols("src/lib.rs", &[sym2]).unwrap();

        let results = store.symbols_in_file("src/lib.rs").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, "struct");
    }
}
