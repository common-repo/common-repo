# Coordination Service Design

## Architecture Overview

The coordination system lives in a **separate repository** with three components:

```
claude-coord/                          # Separate repo
├── crates/
│   ├── claude-coord/                  # Client library (published to crates.io)
│   │   └── src/lib.rs                 # list_claims(), claim_task(), etc.
│   ├── claude-coord-cli/              # CLI binary (PRIMARY interface)
│   │   └── src/main.rs                # Standalone tool, works with any language
│   └── claude-coord-service/          # Shuttle service
│       └── src/main.rs                # HTTP API + background jobs
└── Cargo.toml                         # Workspace
```

**Language-agnostic design**: The CLI binary is the primary interface. Any project (Python, Node, Go, Rust, etc.) can use it directly from SessionStart hooks. The Rust crate is available for projects that want tighter integration.

```
┌──────────────────────────────────────────────────────────────┐
│  Coordination Service (Shuttle.rs)                           │
│  https://claude-coord.shuttleapp.rs                          │
├──────────────────────────────────────────────────────────────┤
│  POST   /claim          Atomically claim a task (CAS)        │
│  POST   /complete       Mark task complete (primary)         │
│  GET    /claims         List active + recently completed     │
│  DELETE /claim          Manually expire a claim (admin)      │
└──────────────────────────────────────────────────────────────┘
              ▲                    ▲                    ▲
              │ HTTPS              │ HTTPS              │ HTTPS
              │                    │                    │
    ┌─────────┴───────┐  ┌────────┴────────┐  ┌───────┴─────────┐
    │   Session A     │  │   Session B     │  │   Session C     │
    │ claude/feat-XXX │  │ claude/fix-YYY  │  │ claude/ref-ZZZ  │
    │ Task: task-001  │  │ Task: task-002  │  │ Task: task-003  │
    │                 │  │                 │  │                 │
    │ Uses CLI:       │  │ Uses CLI:       │  │ Uses CLI:       │
    │ claude-coord    │  │ claude-coord    │  │ claude-coord    │
    └─────────────────┘  └─────────────────┘  └─────────────────┘

                              │
                              ▼ (hourly background job)
                    ┌─────────────────────┐
                    │  GitHub API (auth)  │
                    │  Check branch exist │
                    └─────────────────────┘
```

## CLI Distribution (Primary - Any Language)

The `claude-coord` CLI is the primary interface, usable from any project regardless of language.

**Installation options:**

```bash
# Via cargo (if Rust is available)
cargo install claude-coord-cli

# Via pre-built binary (GitHub releases)
curl -sSL https://github.com/.../releases/latest/download/claude-coord-$(uname -s)-$(uname -m) -o /usr/local/bin/claude-coord
chmod +x /usr/local/bin/claude-coord

# Via install script
curl -sSL https://raw.githubusercontent.com/.../install.sh | bash
```

**CLI usage:**

```bash
# List claims for a repo
claude-coord claims --repo owner/repo

# Claim a task (reads session ID and branch from environment/git)
claude-coord claim --repo owner/repo --task implement-feature-x

# With explicit values
claude-coord claim \
  --repo owner/repo \
  --task implement-feature-x \
  --session-id "$CLAUDE_CODE_SESSION_ID" \
  --branch "$(git branch --show-current)"

# Mark task complete (primary completion mechanism)
claude-coord complete --task implement-feature-x

# Manually expire a claim (admin/cleanup)
claude-coord expire --repo owner/repo --task implement-feature-x
claude-coord expire --repo owner/repo --session-id <uuid>

# Show help
claude-coord --help
```

**Exit codes:**
- `0` - Success
- `1` - Claim conflict (task already claimed by another session)
- `2` - Service unavailable (coordination halted, user must decide)
- `3` - Session already has an active claim (must merge current branch first)
- `4` - Stale token (repo state changed, need to rebase)
- `5` - Repository not in whitelist (not configured for coordination)

**Configuration**: The CLI reads from environment variables or a `.claude-coord.toml` file:

```toml
# .claude-coord.toml (in project root)
repo = "owner/repo"
service_url = "https://claude-coord.shuttleapp.rs"  # optional, has default
plan_path = "context"                               # optional, path prefix for plan files (default: "context")
```

With config file present, commands simplify to:
```bash
claude-coord claims
claude-coord claim --task implement-feature-x
```

