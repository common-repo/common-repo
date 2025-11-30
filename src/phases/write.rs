//! Phase 6: Writing to Disk
//!
//! This is the final phase of the `common-repo` execution pipeline. Its main
//! responsibility is to write the final merged filesystem to the host filesystem.
//!
//! ## Process
//!
//! 1.  **Iterate Files**: The process iterates through all files in the MemoryFS.
//!
//! 2.  **Create Directories**: For each file, creates any necessary parent
//!     directories recursively.
//!
//! 3.  **Write Content**: Writes the file content to disk.
//!
//! 4.  **Set Permissions**: On Unix-like systems, sets file permissions to match
//!     the stored permissions (e.g., executable bit for scripts).
//!
//! This phase produces files on the host filesystem, completing the pull operation.

use std::fs;
use std::path::Path;

use crate::error::{Error, Result};
use crate::filesystem::MemoryFS;

/// Execute Phase 6: Write final filesystem to disk
///
/// Writes all files from the MemoryFS to the host filesystem at the specified output path.
/// Creates all necessary directories recursively and preserves file permissions where possible.
pub fn execute(final_fs: &MemoryFS, output_path: &Path) -> Result<()> {
    for (relative_path, file) in final_fs.files() {
        // Construct full output path
        let full_path = output_path.join(relative_path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::Filesystem {
                message: format!("Failed to create directory '{}': {}", parent.display(), e),
            })?;
        }

        // Write file content
        fs::write(&full_path, &file.content).map_err(|e| Error::Filesystem {
            message: format!("Failed to write file '{}': {}", full_path.display(), e),
        })?;

        // Set permissions on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(file.permissions);
            if let Err(e) = fs::set_permissions(&full_path, perms) {
                // Log warning but don't fail - permissions are best-effort
                // On some systems (e.g., certain mount points), setting permissions may fail
                return Err(Error::Filesystem {
                    message: format!(
                        "Failed to set permissions on '{}': {}",
                        full_path.display(),
                        e
                    ),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::execute;
    use crate::filesystem::{File, MemoryFS};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    #[test]
    fn test_phase6_write_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        let mut fs = MemoryFS::new();
        fs.add_file_string("test.txt", "Hello, world!").unwrap();

        execute(&fs, output_path).unwrap();

        let file_path = output_path.join("test.txt");
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[test]
    fn test_phase6_write_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        let mut fs = MemoryFS::new();
        fs.add_file_string("src/utils/helper.rs", "pub fn helper() {}")
            .unwrap();
        fs.add_file_string("src/main.rs", "fn main() {}").unwrap();
        fs.add_file_string("README.md", "# Project").unwrap();

        execute(&fs, output_path).unwrap();

        // Verify nested file exists
        let nested_path = output_path.join("src/utils/helper.rs");
        assert!(nested_path.exists());
        assert!(nested_path.parent().unwrap().exists()); // utils directory
        assert!(nested_path.parent().unwrap().parent().unwrap().exists()); // src directory

        // Verify other files exist
        assert!(output_path.join("src/main.rs").exists());
        assert!(output_path.join("README.md").exists());

        // Verify content
        let content = fs::read_to_string(&nested_path).unwrap();
        assert_eq!(content, "pub fn helper() {}");
    }

    #[test]
    fn test_phase6_write_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        let mut fs = MemoryFS::new();
        fs.add_file_string("file1.txt", "Content 1").unwrap();
        fs.add_file_string("file2.txt", "Content 2").unwrap();
        fs.add_file_string("file3.txt", "Content 3").unwrap();

        execute(&fs, output_path).unwrap();

        assert_eq!(
            fs::read_to_string(output_path.join("file1.txt")).unwrap(),
            "Content 1"
        );
        assert_eq!(
            fs::read_to_string(output_path.join("file2.txt")).unwrap(),
            "Content 2"
        );
        assert_eq!(
            fs::read_to_string(output_path.join("file3.txt")).unwrap(),
            "Content 3"
        );
    }

    #[test]
    fn test_phase6_write_binary_content() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        let mut memfs = MemoryFS::new();
        let binary_data = vec![0u8, 1u8, 2u8, 255u8, 128u8];
        memfs
            .add_file_content("binary.bin", binary_data.clone())
            .unwrap();

        execute(&memfs, output_path).unwrap();

        let file_path = output_path.join("binary.bin");
        assert!(file_path.exists());
        let content = fs::read(&file_path).unwrap();
        assert_eq!(content, binary_data);
    }

    #[test]
    #[cfg(unix)]
    fn test_phase6_preserve_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        let mut memfs = MemoryFS::new();
        let mut file = File::from_string("executable content");
        file.permissions = 0o755; // Executable permissions
        memfs.add_file("script.sh", file).unwrap();

        execute(&memfs, output_path).unwrap();

        let file_path = output_path.join("script.sh");
        assert!(file_path.exists());

        let metadata = fs::metadata(&file_path).unwrap();
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        // Check that executable bit is set (0o755 = 493 in decimal)
        // We check the last 3 octal digits (permissions)
        assert_eq!(mode & 0o777, 0o755);
    }

    #[test]
    fn test_phase6_empty_filesystem() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        let memfs = MemoryFS::new();

        // Should not error on empty filesystem
        execute(&memfs, output_path).unwrap();

        // Directory should exist but be empty
        assert!(output_path.exists());
        assert!(fs::read_dir(output_path).unwrap().next().is_none());
    }

    #[test]
    fn test_phase6_overwrite_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path();

        // Create an existing file
        let existing_path = output_path.join("existing.txt");
        fs::write(&existing_path, "old content").unwrap();

        let mut memfs = MemoryFS::new();
        memfs
            .add_file_string("existing.txt", "new content")
            .unwrap();

        execute(&memfs, output_path).unwrap();

        // File should be overwritten
        let content = fs::read_to_string(&existing_path).unwrap();
        assert_eq!(content, "new content");
    }
}
