//! In-memory filesystem implementation for file manipulation

use crate::error::{Error, Result};
use glob::Pattern;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Represents a file with content and metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct File {
    /// File content as bytes
    pub content: Vec<u8>,
    /// File permissions (simplified as u32)
    pub permissions: u32,
    /// File modification time
    pub modified_time: SystemTime,
}

#[allow(dead_code)]
impl File {
    /// Create a new file with content
    pub fn new(content: Vec<u8>) -> Self {
        Self {
            content,
            permissions: 0o644, // Default permissions
            modified_time: SystemTime::now(),
        }
    }

    /// Create a new file from string content
    pub fn from_string(content: &str) -> Self {
        Self::new(content.as_bytes().to_vec())
    }

    /// Get file size in bytes
    pub fn size(&self) -> usize {
        self.content.len()
    }
}

/// In-memory filesystem for fast file manipulation
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct MemoryFS {
    /// Files stored as path -> content mapping
    files: HashMap<PathBuf, File>,
}

#[allow(dead_code)]
impl MemoryFS {
    /// Create a new empty filesystem
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update a file
    pub fn add_file<P: AsRef<Path>>(&mut self, path: P, file: File) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        self.files.insert(path, file);
        Ok(())
    }

    /// Add a file with content
    pub fn add_file_content<P: AsRef<Path>>(&mut self, path: P, content: Vec<u8>) -> Result<()> {
        self.add_file(path, File::new(content))
    }

    /// Add a file with string content
    pub fn add_file_string<P: AsRef<Path>>(&mut self, path: P, content: &str) -> Result<()> {
        self.add_file(path, File::from_string(content))
    }

    /// Get a file by path
    pub fn get_file<P: AsRef<Path>>(&self, path: P) -> Option<&File> {
        self.files.get(path.as_ref())
    }

    /// Remove a file
    pub fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Option<File>> {
        Ok(self.files.remove(path.as_ref()))
    }

    /// Check if a file exists
    pub fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.files.contains_key(path.as_ref())
    }

    /// List all files
    pub fn list_files(&self) -> Vec<PathBuf> {
        self.files.keys().cloned().collect()
    }

    /// List files matching a glob pattern
    pub fn list_files_glob(&self, pattern: &str) -> Result<Vec<PathBuf>> {
        let pattern = Pattern::new(pattern).map_err(Error::Glob)?;
        let mut matches = Vec::new();

        for path in self.files.keys() {
            if let Some(path_str) = path.to_str()
                && pattern.matches(path_str)
            {
                matches.push(path.clone());
            }
        }

        Ok(matches)
    }

    /// Rename a file
    pub fn rename_file<P: AsRef<Path>, Q: AsRef<Path>>(&mut self, from: P, to: Q) -> Result<()> {
        let from_path = from.as_ref();
        let to_path = to.as_ref();

        if let Some(file) = self.files.remove(from_path) {
            self.files.insert(to_path.to_path_buf(), file);
            Ok(())
        } else {
            Err(Error::Filesystem {
                message: format!("File not found: {}", from_path.display()),
            })
        }
    }

    /// Copy a file
    pub fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(&mut self, from: P, to: Q) -> Result<()> {
        let from_path = from.as_ref();
        let to_path = to.as_ref();

        if let Some(file) = self.files.get(from_path) {
            self.files.insert(to_path.to_path_buf(), file.clone());
            Ok(())
        } else {
            Err(Error::Filesystem {
                message: format!("File not found: {}", from_path.display()),
            })
        }
    }

    /// Get the number of files
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if filesystem is empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Clear all files
    pub fn clear(&mut self) {
        self.files.clear();
    }

    /// Merge another filesystem into this one (last-write-wins)
    pub fn merge(&mut self, other: &MemoryFS) {
        for (path, file) in &other.files {
            self.files.insert(path.clone(), file.clone());
        }
    }

    /// Iterate over all files as (path, file) pairs
    pub fn files(&self) -> impl Iterator<Item = (&PathBuf, &File)> {
        self.files.iter()
    }
}
