# Community and Governance Best Practices

Research on open source community management, governance models, and contributor engagement patterns.

## Governance Models

### BDFL (Benevolent Dictator for Life)

Single maintainer or small group with final decision authority. Common in creator-led projects.

**Examples**: Python (historically), Perl, Ruby, Rails, Scala
**Pros**: Fast decisions, clear direction, reduced bikeshedding
**Cons**: Burnout risk, bus factor, can disenfranchise contributors
**Best for**: Early-stage projects, strong technical vision

The role is "less about dictatorship and more about diplomacy"—ensuring the right people gain influence as the project grows. ([Stack Overflow](https://stackoverflow.blog/2020/09/09/open-source-governance-benevolent-dictator-or-decision-by-committee/))

### Meritocratic/Committee Governance

Decision-making distributed among active contributors who demonstrate expertise.

**Examples**: Apache projects, Rust (via RFC process)
**Pros**: Distributes workload, builds community ownership, reduces single points of failure
**Cons**: Slower decisions, can be exhausting to manage discussions
**Best for**: Mature projects with active contributor base

Rust's approach: Each major decision starts as an RFC where everyone discusses tradeoffs. This "community-driven approach is Rust's secret sauce for quality." ([Rust Governance](https://www.rust-lang.org/governance))

### Hybrid Models

Many successful projects combine models: BDFL for day-to-day with steering committee for major decisions, or specialized teams with domain authority.

**Rust team structure**:
- Leadership Council (oversight)
- Compiler, Language, Library teams (technical)
- Dev Tools, Infrastructure teams (tooling)
- Moderation team (community)
- Launching Pad (incubation)

## Governance Documents

### GOVERNANCE.md

Document that describes:
- Decision-making process (who decides what)
- Role definitions and responsibilities
- How to become a maintainer/contributor
- Conflict resolution procedures
- Voting mechanisms (if any)

### CODEOWNERS

Maps code paths to responsible reviewers. Ensures appropriate expertise reviews changes.

```
# Example patterns
* @org/core-team
/docs/ @org/docs-team
/src/security/ @org/security-team
*.rs @org/rust-reviewers
```

**Best practices**:
- Start broad, refine as needed
- Use team handles over individuals (reduces single points)
- Review and update periodically
- Particularly valuable in monorepos

([Harness Blog](https://www.harness.io/blog/mastering-codeowners))

## Community Health Files

### CONTRIBUTING.md

Essential guide for new contributors. Examined patterns from uv, Tokio, Rust:

**uv approach** (8 sections):
1. Finding ways to help (labeled issues)
2. Setup requirements
3. Testing (snapshot testing, Python versions)
4. Docker testing
5. Profiling/benchmarking
6. Documentation
7. Releases

Key principle: Pre-approval required for unlabeled issues and new features—discourages speculative work.

**Tokio approach**:
- Opens with "No contribution is too small"
- Clear pathways: Issues → Triage → PRs
- Commit message conventions (module prefix, imperative voice)
- Review culture: "recognize the person behind the code"
- Label system: Area (A-), Category (C-), Difficulty (E-), Module (M-), Topic (T-)

**Rust approach**:
- Concise gateway document
- Directs to rustc-dev-guide for details
- Emphasizes finding mentors via Zulip
- Separates concerns (subtrees vs main repo)

### CODE_OF_CONDUCT.md

The Contributor Covenant is the de facto standard, adopted by 40,000+ projects including Linux, Rails, Swift, Go, Kubernetes.

**Current version**: 3.0

**Enforcement levels**:
1. Correction (written warning)
2. Warning (reduced interaction)
3. Temporary ban
4. Permanent ban

**Critical**: A code without enforcement "sends a false signal that your project is welcoming and inclusive, and can create a dangerous situation for marginalized people." ([Open Source Guide](https://opensource.guide/code-of-conduct/))

### SECURITY.md

Security reporting process. Covered in security-practices.md research.

## Issue and PR Templates

### Issue Templates

Modern projects use YAML-based issue forms (not just markdown templates):

**Ruff template structure**:
- `1_bug_report.yaml` - Bug reports
- `2_rule_request.yaml` - New rule requests
- `3_question.yaml` - Questions
- `config.yml` - Template configuration

**Best practices**:
- Use form schema for structured input
- Required fields for critical info (version, reproduction steps)
- Set `blank_issues_enabled: false` to enforce templates
- Link related issues automatically

### PR Templates

Single `PULL_REQUEST_TEMPLATE.md` or multiple in `PULL_REQUEST_TEMPLATE/` directory.

**Key sections**:
- Summary/description
- Related issues (Fixes #, Refs #)
- Checklist (tests pass, docs updated)
- Breaking changes notice

**Benefits**:
- Uniform review process
- Reduced back-and-forth
- Clear expectations upfront

### Organization-Wide Templates

Create `.github` repository at org level for shared templates across all repos.

## GitHub Discussions

Complementary to Issues for community engagement.

**When to use Discussions**:
- Discovery phase (low certainty)
- Community feedback/polls
- FAQs and knowledge sharing
- Show and tell
- General conversation

**When to use Issues**:
- Defined work items
- Bug reports
- Feature requests (after discussion)

**Key insight**: "Discussions are for discussing things. Issues are for cataloguing the work you need to do after you've reached a decision." ([GitHub Resources](https://resources.github.com/devops/process/planning/discussions/))

**Statistics**:
- 30% higher engagement with proper labels
- 60% more interactions with open-ended questions
- 35% more engagement when discussions link to issues

**Best practices**:
- Use categories (General, Ideas, Q&A, Show and Tell)
- Pin important discussions
- Convert to issues when work is defined
- Enable voting for prioritization

([GitHub Blog](https://github.blog/2024-05-06-create-a-home-for-your-community-with-github-discussions/))

## Contributor Onboarding

### "Good First Issue" Label

GitHub-recommended label for newcomer-friendly issues.

**Characteristics of good first issues**:
- Low complexity
- Well-documented steps
- Limited scope
- Mentorship available

**Kubernetes criteria**:
- Clear description of problem and solution
- Pointers to relevant code
- Expected time commitment
- Mentor identified

**Tools**:
- [goodfirstissue.dev](https://goodfirstissue.dev/) - Aggregator
- First Timers GitHub App - Automates issue creation
- GitHub ML-powered good first issues feature

([First Timers Only](https://www.firsttimersonly.com/))

### "Help Wanted" Label

Broader than good-first-issue, indicates maintainers welcome help but issue may be more complex.

### Onboarding Documentation

**Best practices**:
- Clear development setup instructions
- Local testing guidance
- Architecture overview
- Communication channels (Discord, Zulip, Slack)
- Response time expectations

## Burnout Prevention

Maintainer burnout is a significant OSS challenge.

**Prevention strategies**:
- Renewable/term-limited governance roles
- Clear boundaries on scope
- Automation for routine tasks
- Distributed maintainership
- Default to "No" on scope creep
- Take breaks openly

**Quote**: "I find it's far easier to prevent burnout by doing this [defaulting to 'No']... which means the community will keep its BDFL for a bit longer." ([Jeff Geerling](https://www.jeffgeerling.com/blog/2016/why-i-close-prs-oss-project-maintainer-notes))

Django's founders retired as BDFLs after 9 years: "the longer I observe the Django community, the more I realize that our community doesn't need [us]." ([Open Source Guide](https://opensource.guide/leadership-and-governance/))

## Label Systems

Consistent labeling improves discoverability and triage.

**Common label categories**:
- **Type**: bug, feature, enhancement, question, documentation
- **Priority**: P0/critical, P1/high, P2/medium, P3/low
- **Status**: needs-triage, needs-design, needs-decision, in-progress
- **Difficulty**: good-first-issue, help-wanted, expert-needed
- **Area**: specific components or modules

**Tokio system**:
- A- (Area): tokio, runtime, sync
- C- (Category): bug, feature, maintenance
- E- (Difficulty): easy, medium, hard
- M- (Module): specific module
- T- (Topic): documentation, performance

## Exemplar Projects

| Project | Governance | Notable Practice |
|---------|------------|------------------|
| Rust | RFC + Teams | Comprehensive team structure |
| uv | BDFL (Astral) | Strong contribution guidelines |
| Tokio | Meritocratic | Detailed commit conventions |
| Kubernetes | Committee | Label system, SIG structure |
| Apache | Meritocratic | Foundation governance |

## Key Takeaways

1. **Start with basics**: CONTRIBUTING.md, CODE_OF_CONDUCT.md, issue templates
2. **Match governance to stage**: BDFL for early projects, distribute as you grow
3. **Enforce codes of conduct**: Unenforced codes are worse than none
4. **Use Discussions**: Separate conversation from work tracking
5. **Label consistently**: Enables discovery and triage
6. **Onboard intentionally**: Good first issues with mentorship
7. **Prevent burnout**: Distribute work, set boundaries, take breaks
8. **Document processes**: Clear expectations reduce friction

## Sources

- [Open Source Guide - Leadership and Governance](https://opensource.guide/leadership-and-governance/)
- [The Open Source Way 2.0](https://www.redhat.com/en/blog/guidebook-open-source-community-management-open-source-way-20)
- [GitHub Docs - Issue Templates](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/configuring-issue-templates-for-your-repository)
- [GitHub Docs - Discussions Best Practices](https://docs.github.com/en/discussions/guides/best-practices-for-community-conversations-on-github)
- [Contributor Covenant](https://www.contributor-covenant.org/)
- [Rust Governance](https://www.rust-lang.org/governance)
- [Harness - CODEOWNERS](https://www.harness.io/blog/mastering-codeowners)
- [Stack Overflow - BDFL vs Committee](https://stackoverflow.blog/2020/09/09/open-source-governance-benevolent-dictator-or-decision-by-committee/)
