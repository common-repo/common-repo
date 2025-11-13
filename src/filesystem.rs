//! # In-Memory Filesystem
//!
//! This module provides an in-memory filesystem implementation that is used
//! throughout the `common-repo` application to stage and manipulate files
//! before writing them to the host filesystem. This approach allows for
//! complex file operations, dry runs, and consistent behavior across different
//! operating systems.
//!
//! ## Key Components
//!
//! - **`MemoryFS`**: The main struct that represents the in-memory filesystem.
//!   It stores files in a `HashMap` where the keys are `PathBuf`s representing
//!   the file paths and the values are `File` structs.
//!
//! - **`File`**: A struct that represents a single file, containing its content
//!   as a `Vec<u8>` and associated metadata like permissions and modification
//!   time.
//!
//! ## Functionality
//!
//! The `MemoryFS` provides a comprehensive set of methods for file manipulation,
//! including:
//!
//! - Adding, retrieving, and removing files.
//! - Checking for the existence of a file.
//! - Listing all files or a subset of files that match a glob pattern.
//! - Renaming and copying files.
//! - Merging one filesystem into another.
//!
//! This in-memory representation is a crucial component of the multi-phase
//! pipeline, as it allows each phase to operate on a consistent and isolated
//! view of the filesystem.

use crate::error::{Error, Result};
use glob::Pattern;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Represents a file with content and metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct File {
    /// The raw byte content of the file.
    pub content: Vec<u8>,
    /// The file's permissions, represented in a Unix-like mode format.
    pub permissions: u32,
    /// The last modification time of the file.
    pub modified_time: SystemTime,
    /// A flag indicating whether this file should be processed as a template.
    pub is_template: bool,
}

#[allow(dead_code)]
impl File {
    /// Creates a new `File` with the given content.
    ///
    /// By default, the file is created with `0o644` permissions and the current
    /// system time as the modification time.
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::filesystem::File;
    ///
    /// let content = vec![72, 101, 108, 108, 111]; // "Hello"
    /// let file = File::new(content);
    ///
    /// assert_eq!(file.size(), 5);
    /// assert_eq!(file.permissions, 0o644);
    /// assert_eq!(file.content, vec![72, 101, 108, 108, 111]);
    /// ```
    pub fn new(content: Vec<u8>) -> Self {
        Self {
            content,
            permissions: 0o644, // Default to standard file permissions
            modified_time: SystemTime::now(),
            is_template: false,
        }
    }

    /// Creates a new `File` from a string slice.
    ///
    /// The string is converted to bytes using UTF-8 encoding.
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::filesystem::File;
    ///
    /// let file = File::from_string("Hello, world!");
    ///
    /// assert_eq!(file.content, b"Hello, world!");
    /// assert_eq!(file.size(), 13);
    /// ```
    pub fn from_string(content: &str) -> Self {
        Self::new(content.as_bytes().to_vec())
    }

    /// Returns the size of the file's content in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::filesystem::File;
    ///
    /// let empty = File::new(vec![]);
    /// assert_eq!(empty.size(), 0);
    ///
    /// let file = File::from_string("content");
    /// assert_eq!(file.size(), 7);
    /// ```
    pub fn size(&self) -> usize {
        self.content.len()
    }
}

/// An in-memory filesystem that stores files and their content.
///
/// This struct provides a virtual filesystem that can be manipulated without
/// affecting the host filesystem, which is essential for staging changes and
/// performing dry runs.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct MemoryFS {
    /// Files stored as path -> content mapping
    files: HashMap<PathBuf, File>,
}

