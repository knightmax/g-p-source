use super::store::SymbolStore;
use super::types::{FileMetadata, SymbolRecord, SymbolRef};
use crate::parser::Symbol;

pub struct SledStore {
    _db: sled::Db,
    sym_def: sled::Tree,
    sym_file: sled::Tree,
    sym_kind: sled::Tree,
    dep_import: sled::Tree,
    dep_reverse: sled::Tree,
    meta_file: sled::Tree,
}

impl SledStore {
    pub fn open(path: &std::path::Path, cache_capacity: u64) -> anyhow::Result<Self> {
        let db = sled::Config::new()
            .path(path)
            .cache_capacity(cache_capacity)
            .open()?;

        let sym_def = db.open_tree("sym:def")?;
        let sym_file = db.open_tree("sym:file")?;
        let sym_kind = db.open_tree("sym:kind")?;
        let dep_import = db.open_tree("dep:import")?;
        let dep_reverse = db.open_tree("dep:reverse")?;
        let meta_file = db.open_tree("meta:file")?;

        Ok(Self {
            _db: db,
            sym_def,
            sym_file,
            sym_kind,
            dep_import,
            dep_reverse,
            meta_file,
        })
    }
}

impl SymbolStore for SledStore {
    fn upsert_file_symbols(&self, file_path: &str, symbols: &[Symbol]) -> anyhow::Result<()> {
        // First remove old entries for this file
        self.remove_file(file_path)?;

        // Insert new symbols
        let mut def_batch = sled::Batch::default();
        let mut file_batch = sled::Batch::default();
        let mut kind_batch = sled::Batch::default();

        for sym in symbols {
            let record = SymbolRecord {
                kind: sym.kind.to_string(),
                file: sym.file.clone(),
                start_line: sym.start_line,
                start_col: sym.start_col,
                end_line: sym.end_line,
                end_col: sym.end_col,
                visibility: format!("{:?}", sym.visibility),
                parent: sym.parent.clone(),
            };
            let value = bincode::serialize(&record)?;

            def_batch.insert(sym.qualified_name.as_bytes(), value);

            let file_key = format!("{}\x00{}", file_path, sym.name);
            let sym_ref = SymbolRef {
                qualified_name: sym.qualified_name.clone(),
            };
            file_batch.insert(file_key.as_bytes(), bincode::serialize(&sym_ref)?);

            let kind_key = format!("{}\x00{}", sym.kind, sym.qualified_name);
            kind_batch.insert(kind_key.as_bytes(), &[]);

            // Handle imports
            if sym.kind == crate::parser::SymbolKind::Import {
                let import_key = format!("{}\x00{}", file_path, sym.name);
                self.dep_import.insert(import_key.as_bytes(), &[])?;
                let reverse_key = format!("{}\x00{}", sym.name, file_path);
                self.dep_reverse.insert(reverse_key.as_bytes(), &[])?;
            }
        }

        self.sym_def.apply_batch(def_batch)?;
        self.sym_file.apply_batch(file_batch)?;
        self.sym_kind.apply_batch(kind_batch)?;

        Ok(())
    }

    fn remove_file(&self, file_path: &str) -> anyhow::Result<()> {
        let prefix = format!("{}\x00", file_path);

        // Remove from sym:file and collect qualified names
        let mut qualified_names = Vec::new();
        for item in self.sym_file.scan_prefix(prefix.as_bytes()) {
            let (key, value) = item?;
            if let Ok(sym_ref) = bincode::deserialize::<SymbolRef>(&value) {
                qualified_names.push(sym_ref.qualified_name);
            }
            self.sym_file.remove(key)?;
        }

        // Remove from sym:def and sym:kind
        for qn in &qualified_names {
            self.sym_def.remove(qn.as_bytes())?;
            // We'd need the kind to remove from sym:kind precisely,
            // so scan for it
            for item in self.sym_kind.scan_prefix(b"") {
                let (key, _) = item?;
                let key_str = String::from_utf8_lossy(&key);
                if key_str.ends_with(qn.as_str()) {
                    self.sym_kind.remove(key)?;
                    break;
                }
            }
        }

        // Remove imports
        for item in self.dep_import.scan_prefix(prefix.as_bytes()) {
            let (key, _) = item?;
            self.dep_import.remove(&key)?;
        }

        // Remove reverse imports referencing this file
        // This is more expensive but necessary for consistency
        let suffix = format!("\x00{}", file_path);
        for item in self.dep_reverse.iter() {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if key_str.ends_with(&suffix) {
                self.dep_reverse.remove(&key)?;
            }
        }

        self.meta_file.remove(file_path.as_bytes())?;

        Ok(())
    }

    fn locate(&self, name: &str) -> anyhow::Result<Vec<SymbolRecord>> {
        let mut results = Vec::new();

        // Try exact match first
        if let Some(value) = self.sym_def.get(name.as_bytes())? {
            let record: SymbolRecord = bincode::deserialize(&value)?;
            results.push(record);
            return Ok(results);
        }

        // Prefix scan for partial matches
        for item in self.sym_def.iter() {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if key_str.contains(name) {
                let record: SymbolRecord = bincode::deserialize(&value)?;
                results.push(record);
            }
        }

        // Sort by visibility rank (public first)
        results.sort_by(|a, b| a.visibility.cmp(&b.visibility));

        Ok(results)
    }

    fn symbols_in_file(&self, file_path: &str) -> anyhow::Result<Vec<SymbolRecord>> {
        let prefix = format!("{}\x00", file_path);
        let mut results = Vec::new();

        for item in self.sym_file.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            let sym_ref: SymbolRef = bincode::deserialize(&value)?;
            if let Some(record_bytes) = self.sym_def.get(sym_ref.qualified_name.as_bytes())? {
                let record: SymbolRecord = bincode::deserialize(&record_bytes)?;
                results.push(record);
            }
        }

        Ok(results)
    }

    fn symbols_by_kind(&self, kind: &str) -> anyhow::Result<Vec<String>> {
        let prefix = format!("{}\x00", kind);
        let mut results = Vec::new();

        for item in self.sym_kind.scan_prefix(prefix.as_bytes()) {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if let Some(qn) = key_str.strip_prefix(&prefix) {
                results.push(qn.to_string());
            }
        }

        Ok(results)
    }

    fn get_imports(&self, file_path: &str) -> anyhow::Result<Vec<String>> {
        let prefix = format!("{}\x00", file_path);
        let mut results = Vec::new();

        for item in self.dep_import.scan_prefix(prefix.as_bytes()) {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if let Some(imported) = key_str.strip_prefix(&prefix) {
                results.push(imported.to_string());
            }
        }

        Ok(results)
    }

    fn get_importers(&self, file_path: &str) -> anyhow::Result<Vec<String>> {
        let prefix = format!("{}\x00", file_path);
        let mut results = Vec::new();

        for item in self.dep_reverse.scan_prefix(prefix.as_bytes()) {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            if let Some(importer) = key_str.strip_prefix(&prefix) {
                results.push(importer.to_string());
            }
        }

        Ok(results)
    }

    fn get_file_meta(&self, file_path: &str) -> anyhow::Result<Option<FileMetadata>> {
        match self.meta_file.get(file_path.as_bytes())? {
            Some(value) => Ok(Some(bincode::deserialize(&value)?)),
            None => Ok(None),
        }
    }

    fn set_file_meta(&self, file_path: &str, meta: &FileMetadata) -> anyhow::Result<()> {
        let value = bincode::serialize(meta)?;
        self.meta_file.insert(file_path.as_bytes(), value)?;
        Ok(())
    }
}
