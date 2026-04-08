#[cfg(test)]
mod tests {
    use g_p_source::index::sled_store::SledStore;
    use g_p_source::index::store::SymbolStore;
    use g_p_source::index::types::{ChangeOp, FileMetadata, WordLocation};
    use tempfile::TempDir;

    fn make_store() -> (SledStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = SledStore::open(dir.path(), 10 * 1024 * 1024).unwrap();
        (store, dir)
    }

    // --- list_all_files / file_tree ---

    #[test]
    fn list_all_files_returns_indexed_files() {
        let (store, _dir) = make_store();
        store
            .set_file_meta(
                "src/main.rs",
                &FileMetadata {
                    mtime: 100,
                    hash: vec![1],
                    symbol_count: 5,
                    language: "rust".into(),
                    line_count: 50,
                },
            )
            .unwrap();
        store
            .set_file_meta(
                "src/lib.ts",
                &FileMetadata {
                    mtime: 200,
                    hash: vec![2],
                    symbol_count: 3,
                    language: "typescript".into(),
                    line_count: 30,
                },
            )
            .unwrap();

        let files = store.list_all_files().unwrap();
        assert_eq!(files.len(), 2);
        // Check that both files are present
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"src/main.rs"));
        assert!(paths.contains(&"src/lib.ts"));
    }

    // --- hot_files ---

    #[test]
    fn hot_files_returns_most_recent_first() {
        let (store, _dir) = make_store();
        store
            .set_file_meta(
                "old.rs",
                &FileMetadata {
                    mtime: 100,
                    hash: vec![1],
                    symbol_count: 1,
                    language: "rust".into(),
                    line_count: 10,
                },
            )
            .unwrap();
        store
            .set_file_meta(
                "new.rs",
                &FileMetadata {
                    mtime: 999,
                    hash: vec![2],
                    symbol_count: 2,
                    language: "rust".into(),
                    line_count: 20,
                },
            )
            .unwrap();
        store
            .set_file_meta(
                "mid.rs",
                &FileMetadata {
                    mtime: 500,
                    hash: vec![3],
                    symbol_count: 3,
                    language: "rust".into(),
                    line_count: 15,
                },
            )
            .unwrap();

        let hot = store.hot_files(2).unwrap();
        assert_eq!(hot.len(), 2);
        assert_eq!(hot[0].0, "new.rs");
        assert_eq!(hot[1].0, "mid.rs");
    }

    // --- word index ---

    #[test]
    fn word_index_upsert_and_lookup() {
        let (store, _dir) = make_store();
        let words = vec![
            WordLocation {
                file: "UserService".to_string(),
                line: 5,
            },
            WordLocation {
                file: "handleRequest".to_string(),
                line: 20,
            },
        ];
        store.upsert_word_index("src/service.java", &words).unwrap();

        let results = store.lookup_word("UserService").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file, "src/service.java");
        assert_eq!(results[0].line, 5);
    }

    #[test]
    fn word_index_remove_clears_entries() {
        let (store, _dir) = make_store();
        let words = vec![WordLocation {
            file: "Config".to_string(),
            line: 1,
        }];
        store.upsert_word_index("src/config.rs", &words).unwrap();

        store.remove_word_index("src/config.rs").unwrap();
        let results = store.lookup_word("Config").unwrap();
        assert!(results.is_empty());
    }

    // --- trigram index ---

    #[test]
    fn trigram_index_search_intersects_correctly() {
        let (store, _dir) = make_store();
        // File A has trigrams: "abc", "bcd", "xyz"
        store
            .upsert_trigram_index("fileA.rs", &["abc".into(), "bcd".into(), "xyz".into()])
            .unwrap();
        // File B has trigrams: "abc", "bcd"
        store
            .upsert_trigram_index("fileB.rs", &["abc".into(), "bcd".into()])
            .unwrap();

        // Search for "abc" + "bcd" should match both
        let results = store
            .search_trigrams(&["abc".into(), "bcd".into()])
            .unwrap();
        assert_eq!(results.len(), 2);

        // Search for "abc" + "bcd" + "xyz" should only match fileA
        let results = store
            .search_trigrams(&["abc".into(), "bcd".into(), "xyz".into()])
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results.contains(&"fileA.rs".to_string()));
    }

    #[test]
    fn trigram_index_remove_clears_entries() {
        let (store, _dir) = make_store();
        store
            .upsert_trigram_index("file.rs", &["abc".into(), "def".into()])
            .unwrap();
        store.remove_trigram_index("file.rs").unwrap();

        let results = store.search_trigrams(&["abc".into()]).unwrap();
        assert!(results.is_empty());
    }

    // --- changes tracking ---

    #[test]
    fn changes_tracking_records_and_queries() {
        let (store, _dir) = make_store();

        assert_eq!(store.current_seq().unwrap(), 0);

        let seq1 = store.record_change("file1.rs", ChangeOp::Upsert).unwrap();
        assert_eq!(seq1, 1);

        let seq2 = store.record_change("file2.rs", ChangeOp::Upsert).unwrap();
        assert_eq!(seq2, 2);

        let seq3 = store.record_change("file1.rs", ChangeOp::Remove).unwrap();
        assert_eq!(seq3, 3);

        // Query changes since seq 0 → all 3
        let changes = store.changes_since(0).unwrap();
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0].file_path, "file1.rs");
        assert_eq!(changes[2].file_path, "file1.rs");

        // Query changes since seq 2 → only seq 3
        let changes = store.changes_since(2).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].seq, 3);

        assert_eq!(store.current_seq().unwrap(), 3);
    }

    // --- sensitive file blocking ---

    #[test]
    fn sensitive_file_detection() {
        use g_p_source::sensitive::is_sensitive_file;
        use std::path::Path;

        assert!(is_sensitive_file(Path::new(".env")));
        assert!(is_sensitive_file(Path::new(".env.local")));
        assert!(is_sensitive_file(Path::new("server.pem")));
        assert!(is_sensitive_file(Path::new("private.key")));
        assert!(is_sensitive_file(Path::new("id_rsa")));
        assert!(is_sensitive_file(Path::new("credentials.json")));

        assert!(!is_sensitive_file(Path::new("src/main.rs")));
        assert!(!is_sensitive_file(Path::new("package.json")));
        assert!(!is_sensitive_file(Path::new("README.md")));
    }

    // --- FileMetadata with new fields ---

    #[test]
    fn file_metadata_language_and_lines() {
        let (store, _dir) = make_store();
        let meta = FileMetadata {
            mtime: 1000,
            hash: vec![42],
            symbol_count: 10,
            language: "python".to_string(),
            line_count: 200,
        };
        store.set_file_meta("src/app.py", &meta).unwrap();

        let loaded = store.get_file_meta("src/app.py").unwrap().unwrap();
        assert_eq!(loaded.language, "python");
        assert_eq!(loaded.line_count, 200);
    }
}
