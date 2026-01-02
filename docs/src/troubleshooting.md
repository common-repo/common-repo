# Troubleshooting

This guide covers common issues and their solutions when using common-repo.

## Git Authentication Errors

### Problem

```
Git clone error for https://github.com/org/private-repo@main: Authentication failed
  hint: Check SSH keys, git credentials, or personal access token
```

or

```
Git command failed for https://github.com/org/repo: ls-remote - Permission denied
```

### Causes

- Accessing a private repository without credentials
- Expired or invalid Git credentials
- SSH key not configured for the repository host

### Solutions

**For HTTPS URLs:**

1. Ensure you have access to the repository
2. Configure Git credential helper:
   ```bash
   git config --global credential.helper store
   ```
3. Or use a personal access token in the URL:
   ```yaml
   - repo:
       url: https://${GITHUB_TOKEN}@github.com/org/private-repo
       ref: main
   ```

**For SSH URLs:**

1. Ensure your SSH key is added to the Git host
2. Test SSH connectivity:
   ```bash
   ssh -T git@github.com
   ```
3. Use SSH URL format in configuration:
   ```yaml
   - repo:
       url: git@github.com:org/repo.git
       ref: main
   ```

**For CI/CD environments:**

- GitHub Actions: Use `${{ secrets.GITHUB_TOKEN }}` or a deploy key
- GitLab CI: Use `CI_JOB_TOKEN` or deploy tokens

---

## YAML Configuration Syntax Errors

### Problem

```
Configuration parsing error: Missing url field
  hint: Add 'url: https://github.com/...' to the repo block
```

or

```
YAML parsing error: expected ',' or ']' at line X column Y
```

### Causes

- Incorrect indentation (YAML requires consistent spacing)
- Missing or extra colons, brackets, or quotes
- Tabs instead of spaces

### Solutions

1. **Validate your YAML** before running:
   ```bash
   common-repo validate
   ```

2. **Check indentation** - YAML uses 2-space indentation by convention:
   ```yaml
   # Correct
   - repo:
       url: https://github.com/org/repo
       ref: main

   # Incorrect (inconsistent indentation)
   - repo:
     url: https://github.com/org/repo
       ref: main
   ```

3. **Quote special characters** in strings:
   ```yaml
   # Correct
   - include: ["*.yml", "**/*.yaml"]

   # May cause issues
   - include: [*.yml, **/*.yaml]
   ```

4. **Use a YAML linter** in your editor (e.g., YAML extension for VS Code)

---

## Circular Dependency Detected

### Problem

```
Cycle detected in repository dependencies: repo-a -> repo-b -> repo-a
```

### Cause

Two or more repositories reference each other, creating an infinite loop.

### Solution

1. **Review your dependency chain** - Draw out which repos inherit from which
2. **Break the cycle** by removing one of the circular references
3. **Use composition instead of inheritance** - Extract shared config into a third repository that both can inherit from without referencing each other

Example fix:
```
Before: A -> B -> A (cycle)
After:  A -> C, B -> C (shared base, no cycle)
```

---

## Network Connectivity Issues

### Problem

```
Network operation error: https://github.com/org/repo - Connection timeout
```

or

```
Git clone error: Could not resolve host
```

### Causes

- No internet connection
- Firewall blocking Git traffic
- GitHub/GitLab outage
- Proxy misconfiguration

### Solutions

1. **Check connectivity**:
   ```bash
   ping github.com
   git ls-remote https://github.com/common-repo/common-repo
   ```

2. **Configure proxy** if behind a corporate firewall:
   ```bash
   git config --global http.proxy http://proxy.example.com:8080
   ```

3. **Check service status**:
   - GitHub: https://www.githubstatus.com/
   - GitLab: https://status.gitlab.com/

4. **Retry** - Transient network issues often resolve themselves

---

## Cache Problems

### Problem

```
Cache operation error: Failed to read cached repository
```

or stale/corrupted cached data causing unexpected behavior.

### Cause

- Corrupted cache files
- Disk space issues
- Interrupted previous operation

### Solutions

1. **Clear the cache**:
   ```bash
   common-repo cache clear
   ```

2. **View cache status**:
   ```bash
   common-repo cache list
   ```

3. **Force fresh clone** by clearing cache before apply:
   ```bash
   common-repo cache clear && common-repo apply
   ```

The cache is stored in your system's cache directory (typically `~/.cache/common-repo` on Linux/macOS).

---

## Merge Conflicts

### Problem

```
Merge conflict warning: source.txt -> dest.txt: File already exists
```

### Cause

A file from an inherited repository conflicts with an existing file in your repository or another inherited repository.

### Solutions

1. **Use `exclude`** to skip conflicting files:
   ```yaml
   - repo:
       url: https://github.com/org/configs
       ref: main
       with:
         - exclude: ["conflicting-file.yml"]
   ```

2. **Use `rename`** to place the file elsewhere:
   ```yaml
   - repo:
       url: https://github.com/org/configs
       ref: main
       with:
         - rename:
             - "ci.yml": ".github/workflows/inherited-ci.yml"
   ```

3. **Check operation order** - Files from later operations overwrite earlier ones. Reorder your configuration if needed.

---

## Invalid Glob Patterns

### Problem

```
Glob pattern error: invalid pattern syntax
```

### Cause

Malformed glob pattern in `include`, `exclude`, or other operators.

### Solutions

1. **Use valid glob syntax**:
   ```yaml
   # Correct patterns
   - include:
       - "**/*.rs"      # All .rs files recursively
       - "src/**/*"     # All files under src/
       - "*.md"         # .md files in root only
       - ".*"           # Hidden files in root

   # Invalid patterns
   - include:
       - "**[.rs"       # Unclosed bracket
       - "src/***"      # Invalid triple asterisk
   ```

2. **Test patterns** with `common-repo ls` before applying:
   ```bash
   common-repo ls
   ```

---

## Git Reference Not Found

### Problem

```
Git clone error for https://github.com/org/repo@v2.0.0: reference not found
  hint: Verify the repository URL and ref (branch/tag) are correct
```

### Cause

The specified `ref` (branch, tag, or commit) does not exist in the repository.

### Solutions

1. **Verify the ref exists**:
   ```bash
   git ls-remote https://github.com/org/repo
   ```

2. **Check for typos** in branch/tag names

3. **Use the correct ref format**:
   ```yaml
   # Tag
   ref: v1.0.0

   # Branch
   ref: main

   # Commit SHA (full or abbreviated)
   ref: abc1234
   ```

4. **Check for updates** - the tag may have been deleted or renamed:
   ```bash
   common-repo check --updates
   ```

---

## Permission Denied Writing Files

### Problem

```
I/O error: Permission denied
```

### Cause

- Running in a read-only directory
- File is owned by another user
- File is locked by another process

### Solutions

1. **Check directory permissions**:
   ```bash
   ls -la .
   ```

2. **Ensure you own the files** or have write access

3. **Close editors** that may have files open

4. **On Windows**, check if files are marked read-only

---

## Getting Help

If your issue isn't covered here:

1. **Run with verbose output** for more details:
   ```bash
   common-repo apply --verbose
   # Or for maximum verbosity (trace level)
   common-repo apply --verbose --verbose
   ```

2. **Check existing issues**: https://github.com/common-repo/common-repo/issues

3. **Open a new issue** with:
   - Your `.common-repo.yaml` (sanitized of secrets)
   - Full error message
   - Output of `common-repo --version`
   - Your operating system