## Crate Distribution (Optional - Rust Projects)

For Rust projects that want compile-time integration instead of shelling out to the CLI, the client crate is published to crates.io. See the crate's documentation for API details.

```toml
[dependencies]
claude-coord = "0.1"
```

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Claim expiry | 10 days | Long gaps between work sessions are common |
| Branch monitoring | Hourly | Detect merged/deleted branches to complete claims |
| Scope | Multi-repo with whitelist | Repos validated against whitelist; fast-fail for unknown repos |
| Race handling | Compare-and-swap | First claim wins, enforced in-process |
| Task discovery | Client-side | Service tracks claims; CLI/xtask reads plan files locally |
| One claim per session | Enforced | Session claims one task, branch merge completes it |
| No claim updates | Enforced | Session cannot change task; start new session instead |
| Private repos | Content-hash auth | Token derived from repo content proves access |
| Service downtime | Halt | User must decide whether to proceed without coordination |
| Local + Web | Both supported | CLI works in any environment |

## Authentication via Content Hash

Private repository support uses a content-hash token that proves the client has repo access without transmitting credentials.

### How It Works

```
┌─────────────────────────────────┐         ┌─────────────────────────────────┐
│  Client (Session)               │         │  Service (has repo credentials) │
├─────────────────────────────────┤         ├─────────────────────────────────┤
│ 1. git checkout main            │         │                                 │
│ 2. Get HEAD sha                 │         │                                 │
│ 3. For each plan file, get      │         │                                 │
│    last-modifying commit sha    │         │                                 │
│ 4. token = sha256(              │         │                                 │
│      head_sha +                 │         │                                 │
│      sorted_file_shas           │         │                                 │
│    )                            │         │                                 │
│ 5. Send request with token ─────┼────────►│ 6. Fetch same commit            │
│                                 │         │ 7. Compute same hash            │
│                                 │         │ 8. Compare tokens               │
│                                 │◄────────┼─ 9. Accept/reject               │
└─────────────────────────────────┘         └─────────────────────────────────┘
```

### Token Formula

```
token = sha256(head_sha + file1_sha + file2_sha + ...)
```

Where:
- `head_sha` = HEAD commit of main branch at session start
- `fileN_sha` = commit SHA that last modified each `{plan_path}/*.json` file, sorted by path

**Computing per-file SHAs**:
```bash
git log -1 --format=%H -- context/current-task.json
# Returns: abc123... (the commit that last touched this file)
```

This is efficient because:
- Only requires git metadata, not file content reads
- Per-file commit SHAs already encode file content (content-addressable storage)
- Significantly less data to hash than full file contents

The `plan_path` is configurable via `.claude-coord.toml` to support different project structures.

### What This Proves

| Property | How |
|----------|-----|
| Client has repo access | Per-file commit SHAs require git history access |
| Client sees current state | Hash includes HEAD SHA |
| No credential leakage | Token is derived from git metadata, not secrets |

### Shared Tokens Are Intentional

Multiple sessions starting from the same commit will compute the same token. This is correct:
- Token proves **authentication** (can I access this repo?)
- Session ID provides **identification** (who am I?)

### Stale Token Handling

When main branch is updated, tokens from previous commits become invalid:

```
Client: POST /claim with token (from commit abc123)
Service: Fetches main, now at commit def456
Service: Computes hash → doesn't match token
Service: Returns 401 with "stale_token" error

CLI output:
ERROR: Repository state has changed since session start.

Your local state is at commit: abc123
Current main branch is at:     def456

ACTION REQUIRED: Run 'git pull --rebase origin main' to sync with the
latest changes, then run 'claude-coord claims' to refresh and try again.
```

### Service Configuration

The service supports multiple repositories via a whitelist. Requests for repos not in the whitelist are rejected immediately (exit code 5), preventing resource waste on bogus requests.

```toml
# Service configuration (not in client repo)

[github]
# GitHub App (recommended) - works for all repos the app is installed on
app_id = 12345
private_key_path = "/secrets/github-app.pem"

# Or: Personal Access Token (simpler, for personal repos)
# token = "ghp_xxxxxxxxxxxx"

# Whitelist of allowed repositories
[[repos]]
name = "owner/repo-one"
plan_path = "context"

[[repos]]
name = "owner/repo-two"
plan_path = ".tasks"

[[repos]]
name = "org/private-project"
plan_path = "plans"
```

