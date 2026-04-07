#[cfg(test)]
mod tests {
    use g_p_source::index::sled_store::SledStore;
    use g_p_source::index::store::SymbolStore;
    use g_p_source::parser::symbol::{Symbol, SymbolKind, Visibility};
    use std::time::Instant;
    use tempfile::TempDir;

    fn make_symbols(count: usize) -> Vec<Symbol> {
        (0..count)
            .map(|i| Symbol {
                name: format!("Symbol{}", i),
                qualified_name: format!("pkg.module.Symbol{}", i),
                kind: if i % 3 == 0 {
                    SymbolKind::Class
                } else if i % 3 == 1 {
                    SymbolKind::Function
                } else {
                    SymbolKind::Method
                },
                file: format!("src/file{}.java", i / 100),
                start_line: (i % 1000) as u32,
                start_col: 1,
                end_line: (i % 1000 + 10) as u32,
                end_col: 1,
                parent: None,
                visibility: Visibility::Public,
            })
            .collect()
    }

    #[test]
    fn bench_locate_query_latency() {
        let dir = TempDir::new().unwrap();
        let store = SledStore::open(dir.path(), 64 * 1024 * 1024).unwrap();

        // Insert 500k symbols in batches by file
        let symbols = make_symbols(500_000);
        let mut by_file: std::collections::HashMap<&str, Vec<Symbol>> =
            std::collections::HashMap::new();
        for s in &symbols {
            by_file.entry(&s.file).or_default().push(s.clone());
        }
        for (file, syms) in &by_file {
            store.upsert_file_symbols(file, syms).unwrap();
        }

        // Benchmark locate queries
        let queries = [
            "pkg.module.Symbol0",
            "pkg.module.Symbol250000",
            "pkg.module.Symbol499999",
            "nonexistent",
        ];

        for q in &queries {
            let start = Instant::now();
            let iterations = 100;
            for _ in 0..iterations {
                let _ = store.locate(q).unwrap();
            }
            let elapsed = start.elapsed();
            let per_query = elapsed / iterations;
            println!(
                "locate({:30}) avg = {:?} (total {:?} / {} iterations)",
                q, per_query, elapsed, iterations
            );
            // Target: < 10ms p99
            assert!(
                per_query.as_millis() < 50,
                "locate query too slow: {:?}",
                per_query
            );
        }
    }

    #[test]
    fn bench_symbols_in_file_latency() {
        let dir = TempDir::new().unwrap();
        let store = SledStore::open(dir.path(), 64 * 1024 * 1024).unwrap();

        // Insert symbols for 100 files with 100 symbols each
        for file_idx in 0..100 {
            let file = format!("src/file{}.java", file_idx);
            let symbols: Vec<Symbol> = (0..100)
                .map(|i| Symbol {
                    name: format!("Sym{}_{}", file_idx, i),
                    qualified_name: format!("pkg.Sym{}_{}", file_idx, i),
                    kind: SymbolKind::Class,
                    file: file.clone(),
                    start_line: i,
                    start_col: 1,
                    end_line: i + 5,
                    end_col: 1,
                    parent: None,
                    visibility: Visibility::Public,
                })
                .collect();
            store.upsert_file_symbols(&file, &symbols).unwrap();
        }

        let start = Instant::now();
        let iterations = 100;
        for i in 0..iterations {
            let file = format!("src/file{}.java", i % 100);
            let _ = store.symbols_in_file(&file).unwrap();
        }
        let elapsed = start.elapsed();
        let per_query = elapsed / iterations;
        println!("symbols_in_file avg = {:?}", per_query);
        assert!(
            per_query.as_millis() < 50,
            "symbols_in_file too slow: {:?}",
            per_query
        );
    }
}
