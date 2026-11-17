use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

/// Lightweight, cross-platform directory change detector based on file paths, sizes, and mtimes.
#[derive(Debug, Default)]
pub struct LocalPathWatcher {
    last_fingerprints: HashMap<String, [u8; 32]>,
}

impl LocalPathWatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the directory tree for `connector_id` changed since the previous check.
    /// The first call seeds the baseline fingerprint and returns `false`.
    pub fn has_changed(&mut self, connector_id: &str, paths: &[PathBuf]) -> Result<bool> {
        let fp = compute_paths_fingerprint(paths)?;
        match self.last_fingerprints.get(connector_id) {
            Some(prev) if prev == &fp => Ok(false),
            Some(_) => {
                self.last_fingerprints.insert(connector_id.to_string(), fp);
                Ok(true)
            }
            None => {
                self.last_fingerprints.insert(connector_id.to_string(), fp);
                Ok(false)
            }
        }
    }
}

pub fn compute_paths_fingerprint(paths: &[PathBuf]) -> Result<[u8; 32]> {
    let mut entries: Vec<(String, u64, u128)> = Vec::new();

    for root in paths {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_ignored_dir(e.path()))
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            entries.push((
                entry.path().to_string_lossy().to_string(),
                meta.len(),
                mtime,
            ));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = Sha256::new();
    for (path, len, mtime) in entries {
        hasher.update(path.as_bytes());
        hasher.update(len.to_le_bytes());
        hasher.update(mtime.to_le_bytes());
    }
    Ok(hasher.finalize().into())
}

fn is_ignored_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(name, "node_modules" | "target" | ".git" | ".idea" | "dist")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_local_path_watcher_detects_file_change() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("README.md");
        fs::write(&file, "# Initial").unwrap();

        let mut watcher = LocalPathWatcher::new();
        let paths = vec![dir.path().to_path_buf()];

        // First call seeds baseline -> false
        assert!(!watcher.has_changed("md-1", &paths).unwrap());
        // Unchanged -> false
        assert!(!watcher.has_changed("md-1", &paths).unwrap());

        // Modify file -> true
        fs::write(&file, "# Updated content with more bytes").unwrap();
        assert!(watcher.has_changed("md-1", &paths).unwrap());
        // Subsequent check without changes -> false
        assert!(!watcher.has_changed("md-1", &paths).unwrap());
    }
}