The service uses these credentials to:
1. Validate repo is in whitelist (fast-fail if not)
2. Fetch per-file commit SHAs for token validation
3. Check branch existence for claim lifecycle

## API Design

### Claim a Task (Atomic CAS)

```
POST /claim
Content-Type: application/json

{
  "repo": "common-repo/common-repo",
  "token": "sha256:abc123def456...",
  "commit_sha": "abc123",
  "session_id": "8db090b6-ed48-441b-a6ca-043110bc834b",
  "task_id": "implement-feature-x",
  "branch": "claude/implement-feature-x-AbCdE",
  "plan_file": "context/current-task.json"
}
```

**Responses:**

- `200 OK` - Claim successful
  ```json
  { "status": "claimed", "task_id": "implement-feature-x" }
  ```

- `409 Conflict` - Task already claimed by another session
  ```json
  {
    "status": "conflict",
    "task_id": "implement-feature-x",
    "claimed_by": {
      "session_id": "other-session-uuid",
      "branch": "claude/implement-feature-x-OtHeR",
      "claimed_at": "2026-01-05T20:00:00Z"
    }
  }
  ```

**Semantics**: Compare-and-swap. If `task_id` is not currently claimed, the claim succeeds. If already claimed by another session, returns 409. If claimed by the same session, updates the claim (idempotent).

### Complete a Task (Primary Completion)

```
POST /complete
Content-Type: application/json

{
  "repo": "owner/repo",
  "token": "sha256:abc123def456...",
  "commit_sha": "abc123",
  "session_id": "8db090b6-ed48-441b-a6ca-043110bc834b",
  "task_id": "implement-feature-x"
}
```

**Responses:**

- `200 OK` - Task marked complete
  ```json
  { "status": "completed", "task_id": "implement-feature-x" }
  ```

- `404 Not Found` - No active claim for this task/session
  ```json
  { "status": "not_found", "message": "No active claim for task" }
  ```

- `403 Forbidden` - Session doesn't own this claim
  ```json
  { "status": "forbidden", "message": "Task claimed by different session" }
  ```

**Semantics**: Only the session that owns a claim can complete it. The claim moves to `recently_completed` with `completion_reason: "client_completed"`.

### List Claims

```
GET /claims?repo=common-repo/common-repo&token=sha256:abc123...&commit_sha=abc123

Response:
{
  "active": [
    {
      "session_id": "8db090b6-ed48-441b-a6ca-043110bc834b",
      "task_id": "implement-feature-x",
      "branch": "claude/implement-feature-x-AbCdE",
      "plan_file": "context/current-task.json",
      "claimed_at": "2026-01-05T23:00:00Z"
    }
  ],
  "recently_completed": [
    {
      "session_id": "previous-session-uuid",
      "task_id": "fix-bug-y",
      "branch": "claude/fix-bug-y-PrEvS",
      "completed_at": "2026-01-05T22:30:00Z",
      "completion_reason": "branch_removed"
    }
  ]
}
```

**recently_completed**: Claims that were completed in the last 24 hours. Helps in-flight sessions coordinate even with stale local repo state.

### Expire a Claim (Manual Cleanup)

```
DELETE /claim
Content-Type: application/json

{
  "repo": "owner/repo",
  "token": "sha256:abc123def456...",
  "commit_sha": "abc123",
  "task_id": "implement-feature-x"
}
```

Or by session:
```json
{
  "repo": "owner/repo",
  "token": "sha256:abc123def456...",
  "commit_sha": "abc123",
  "session_id": "8db090b6-ed48-441b-a6ca-043110bc834b"
}
```

**Response:** `200 OK` - Claim expired and moved to recently_completed

**Use case**: Manual cleanup of stuck or erroneous claims. Intended for human operators, not automated use.

## Data Model

```rust
#[derive(Clone, Serialize, Deserialize)]
struct Claim {
    session_id: String,     // UUID from CLAUDE_CODE_SESSION_ID
    task_id: String,        // Task identifier from plan JSON
    branch: String,         // Git branch name
    plan_file: String,      // Path to the plan file being worked on
    claimed_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
struct CompletedClaim {
    session_id: String,
    task_id: String,
    branch: String,
    completed_at: DateTime<Utc>,
    completion_reason: CompletionReason,
}

#[derive(Clone, Serialize, Deserialize)]
enum CompletionReason {
    ClientCompleted, // Session explicitly marked task complete (primary)
    BranchRemoved,   // Branch merged or deleted (fallback detection)
    TimedOut,        // 10-day expiry - session abandoned
}
```

