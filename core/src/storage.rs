// SPDX-License-Identifier: AGPL-3.0-or-later
//! Storage - persistence layer for notebooks

use crate::notebook::Notebook;
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

impl Storage for JsonStorage {
    fn save(&self, notebook: &Notebook, path: &Path) -> Result<(), StorageError> {
        let json = serde_json::to_string_pretty(notebook)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn load(&self, path: &Path) -> Result<Notebook, StorageError> {
        if !path.exists() {
            return Err(StorageError::NotFound(path.display().to_string()));
        }

        let json = std::fs::read_to_string(path)?;
        let notebook = serde_json::from_str(&json)?;
        Ok(notebook)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
