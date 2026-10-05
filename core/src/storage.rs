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

/// Create a new temp file beside `path`, exclusively (`O_EXCL`; mode 0600
/// on Unix), with a name unique to this process and attempt.
fn create_unique_temp(path: &Path) -> std::io::Result<(std::path::PathBuf, std::fs::File)> {
    let base = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut last_err = None;
    for attempt in 0..32u32 {
        let name = format!(".{base}.{}.{nanos}.{attempt}.tmp", std::process::id());
        let tmp = path.with_file_name(name);
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        match opts.open(&tmp) {
            Ok(file) => return Ok((tmp, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last_err = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::other("no unique temp file name")))
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
        // Write a fresh, exclusively created temp file next to the target,
        // flush it to disk, and rename it over the target: a crash mid-write
        // leaves the previous notebook intact, concurrent saves never share a
        // temp file, and a planted `.tmp` symlink cannot redirect the write.
        // The new file keeps the old one's permissions (a new notebook is
        // private); syncing the file and then (Unix) the directory makes the
        // result durable across power loss.
        let (tmp, mut file) = create_unique_temp(path)?;
        let written = (|| -> std::io::Result<()> {
            if let Ok(meta) = std::fs::metadata(path) {
                std::fs::set_permissions(&tmp, meta.permissions())?;
            }
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
            std::fs::rename(&tmp, path)?;
            sync_parent_dir(path)
        })();
        if let Err(e) = written {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
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

    #[cfg(unix)]
    #[test]
    fn save_keeps_permissions_and_new_files_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let storage = JsonStorage::new();
        let nb = Notebook::new("Perms");

        let fresh = dir.path().join("new.nexia.json");
        storage.save(&nb, &fresh).unwrap();
        let mode = std::fs::metadata(&fresh).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "a new notebook is private");

        let shared = dir.path().join("shared.nexia.json");
        std::fs::write(&shared, "{}").unwrap();
        std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o640)).unwrap();
        storage.save(&nb, &shared).unwrap();
        let mode = std::fs::metadata(&shared).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640, "an existing notebook keeps its mode");

        let names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().all(|n| !n.ends_with(".tmp")),
            "no temp files left: {names:?}"
        );
    }

    #[test]
    fn test_load_not_found() {
        let storage = JsonStorage::new();
        let result = storage.load(Path::new("/nonexistent/path.json"));
        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }
}