**Storage**: Persistent storage via Shuttle Persist or Turso (SQLite). In-memory caching for fast reads.

**Indexes**:
- Active claims by `(repo, task_id)` - for CAS lookup
- Active claims by `(repo, session_id)` - for session's current claim
- Recently completed by `repo` - for coordination

## Claim Lifecycle

There are only two states: **ACTIVE** and **RECENTLY_COMPLETED**. Timeout-expired claims are deleted, not preserved.

```
                    POST /claim
                         │
                         ▼
              ┌──────────────────────┐
              │   Task unclaimed?    │
              └──────────────────────┘
                    │           │
                   Yes          No
                    │           │
                    ▼           ▼
              ┌──────────┐  ┌──────────────┐
              │  ACTIVE  │  │ 409 Conflict │
              └──────────┘  └──────────────┘
                    │
        ┌───────────┴───────────┐
        │                       │
        ▼                       ▼
   POST /complete        (hourly background)
        │               ┌─────────────────┐
        │               │ Branch exists?  │
        │               └─────────────────┘
        │                    │         │
        │                   Yes        No
        │                    │         │
        ▼                    ▼         ▼
   ┌─────────────────────┐  (stay   ┌─────────────────────┐
   │ RECENTLY_COMPLETED  │  active) │ RECENTLY_COMPLETED  │
   │ (client_completed)  │          │ (branch_removed)    │
   └─────────────────────┘          └─────────────────────┘
              │                              │
              └──────────────┬───────────────┘
                             │  (after 24h)
                             ▼
                       ┌──────────┐
                       │ DELETED  │
                       └──────────┘

         ┌──────────┐
         │  ACTIVE  │  (after 10 days without completion)
         └──────────┘
                │
                ▼
         ┌──────────┐
         │ DELETED  │  (timeout expired claims are not preserved)
         └──────────┘
```

**Completion priority**:
1. **Client explicit** (`POST /complete`) - Primary, most reliable
2. **Branch removal** (hourly poll) - Fallback for sessions that don't call complete
3. **Timeout** (10 days) - Safety net for abandoned sessions

## Background Branch Validation (Fallback)

This is a **fallback mechanism** for sessions that don't explicitly call `complete`. The primary completion path is client-side via `POST /complete`.

**Frequency**: Once per hour (configurable, could be less frequent)

**Process**:
1. Fetch all active claims
2. For each claim, check if branch exists via GitHub API (using configured credentials):
   ```
   GET https://api.github.com/repos/{owner}/{repo}/branches/{branch}
   Authorization: Bearer <token>
   ```
3. If branch doesn't exist (404), move claim to `recently_completed` with `BranchRemoved`
4. If branch exists, no action

**Rate Limiting**: With authenticated requests, GitHub API allows 5000 requests/hour. More than sufficient for hourly checks.

**Blobless clone optimization**: The service can use `git clone --filter=blob:none` to clone only git metadata (commits, trees) without file contents. This significantly reduces storage and bandwidth since the service only needs commit SHAs, not file contents.

## Task/Plan JSON Format

The coordination service tracks claims by `task_id`, which corresponds to the task IDs in plan JSON files. Projects using coordination should follow this plan structure (from CLAUDE.md conventions):

```json
{
  "plan_name": "Feature or Project Name",
  "last_updated": "YYYY-MM-DD",
  "tasks": [
    {
      "id": "task-id-kebab-case",
      "name": "Human readable task name",
      "status": "pending",
      "priority": 1,
      "blocked_by": null,
      "steps": [
        "Concrete step 1 with specific action",
        "Concrete step 2 with verification command"
      ],
      "acceptance_criteria": [
        "File exists at expected path",
        "Tests pass",
        "No lint warnings"
      ]
    }
  ]
}
```

**Key fields for coordination:**
- `id` - The `task_id` used in claims (kebab-case format)
- `status` - `"pending"`, `"in_progress"`, or `"complete"`
- `blocked_by` - `null` or another task's ID

