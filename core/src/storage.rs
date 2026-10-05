// SPDX-License-Identifier: MPL-2.0
//! Storage - persistence layer for notebooks

use crate::notebook::{LoadError, LoadReport, Notebook};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during storage operations
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Cannot load notebook: {0}")]
    Load(#[from] LoadError),
}

/// Storage trait for notebook persistence
pub trait Storage {
    /// Save a notebook
    fn save(&self, notebook: &Notebook, path: &Path) -> Result<(), StorageError>;

    /// Load a notebook
    fn load(&self, path: &Path) -> Result<Notebook, StorageError>;
}

/// JSON file storage implementation
pub struct JsonStorage;

impl JsonStorage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Persist the directory entry created by a rename (Unix: fsync the parent
/// directory). Elsewhere directories cannot be opened for syncing; the file
/// itself was already synced.
#[cfg(unix)]
fn sync_parent_dir(path: &Path) -> std::io::Result<()> {
    match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => std::fs::File::open(dir)?.sync_all(),
        _ => std::fs::File::open(".")?.sync_all(),
    }
}

#[cfg(not(unix))]
fn sync_parent_dir(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

impl Storage for JsonStorage {
    fn save(&self, notebook: &Notebook, path: &Path) -> Result<(), StorageError> {
        use std::io::Write;
        let json = serde_json::to_string_pretty(notebook)?;
        // Write a sibling temp file, flush it to disk, and rename it over the
        // target: a crash mid-write leaves the previous notebook intact, never
        // a truncated one. Syncing the file before the rename (and, on Unix,
        // the directory after it) makes the new version durable across power
        // loss, not just process death.
        let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
        tmp_name.push(".tmp");
        let tmp = path.with_file_name(tmp_name);
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)?;
        sync_parent_dir(path)?;
        Ok(())
    }

    fn load(&self, path: &Path) -> Result<Notebook, StorageError> {
        Ok(self.load_with_report(path)?.0)
    }
}

impl JsonStorage {
    /// Load like [`Storage::load`], also returning what migration and repair
    /// changed (see [`Notebook::load_json`]).
    pub fn load_with_report(&self, path: &Path) -> Result<(Notebook, LoadReport), StorageError> {
        if !path.exists() {
            return Err(StorageError::NotFound(path.display().to_string()));
        }
        let json = std::fs::read_to_string(path)?;
        Ok(Notebook::load_json(&json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.nexia.json");

        let mut notebook = Notebook::new("Test Notebook");
        let id1 = notebook.create_note("Note 1");
        let id2 = notebook.create_note("Note 2");
        notebook.link_notes(id1, id2).unwrap();

        let storage = JsonStorage::new();

        // Save
        storage.save(&notebook, &path).unwrap();
        assert!(path.exists());

        // Load
        let loaded = storage.load(&path).unwrap();
        assert_eq!(loaded.name, "Test Notebook");
        assert_eq!(loaded.len(), 2);

        let note1 = loaded.get_note(&id1).unwrap();
        assert!(note1.links_to(&id2));
    }

    #[test]
    fn test_load_not_found() {
        let storage = JsonStorage::new();
        let result = storage.load(Path::new("/nonexistent/path.json"));
        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }
}
