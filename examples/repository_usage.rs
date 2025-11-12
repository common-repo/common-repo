//! Example demonstrating how the RepositoryManager will be used
//!
//! Run with: cargo run --example repository_usage

use common_repo::repository::RepositoryManager;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // In the real application, this would come from configuration
    let cache_root = PathBuf::from("/tmp/commonrepo-cache");

    // Create the repository manager
    let manager = RepositoryManager::new(cache_root);

    // Example 1: Check if a repository is cached
    let is_cached = manager.is_cached("https://github.com/rust-lang/mdBook", "v0.4.35");
    println!("Repository cached: {}", is_cached);

    // Example 2: Fetch a repository (will clone if not cached)
    println!("Fetching repository...");
    match manager.fetch_repository("https://github.com/rust-lang/mdBook", "v0.4.35") {
        Ok(fs) => {
            println!("Repository loaded into memory!");
            println!("Files in repository: {}", fs.len());

            // List some files
            let files = fs.list_files();
            for file in files.iter().take(5) {
                println!("  - {}", file.display());
            }
        }
        Err(e) => {
            println!("Error fetching repository: {}", e);
        }
    }

    // Example 3: List available tags
    println!("\nListing tags...");
    match manager.list_repository_tags("https://github.com/rust-lang/mdBook") {
        Ok(tags) => {
            println!("Found {} tags", tags.len());
            for tag in tags.iter().take(5) {
                println!("  - {}", tag);
            }
        }
        Err(e) => {
            println!("Error listing tags: {}", e);
        }
    }

    Ok(())
}

// Note: To make this example work, we'd need to add:
// 1. A lib.rs file that exports the repository module
// 2. Update Cargo.toml with [[example]] section
// For now, this serves as documentation of the intended usage