**Current task pointer**: Projects typically use `context/current-task.json` or similar to indicate which plan file is active.

## Integration with Claude Code (Web and Local)

The coordination system works in both Claude Code Web and local CLI environments.

### Environment Setup

1. Add `claude-coord.shuttleapp.rs` to the Web environment's network allowlist
2. Create `.claude-coord.toml` config file in project root
3. The CLI will be auto-installed by the SessionStart hook if not present

### SessionStart Hook (Any Language)

The SessionStart hook installs the CLI if needed, then fetches claims. Works for Python, Node, Go, Rust, or any other project, in both Web and local environments.

```bash
#!/bin/bash
# .claude/hooks/session-start-coord.sh

# Install CLI if not present (works in both Web and local)
if ! command -v claude-coord &> /dev/null; then
  echo "Installing claude-coord CLI..."
  curl -sSL https://raw.githubusercontent.com/user/claude-coord/main/install.sh | bash
fi

# CLI reads config from .claude-coord.toml
claude-coord claims
```

The CLI outputs formatted, LLM-friendly text for Claude's context:

```
=== Coordination State ===
Active claims:
  - implement-feature-x on claude/implement-feature-x-AbCdE
  - fix-bug-y on claude/fix-bug-y-XyZaB

Recently completed:
  - refactor-config (branch_removed)

To claim a task: claude-coord claim --task <task-id>
```

**Claiming a task** (Claude runs this after choosing a task):
```bash
claude-coord claim --task implement-feature-x
```

### LLM-Friendly Error Output

All CLI errors include actionable guidance for the agent:

**Conflict (exit code 1):**
```
ERROR: Task 'implement-feature-x' is already claimed.

Claimed by:
  Session: 8db090b6-...
  Branch: claude/implement-feature-x-AbCdE
  Claimed at: 2026-01-05T20:00:00Z

ACTION REQUIRED: Run 'claude-coord claims' to refresh the list of available tasks, then claim a different task that is not already claimed.
```

**Session already has claim (exit code 3):**
```
ERROR: This session already has an active claim.

Current claim:
  Task: fix-bug-y
  Branch: claude/fix-bug-y-XyZaB

ACTION REQUIRED: Complete your current task and merge the branch. Once the branch is merged or deleted, your claim will be automatically released and you can start a new session to work on a different task.
```

**Service unavailable (exit code 2):**
```
ERROR: Coordination service is unavailable.

Service URL: https://claude-coord.shuttleapp.rs
Status: Connection refused / Timeout

ACTION REQUIRED: The coordination service is down. Please ask the user whether to:
1. Wait and retry later
2. Proceed without coordination (risk of duplicate work)
```

**Stale token (exit code 4):**
```
ERROR: Repository state has changed since session start.

Your local state is at commit: abc123
Current main branch is at:     def456

ACTION REQUIRED: Run 'git pull --rebase origin main' to sync with the
latest changes, then run 'claude-coord claims' to refresh and try again.
```

**Repository not in whitelist (exit code 5):**
```
ERROR: Repository 'owner/unknown-repo' is not configured for coordination.

This coordination service only handles whitelisted repositories.
Contact the service administrator to add this repository.

ACTION REQUIRED: Verify you have the correct repository name in your
.claude-coord.toml file, or proceed without coordination for this project.
```

### Language-Specific Wrappers (Optional)

Projects can optionally wrap the CLI in their build system for convenience:

**Rust (xtask)**:
```bash
cargo xtask coord claims
cargo xtask coord claim implement-feature-x
```

**Python (invoke/make)**:
```bash
invoke coord-claims
invoke coord-claim --task implement-feature-x
```

**Node (npm scripts)**:
```bash
npm run coord:claims
npm run coord:claim -- implement-feature-x
```

**Make**:
```bash
make coord-claims
make coord-claim TASK=implement-feature-x
```

These wrappers just call the `claude-coord` CLI underneath.

## Hosting

