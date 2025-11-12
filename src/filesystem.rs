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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_file_new() {
        let content = vec![1, 2, 3, 4, 5];
        let file = File::new(content.clone());

        assert_eq!(file.content, content);
        assert_eq!(file.size(), 5);
        assert_eq!(file.permissions, 0o644);
    }

    #[test]
    fn test_file_from_string() {
        let content = "Hello, world!";
        let file = File::from_string(content);

        assert_eq!(file.content, content.as_bytes());
        assert_eq!(file.size(), content.len());
    }

    #[test]
    fn test_file_size() {
        let empty_file = File::new(vec![]);
        assert_eq!(empty_file.size(), 0);

        let file = File::new(vec![42]);
        assert_eq!(file.size(), 1);

        let large_file = File::new(vec![0; 1000]);
        assert_eq!(large_file.size(), 1000);
    }

    #[test]
    fn test_memory_fs_new() {
        let fs = MemoryFS::new();
        assert!(fs.is_empty());
        assert_eq!(fs.len(), 0);
    }

    #[test]
    fn test_memory_fs_add_file() {
        let mut fs = MemoryFS::new();
        let file = File::from_string("test content");

        fs.add_file("test.txt", file).unwrap();

        assert!(!fs.is_empty());
        assert_eq!(fs.len(), 1);
        assert!(fs.exists("test.txt"));
    }

    #[test]
    fn test_memory_fs_add_file_content() {
        let mut fs = MemoryFS::new();
        let content = vec![1, 2, 3];

        fs.add_file_content("binary.dat", content.clone()).unwrap();

        assert!(fs.exists("binary.dat"));
        let retrieved = fs.get_file("binary.dat").unwrap();
        assert_eq!(retrieved.content, content);
    }

    #[test]
    fn test_memory_fs_add_file_string() {
        let mut fs = MemoryFS::new();

        fs.add_file_string("hello.txt", "Hello, World!").unwrap();

        assert!(fs.exists("hello.txt"));
        let retrieved = fs.get_file("hello.txt").unwrap();
        assert_eq!(
            String::from_utf8(retrieved.content.clone()).unwrap(),
            "Hello, World!"
        );
    }

    #[test]
    fn test_memory_fs_get_file() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("test.txt", "content").unwrap();

        let file = fs.get_file("test.txt").unwrap();
        assert_eq!(String::from_utf8(file.content.clone()).unwrap(), "content");

        assert!(fs.get_file("nonexistent.txt").is_none());
    }

    #[test]
    fn test_memory_fs_remove_file() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("test.txt", "content").unwrap();

        assert!(fs.exists("test.txt"));

        let removed = fs.remove_file("test.txt").unwrap().unwrap();
        assert_eq!(String::from_utf8(removed.content).unwrap(), "content");

        assert!(!fs.exists("test.txt"));
        assert!(fs.remove_file("nonexistent.txt").unwrap().is_none());
    }

    #[test]
    fn test_memory_fs_exists() {
        let mut fs = MemoryFS::new();

        assert!(!fs.exists("test.txt"));

        fs.add_file_string("test.txt", "content").unwrap();
        assert!(fs.exists("test.txt"));
    }

    #[test]
    fn test_memory_fs_list_files() {
        let mut fs = MemoryFS::new();

        assert!(fs.list_files().is_empty());

        fs.add_file_string("file1.txt", "content1").unwrap();
        fs.add_file_string("file2.txt", "content2").unwrap();
        fs.add_file_string("dir/file3.txt", "content3").unwrap();

        let files = fs.list_files();
        assert_eq!(files.len(), 3);
        assert!(files.contains(&PathBuf::from("file1.txt")));
        assert!(files.contains(&PathBuf::from("file2.txt")));
        assert!(files.contains(&PathBuf::from("dir/file3.txt")));
    }

    #[test]
    fn test_memory_fs_list_files_glob() {
        let mut fs = MemoryFS::new();

        fs.add_file_string("test.txt", "content").unwrap();
        fs.add_file_string("test.rs", "code").unwrap();
        fs.add_file_string("other.txt", "other").unwrap();
        fs.add_file_string("dir/test.txt", "nested").unwrap();

        let txt_files = fs.list_files_glob("*.txt").unwrap();
        assert_eq!(txt_files.len(), 3); // Matches test.txt, other.txt, and dir/test.txt
        assert!(txt_files.contains(&PathBuf::from("test.txt")));
        assert!(txt_files.contains(&PathBuf::from("other.txt")));
        assert!(txt_files.contains(&PathBuf::from("dir/test.txt")));

        let rs_files = fs.list_files_glob("*.rs").unwrap();
        assert_eq!(rs_files.len(), 1);
        assert!(rs_files.contains(&PathBuf::from("test.rs")));

        let all_nested = fs.list_files_glob("**/test.txt").unwrap();
        assert_eq!(all_nested.len(), 2);
        assert!(all_nested.contains(&PathBuf::from("test.txt")));
        assert!(all_nested.contains(&PathBuf::from("dir/test.txt")));
    }

    #[test]
    fn test_memory_fs_rename_file() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("old.txt", "content").unwrap();

        fs.rename_file("old.txt", "new.txt").unwrap();

        assert!(!fs.exists("old.txt"));
        assert!(fs.exists("new.txt"));
        let content = fs.get_file("new.txt").unwrap();
        assert_eq!(
            String::from_utf8(content.content.clone()).unwrap(),
            "content"
        );
    }

    #[test]
    fn test_memory_fs_rename_file_nonexistent() {
        let mut fs = MemoryFS::new();

        let result = fs.rename_file("nonexistent.txt", "new.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_fs_copy_file() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("source.txt", "content").unwrap();

        fs.copy_file("source.txt", "dest.txt").unwrap();

        assert!(fs.exists("source.txt"));
        assert!(fs.exists("dest.txt"));

        let source = fs.get_file("source.txt").unwrap();
        let dest = fs.get_file("dest.txt").unwrap();
        assert_eq!(source.content, dest.content);
    }

    #[test]
    fn test_memory_fs_copy_file_nonexistent() {
        let mut fs = MemoryFS::new();

        let result = fs.copy_file("nonexistent.txt", "dest.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_fs_len_and_is_empty() {
        let mut fs = MemoryFS::new();

        assert_eq!(fs.len(), 0);
        assert!(fs.is_empty());

        fs.add_file_string("file1.txt", "content").unwrap();
        assert_eq!(fs.len(), 1);
        assert!(!fs.is_empty());

        fs.add_file_string("file2.txt", "content").unwrap();
        assert_eq!(fs.len(), 2);
        assert!(!fs.is_empty());

        fs.clear();
        assert_eq!(fs.len(), 0);
        assert!(fs.is_empty());
    }

    #[test]
    fn test_memory_fs_clear() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("file1.txt", "content1").unwrap();
        fs.add_file_string("file2.txt", "content2").unwrap();

        assert_eq!(fs.len(), 2);
        fs.clear();
        assert_eq!(fs.len(), 0);
    }

    #[test]
    fn test_memory_fs_merge() {
        let mut fs1 = MemoryFS::new();
        fs1.add_file_string("file1.txt", "content1").unwrap();

        let mut fs2 = MemoryFS::new();
        fs2.add_file_string("file2.txt", "content2").unwrap();
        fs2.add_file_string("file1.txt", "overwritten").unwrap(); // Should overwrite

        fs1.merge(&fs2);

        assert_eq!(fs1.len(), 2);
        assert!(fs1.exists("file1.txt"));
        assert!(fs1.exists("file2.txt"));

        let file1 = fs1.get_file("file1.txt").unwrap();
        assert_eq!(
            String::from_utf8(file1.content.clone()).unwrap(),
            "overwritten"
        );

        let file2 = fs1.get_file("file2.txt").unwrap();
        assert_eq!(
            String::from_utf8(file2.content.clone()).unwrap(),
            "content2"
        );
    }

    #[test]
    fn test_memory_fs_files_iterator() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("file1.txt", "content1").unwrap();
        fs.add_file_string("file2.txt", "content2").unwrap();

        let mut files: Vec<_> = fs.files().collect();
        files.sort_by(|a, b| a.0.cmp(b.0));

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, &PathBuf::from("file1.txt"));
        assert_eq!(files[1].0, &PathBuf::from("file2.txt"));

        assert_eq!(
            String::from_utf8(files[0].1.content.clone()).unwrap(),
            "content1"
        );
        assert_eq!(
            String::from_utf8(files[1].1.content.clone()).unwrap(),
            "content2"
        );
    }

    #[test]
    fn test_file_modified_time() {
        let file = File::new(vec![1, 2, 3]);
        // Just check that modified_time is set to some reasonable time
        // We can't test exact time without mocking, but we can ensure it's not ancient
        let now = SystemTime::now();
        let file_time = file.modified_time;

        // File time should be close to now (within a few seconds)
        let duration = now
            .duration_since(file_time)
            .unwrap_or_else(|_| file_time.duration_since(now).unwrap());
        assert!(duration.as_secs() < 10); // Should be very recent
    }
}
