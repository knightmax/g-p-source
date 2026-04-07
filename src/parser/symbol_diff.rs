use super::symbol::Symbol;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct SymbolDiff {
    pub added: Vec<Symbol>,
    pub removed: Vec<Symbol>,
    pub modified: Vec<Symbol>,
}

impl SymbolDiff {
    #[allow(dead_code)]
    pub fn compute(old: &[Symbol], new: &[Symbol]) -> Self {
        use std::collections::HashMap;

        let old_map: HashMap<&str, &Symbol> =
            old.iter().map(|s| (s.qualified_name.as_str(), s)).collect();
        let new_map: HashMap<&str, &Symbol> =
            new.iter().map(|s| (s.qualified_name.as_str(), s)).collect();

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut modified = Vec::new();

        for (name, sym) in &new_map {
            match old_map.get(name) {
                None => added.push((*sym).clone()),
                Some(old_sym) => {
                    if old_sym.start_line != sym.start_line
                        || old_sym.end_line != sym.end_line
                        || old_sym.kind != sym.kind
                        || old_sym.visibility != sym.visibility
                    {
                        modified.push((*sym).clone());
                    }
                }
            }
        }

        for (name, sym) in &old_map {
            if !new_map.contains_key(name) {
                removed.push((*sym).clone());
            }
        }

        Self {
            added,
            removed,
            modified,
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }
}