**Recommended**: [Shuttle.rs](https://shuttle.rs) with axum

- Rust-native platform
- Free tier: 3 projects
- Simple deployment: `cargo shuttle deploy`
- Built-in persistence (Shuttle Persist or Turso)
- Background tasks supported via `shuttle-runtime`
- axum is the modern standard for Rust web services

**Service dependencies**:
```toml
# claude-coord-service/Cargo.toml
[dependencies]
axum = "0.7"
shuttle-axum = "0.49"
shuttle-runtime = "0.49"
shuttle-persist = "0.49"  # Simple file-based persistence
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
```

**Deployment**:
```bash
cargo install cargo-shuttle
cargo shuttle login
cargo shuttle init --template axum
# ... implement service ...
cargo shuttle deploy
```

## Security Considerations

1. **Content-hash authentication** - Token derived from repo content proves access
2. **Private repo support** - Service configured with GitHub credentials per-repo
3. **Session IDs are unguessable** - UUIDs provide identification, not authentication
4. **Rate limiting** - Consider adding rate limits to prevent abuse
5. **No sensitive data** - Only coordination metadata, no code or secrets
6. **Token expiry** - Stale tokens (from old commits) are rejected, forcing sync

## Documentation Requirements

The `claude-coord` documentation must be **LLM-friendly** to enable agents to easily set up coordination in any project. This means:

1. **Complete, copy-paste examples** - Full working commands, not fragments
2. **No assumed context** - Each example self-contained
3. **Step-by-step integration guide** - Numbered steps an agent can follow
4. **Environment variable reference** - All expected env vars documented
5. **Error handling patterns** - Show how to handle each error case
6. **Testing instructions** - How to verify the integration works

### Quick Start Templates (Language-Agnostic)

The repo should provide ready-to-use files that any project can copy:

```
claude-coord/
├── install.sh                     # CLI installer script
├── templates/
│   ├── session-start-hook.sh      # Drop-in bash script
│   ├── settings.json.example      # Example .claude/settings.json snippet
│   └── claude-coord.toml.example  # Example config file
```

**Drop-in config file** (`.claude-coord.toml`):
```toml
# .claude-coord.toml - copy to project root and edit
repo = "owner/repo"  # ← Replace with your repo
plan_path = "context" # ← Path to plan/task JSON files (default: "context")
```

**Drop-in hook script** (installs CLI if needed):
```bash
#!/bin/bash
if ! command -v claude-coord &> /dev/null; then
  curl -sSL https://raw.githubusercontent.com/user/claude-coord/main/install.sh | bash
fi
claude-coord claims
```

**Settings snippet** (`.claude/settings.json`):
```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/session-start-coord.sh"
          }
        ]
      }
    ]
  }
}
```

### Example Doc Structure (for AI Agents)

```markdown
## Quick Start (for AI agents)

### Step 1: Create config file
\`\`\`bash
cat > .claude-coord.toml << 'EOF'
repo = "owner/repo"  # ← Replace with your GitHub repo
plan_path = "context" # ← Path to plan/task JSON files
EOF
\`\`\`

### Step 2: Create SessionStart hook
The hook will auto-install the CLI if not present:
\`\`\`bash
mkdir -p .claude/hooks
cat > .claude/hooks/session-start-coord.sh << 'EOF'
#!/bin/bash
if ! command -v claude-coord &> /dev/null; then
  curl -sSL https://raw.githubusercontent.com/user/claude-coord/main/install.sh | bash
fi
claude-coord claims
EOF
chmod +x .claude/hooks/session-start-coord.sh
\`\`\`

### Step 3: Configure Claude settings
Add to `.claude/settings.json`:
\`\`\`json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/session-start-coord.sh"
          }
        ]
      }
    ]
  }
}
\`\`\`

### Step 4: Add to Web environment allowlist (Web only)
In Claude Code Web environment settings, add:
- `claude-coord.shuttleapp.rs`

### Step 5: Test
\`\`\`bash
.claude/hooks/session-start-coord.sh
\`\`\`

### Usage
\`\`\`bash
# View current claims (happens automatically on session start)
claude-coord claims

# Claim a task (one claim per session, cannot be changed)
claude-coord claim --task <task-id>

# If you get a conflict, refresh and pick another task
claude-coord claims
claude-coord claim --task <different-task-id>
\`\`\`
```

### Rust Crate Documentation

For Rust projects wanting direct crate integration, the `claude-coord` crate documentation should cover API usage. See crates.io/docs.rs for details.

## Future Enhancements

1. **Webhook integration** - GitHub webhooks for instant branch deletion detection (upgrade fallback)
2. **Metrics/observability** - Track claim patterns, completion rates
3. **Admin UI** - Simple dashboard to view/manage claims
