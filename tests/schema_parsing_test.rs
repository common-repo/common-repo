//! Schema parsing tests using datatest-stable for test data discovery
//!
//! This test suite uses datatest-stable to automatically discover and test
//! schema YAML files in the testdata directory. Each YAML file is tested
//! to ensure it parses correctly.

use common_repo::config::{parse, Schema};
use std::path::Path;

/// Test that a schema YAML file parses successfully
///
/// This test is automatically run for each YAML file in the testdata directory.
/// It verifies that:
/// 1. The file can be read
/// 2. The YAML content is valid
/// 3. The schema parses into a valid Schema structure
/// 4. The parsed schema contains at least one operation
fn test_schema_parsing(path: &Path) -> datatest_stable::Result<()> {
    // Read the test file
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read test file {}: {}", path.display(), e))?;

    // Parse the schema
    let schema: Schema = parse(&content)
        .map_err(|e| format!("Failed to parse schema from {}: {}", path.display(), e))?;

    // Verify the schema is not empty
    assert!(
        !schema.is_empty(),
        "Schema in {} should contain at least one operation",
        path.display()
    );

    // Verify basic structure - check that operations are valid
    for (idx, operation) in schema.iter().enumerate() {
        match operation {
            common_repo::config::Operation::Repo { repo } => {
                assert!(
                    !repo.url.is_empty(),
                    "Repo operation {} in {} has empty URL",
                    idx,
                    path.display()
                );
                assert!(
                    !repo.r#ref.is_empty(),
                    "Repo operation {} in {} has empty ref",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::Include { include } => {
                assert!(
                    !include.patterns.is_empty(),
                    "Include operation {} in {} has no patterns",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::Exclude { exclude } => {
                assert!(
                    !exclude.patterns.is_empty(),
                    "Exclude operation {} in {} has no patterns",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::Template { template } => {
                assert!(
                    !template.patterns.is_empty(),
                    "Template operation {} in {} has no patterns",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::Rename { rename } => {
                assert!(
                    !rename.mappings.is_empty(),
                    "Rename operation {} in {} has no mappings",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::Tools { tools } => {
                assert!(
                    !tools.tools.is_empty(),
                    "Tools operation {} in {} has no tools",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::TemplateVars { template_vars } => {
                assert!(
                    !template_vars.vars.is_empty(),
                    "TemplateVars operation {} in {} has no vars",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::Yaml { yaml } => {
                assert!(
                    !yaml.source.is_empty(),
                    "Yaml operation {} in {} has empty source",
                    idx,
                    path.display()
                );
                assert!(
                    !yaml.dest.is_empty(),
                    "Yaml operation {} in {} has empty dest",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::Json { json } => {
                assert!(
                    !json.source.is_empty(),
                    "Json operation {} in {} has empty source",
                    idx,
                    path.display()
                );
                assert!(
                    !json.dest.is_empty(),
                    "Json operation {} in {} has empty dest",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::Toml { toml } => {
                assert!(
                    !toml.source.is_empty(),
                    "Toml operation {} in {} has empty source",
                    idx,
                    path.display()
                );
                assert!(
                    !toml.dest.is_empty(),
                    "Toml operation {} in {} has empty dest",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::Ini { ini } => {
                assert!(
                    !ini.source.is_empty(),
                    "Ini operation {} in {} has empty source",
                    idx,
                    path.display()
                );
                assert!(
                    !ini.dest.is_empty(),
                    "Ini operation {} in {} has empty dest",
                    idx,
                    path.display()
                );
            }
            common_repo::config::Operation::Markdown { markdown } => {
                assert!(
                    !markdown.source.is_empty(),
                    "Markdown operation {} in {} has empty source",
                    idx,
                    path.display()
                );
                assert!(
                    !markdown.dest.is_empty(),
                    "Markdown operation {} in {} has empty dest",
                    idx,
                    path.display()
                );
            }
        }
    }

    println!(
        "✓ Successfully parsed schema from {} ({} operations)",
        path.display(),
        schema.len()
    );
    Ok(())
}

// Register datatest harness to discover and run tests on all YAML files in testdata directory
datatest_stable::harness!(test_schema_parsing, "tests/testdata", r".*\.yaml$");