#[allow(dead_code)]
impl MemoryFS {
    /// Creates a new, empty `MemoryFS`.
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::filesystem::MemoryFS;
    ///
    /// let fs = MemoryFS::new();
    /// assert!(fs.is_empty());
    /// assert_eq!(fs.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file to the filesystem.
    ///
    /// If a file already exists at the given path, it will be overwritten.
    pub fn add_file<P: AsRef<Path>>(&mut self, path: P, file: File) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        self.files.insert(path, file);
        Ok(())
    }

    /// A convenience method to add a file with raw byte content.
    pub fn add_file_content<P: AsRef<Path>>(&mut self, path: P, content: Vec<u8>) -> Result<()> {
        self.add_file(path, File::new(content))
    }

    /// A convenience method to add a file with string content.
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::filesystem::MemoryFS;
    ///
    /// let mut fs = MemoryFS::new();
    /// fs.add_file_string("README.md", "# My Project").unwrap();
    ///
    /// assert!(fs.exists("README.md"));
    /// assert_eq!(fs.len(), 1);
    ///
    /// let file = fs.get_file("README.md").unwrap();
    /// assert_eq!(file.content, b"# My Project");
    /// ```
    pub fn add_file_string<P: AsRef<Path>>(&mut self, path: P, content: &str) -> Result<()> {
        self.add_file(path, File::from_string(content))
    }

    /// Retrieves a reference to a file in the filesystem.
    pub fn get_file<P: AsRef<Path>>(&self, path: P) -> Option<&File> {
        self.files.get(path.as_ref())
    }

    /// Retrieves a mutable reference to a file in the filesystem.
    pub fn get_file_mut<P: AsRef<Path>>(&mut self, path: P) -> Option<&mut File> {
        self.files.get_mut(path.as_ref())
    }

    /// Removes a file from the filesystem, returning it if it existed.
    pub fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Option<File>> {
        Ok(self.files.remove(path.as_ref()))
    }

    /// Checks if a file exists at the given path.
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::filesystem::MemoryFS;
    ///
    /// let mut fs = MemoryFS::new();
    ///
    /// assert!(!fs.exists("file.txt"));
    ///
    /// fs.add_file_string("file.txt", "content").unwrap();
    /// assert!(fs.exists("file.txt"));
    /// ```
    pub fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.files.contains_key(path.as_ref())
    }

    /// Returns a list of all file paths in the filesystem.
    pub fn list_files(&self) -> Vec<PathBuf> {
        self.files.keys().cloned().collect()
    }

    /// Returns a list of file paths that match the given glob pattern.
    pub fn list_files_glob(&self, pattern: &str) -> Result<Vec<PathBuf>> {
        let pattern = Pattern::new(pattern).map_err(Error::Glob)?;
        let mut matches = Vec::new();

        for path in self.files.keys() {
            if let Some(path_str) = path.to_str() {
                if pattern.matches(path_str) {
                    matches.push(path.clone());
                }
            }
        }

        Ok(matches)
    }

    /// Renames a file from one path to another.
    ///
    /// Returns an error if the `from` path does not exist. If the `to` path
    /// already exists, it will be overwritten.
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::filesystem::MemoryFS;
    ///
    /// let mut fs = MemoryFS::new();
    /// fs.add_file_string("old_name.txt", "content").unwrap();
    ///
    /// fs.rename_file("old_name.txt", "new_name.txt").unwrap();
    ///
    /// assert!(!fs.exists("old_name.txt"));
    /// assert!(fs.exists("new_name.txt"));
    /// ```
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

    /// Copies a file from one path to another.
    ///
    /// Returns an error if the `from` path does not exist. If the `to` path
    /// already exists, it will be overwritten.
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

    /// Returns the number of files in the filesystem.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns `true` if the filesystem contains no files.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Removes all files from the filesystem.
    pub fn clear(&mut self) {
        self.files.clear();
    }

    /// Merges another `MemoryFS` into this one.
    ///
    /// If a file exists in both filesystems, the one from `other` will
    /// overwrite the one in `self` (last-write-wins).
    ///
    /// # Examples
    ///
    /// ```
    /// use common_repo::filesystem::MemoryFS;
    ///
    /// let mut fs1 = MemoryFS::new();
    /// fs1.add_file_string("file1.txt", "content1").unwrap();
    ///
    /// let mut fs2 = MemoryFS::new();
    /// fs2.add_file_string("file2.txt", "content2").unwrap();
    /// fs2.add_file_string("file1.txt", "overwritten").unwrap(); // Should overwrite
    ///
    /// fs1.merge(&fs2);
    ///
    /// assert_eq!(fs1.len(), 2);
    /// assert!(fs1.exists("file1.txt"));
    /// assert!(fs1.exists("file2.txt"));
    /// ```
    pub fn merge(&mut self, other: &MemoryFS) {
        for (path, file) in &other.files {
            self.files.insert(path.clone(), file.clone());
        }
    }

    /// Returns an iterator over the `(path, file)` pairs in the filesystem.
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

    #[test]
    fn test_list_files_glob_recursive_pattern() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("src/main.rs", "code").unwrap();
        fs.add_file_string("src/lib.rs", "code").unwrap();
        fs.add_file_string("src/utils/helper.rs", "code").unwrap();
        fs.add_file_string("tests/test.rs", "test").unwrap();
        fs.add_file_string("README.md", "readme").unwrap();

        // Test recursive pattern
        let matches = fs.list_files_glob("**/*.rs").unwrap();
        assert_eq!(matches.len(), 4);
        assert!(matches.contains(&PathBuf::from("src/main.rs")));
        assert!(matches.contains(&PathBuf::from("src/lib.rs")));
        assert!(matches.contains(&PathBuf::from("src/utils/helper.rs")));
        assert!(matches.contains(&PathBuf::from("tests/test.rs")));
    }

    #[test]
    fn test_list_files_glob_multiple_wildcards() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("src/test_main.rs", "code").unwrap();
        fs.add_file_string("src/test_utils.rs", "code").unwrap();
        fs.add_file_string("src/main.rs", "code").unwrap();
        fs.add_file_string("tests/test_main.rs", "test").unwrap();

        // Test pattern with multiple wildcards
        let matches = fs.list_files_glob("src/**/test*.rs").unwrap();
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&PathBuf::from("src/test_main.rs")));
        assert!(matches.contains(&PathBuf::from("src/test_utils.rs")));
    }

    #[test]
    fn test_list_files_glob_character_classes() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("a.txt", "content").unwrap();
        fs.add_file_string("b.txt", "content").unwrap();
        fs.add_file_string("c.txt", "content").unwrap();
        fs.add_file_string("1.txt", "content").unwrap();
        fs.add_file_string("A.txt", "content").unwrap();

        // Test character class pattern
        let matches = fs.list_files_glob("[a-z].txt").unwrap();
        assert_eq!(matches.len(), 3);
        assert!(matches.contains(&PathBuf::from("a.txt")));
        assert!(matches.contains(&PathBuf::from("b.txt")));
        assert!(matches.contains(&PathBuf::from("c.txt")));
    }

    #[test]
    fn test_list_files_glob_empty_filesystem() {
        let fs = MemoryFS::new();

        let matches = fs.list_files_glob("*.txt").unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_list_files_glob_no_matches() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("test.txt", "content").unwrap();
        fs.add_file_string("test.rs", "code").unwrap();

        let matches = fs.list_files_glob("*.js").unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_list_files_glob_invalid_pattern() {
        let fs = MemoryFS::new();

        // Test invalid glob pattern
        let result = fs.list_files_glob("[invalid");
        assert!(result.is_err());
        if let Err(crate::error::Error::Glob(_)) = result {
            // Expected glob error
        } else {
            panic!("Expected Glob error for invalid pattern");
        }
    }

    #[test]
    fn test_add_file_with_file_object() {
        let mut fs = MemoryFS::new();
        let mut file = File::new(b"content".to_vec());
        file.permissions = 0o755;
        file.modified_time = std::time::SystemTime::UNIX_EPOCH;

        fs.add_file("test.txt", file.clone()).unwrap();

        let retrieved = fs.get_file("test.txt").unwrap();
        assert_eq!(retrieved.content, b"content");
        assert_eq!(retrieved.permissions, 0o755);
        assert_eq!(retrieved.modified_time, std::time::SystemTime::UNIX_EPOCH);
    }

    #[test]
    fn test_add_file_overwrite_existing() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("test.txt", "old content").unwrap();

        let mut new_file = File::new(b"new content".to_vec());
        new_file.permissions = 0o644;
        fs.add_file("test.txt", new_file).unwrap();

        let file = fs.get_file("test.txt").unwrap();
        assert_eq!(
            String::from_utf8(file.content.clone()).unwrap(),
            "new content"
        );
        assert_eq!(file.permissions, 0o644);
    }

    #[test]
    fn test_add_file_to_nested_path() {
        let mut fs = MemoryFS::new();
        let file = File::new(b"nested content".to_vec());

        fs.add_file("deep/nested/path/file.txt", file).unwrap();

        assert!(fs.exists("deep/nested/path/file.txt"));
        let retrieved = fs.get_file("deep/nested/path/file.txt").unwrap();
        assert_eq!(
            String::from_utf8(retrieved.content.clone()).unwrap(),
            "nested content"
        );
    }

    #[test]
    fn test_copy_file_overwrite_behavior() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("source.txt", "source content").unwrap();
        fs.add_file_string("dest.txt", "old dest content").unwrap();

        fs.copy_file("source.txt", "dest.txt").unwrap();

        // dest.txt should be overwritten with source content
        assert!(fs.exists("source.txt"));
        assert!(fs.exists("dest.txt"));
        let source = fs.get_file("source.txt").unwrap();
        let dest = fs.get_file("dest.txt").unwrap();
        assert_eq!(source.content, dest.content);
        assert_eq!(
            String::from_utf8(dest.content.clone()).unwrap(),
            "source content"
        );
    }

    #[test]
    fn test_copy_file_to_nested_path() {
        let mut fs = MemoryFS::new();
        fs.add_file_string("source.txt", "content").unwrap();

        fs.copy_file("source.txt", "nested/path/copy.txt").unwrap();

        assert!(fs.exists("source.txt"));
        assert!(fs.exists("nested/path/copy.txt"));
        let source = fs.get_file("source.txt").unwrap();
        let copy = fs.get_file("nested/path/copy.txt").unwrap();
        assert_eq!(source.content, copy.content);
    }
}
