# Template Variable Syntax Candidates

Research into replacing the current `${VAR}` / `${VAR:-default}` template syntax
with something that has zero conflicts across all ecosystems common-repo manages
files for.

## Requirements

- Zero conflicts with any shell, CI/CD system, template engine, query language,
  config format, or programming language that could appear in managed files
- Safe as a value (no escaping/quoting needed) in: YAML, TOML, JSON, INI, XML,
  Markdown
- Easy to type on a standard US keyboard
- Visually scannable in a file full of config
- Must support default values (e.g., `SYNTAX:-default` or `SYNTAX, default`)

## File formats common-repo processes

YAML, JSON, TOML, INI, Markdown, and (planned) XML.

---

## Candidates explored

### Current syntax

| Syntax | Reason rejected |
|---|---|
| `${VAR}` | Shell, Docker Compose, Dockerfiles, GitHub Actions `run:`, GitLab CI, CircleCI, envsubst, Kubernetes, Nginx, systemd, Gradle, Maven, CMake, Nix, Dhall, Mako, Velocity, FreeMarker, Salt. The entire reason for this research. |

### Dollar + braces with inner marker

| Syntax | Reason rejected |
|---|---|
| `$${VAR}` | Docker Compose uses this as its own escape for literal `${VAR}`. Terraform/HCL and Make also use `$$` as escape for `$`. |
| `${!VAR}` | Bash indirect variable expansion (`${!prefix*}`, `${!name}`). |
| `${#VAR}` | Bash string length operator (`${#VAR}` returns length of `$VAR`). |
| `${^VAR}` | Bash 4+ uppercase first character (`${VAR^}` uppercases first char). |
| `${%VAR}` | No known conflict, but still uses `${...}` which collides with the broader shell/Docker/CI/CD family if a tool does prefix-insensitive matching. |
| `${~VAR}` | Bash tilde expansion (rare but exists). |
| `${@VAR}` | No known conflict in isolation, but `${...}` envelope is the problem. |
| `${=VAR}` | Same `${...}` envelope problem. |
| `${+VAR}` | Same `${...}` envelope problem. |
| `${.VAR}` | Same `${...}` envelope problem. |
| `${/VAR}` | Bash substitution syntax `${VAR/pattern/replacement}`. |
| `${>VAR}` | Same `${...}` envelope problem. |
| `${<VAR}` | Same `${...}` envelope problem. |
| `${'VAR}` | Same `${...}` envelope problem. |

### Dollar + square brackets

| Syntax | Reason rejected |
|---|---|
| `$[VAR]` | MongoDB filtered positional operator (`$[elem]`) -- exact `$[IDENTIFIER]` match, appears in YAML/JSON configs. Also: deprecated bash/zsh arithmetic (`$[1+2]`), Azure Pipelines runtime expressions (`$[ expr ]`), JSONPath root bracket access, MySQL/PostgreSQL `$[last]`, Jsonnet dynamic root access. |

### Dollar + angle brackets

| Syntax | Reason rejected |
|---|---|
| `$<VAR>` | Angle brackets conflict with XML/HTML. `<` and `>` are reserved characters in XML requiring entity escaping. Visually confusing near shell redirects. common-repo plans to support XML merge. |

### Dollar + parentheses

| Syntax | Reason rejected |
|---|---|
| `$(VAR)` | Make variable expansion, POSIX shell command substitution, Tekton parameter syntax, CFEngine variable expansion. |
| `$(=VAR=)` | Shell `$(...)` is command substitution; while `$(=FOO=)` is a syntax error in practice, the `$(` prefix may trigger shell escaping/highlighting and confuse tools that scan for command substitution. |

### Dollar + caret

| Syntax | Reason rejected |
|---|---|
| `$^VAR^` | GNU Make `$^` is the automatic variable for all prerequisites (fatal). Zsh `$^` is RC_EXPAND_PARAM array expansion (fatal). Perl `$^X` special variables. PowerShell `$^` automatic variable. Windows CMD `^` is escape character. Pandoc Markdown `^text^` superscript. LaTeX `$^` enters math+superscript. |

### Dollar + other single-char delimiters (symmetric)

| Syntax | Reason rejected |
|---|---|
| `$+VAR+` | Technically clean, but symmetric delimiter means default values containing `+` cause ambiguity (e.g., `$+REGEX:-foo+bar+`). |
| `$..VAR..` | `..` is extremely common in file paths (`/usr/../bin`), causing parse ambiguity in defaults. |
| `$-VAR-` | Hyphens blend into YAML (list items, key names). Nearly invisible -- poor scannability. |
| `$=VAR=` | Minor INI concern (`=` is key-value separator in some parsers). More importantly, symmetric delimiter ambiguity with defaults containing `=`. |
| `$~VAR~` | Perl `$~` is a special variable (output format name). Common Lisp `~` is format directive prefix. Elixir sigils use `~` + letter + delimiters. |
| `$/VAR/` | Perl `$/` is input record separator. Ruby `$/` is also input record separator. Visual similarity to regex `/pattern/` causes confusion. |
| `$.VAR.` | Perl `$.` is input line number variable. JSONPath uses `$.` for root access. Single dots too subtle -- poor scannability. |
| `$:VAR:` | YAML parse error: `$:` triggers mapping value parsing. Colon is the key-value separator. |

### Double-brace families

| Syntax | Reason rejected |
|---|---|
| `{{ VAR }}` | Jinja2, Ansible, Helm, Handlebars, Mustache, Go templates, Liquid, Twig, Tera, cookiecutter, copier, Salt. |
| `${{ VAR }}` | GitHub Actions expression syntax. |
| `{{{ VAR }}}` | Handlebars raw/unescaped output. |

### Double-bracket families

| Syntax | Reason rejected |
|---|---|
| `[[VAR]]` | TOML array of tables (`[[section]]`). Bash extended test (`[[ expr ]]`). Lua long strings. C++ attributes (`[[nodiscard]]`). MediaWiki/wiki links (`[[Page]]`). Obsidian internal links. |
| `[|VAR|]` | OCaml array literal syntax. F# array literals. Coq notation. |
| `[:VAR:]` | POSIX regex character classes (`[:alpha:]`, `[:digit:]`). |

### Angle bracket families

| Syntax | Reason rejected |
|---|---|
| `<[VAR]>` | Angle brackets conflict with XML/HTML. |
| `<<VAR>>` | Shell/Perl/Ruby heredoc (`<<EOF`). C++ stream/template operators. |
| `<:VAR:>` | Perl6/Raku Unicode property match in regex (`<:Letter>`). Also angle brackets are problematic for XML. |
| `<%= VAR %>` | ERB (Ruby on Rails). |
| `{% VAR %}` | Jinja2 block/statement syntax. |
| `{<VAR>}` | Angle brackets inside braces -- still problematic for XML attribute values and any HTML context. |

### Percent-based

| Syntax | Reason rejected |
|---|---|
| `%{VAR}` | Puppet variable syntax. |
| `%[VAR]` | C `scanf` format specifier scanset (`%[a-z]`). Close to Ruby `%w[]`/`%i[]` sigil syntax. |
| `%%VAR%%` | Python configparser (INI) interprets `%%` as escaped `%`, mangling to `%VAR%`. Windows batch `%%` in FOR loops. Printf-family `%%` escape. |

### At-sign based

| Syntax | Reason rejected |
|---|---|
| `@{VAR}` | PowerShell hashtable literal syntax (`@{key=value}`). |
| `@[VAR]` | Objective-C array literal syntax (`@[@"item"]`). |
| `@VAR@` | Autoconf/autotools substitution. CMake `configure_file()`. Maven resource filtering. Meson `configure_file()`. pkg-config `.pc.in` files. Gradle `ReplaceTokens`. |

### Hash-based

| Syntax | Reason rejected |
|---|---|
| `#[VAR]` | Rust attribute syntax (`#[derive(Debug)]`). Also `#` starts comments in YAML, TOML, INI, shell, Python -- value would be silently stripped. |
| `#{VAR}` | Ruby string interpolation inside double-quoted strings. |
| `##VAR##` | C preprocessor token-pasting operator. YAML/TOML/INI treat `#` as comment start -- value silently stripped. |

### Ampersand-based

| Syntax | Reason rejected |
|---|---|
| `&[VAR]` | Rust slice reference syntax (`&[T]`). YAML anchor syntax (`&anchor`). HTML/XML entity start character (`&amp;`). |
| `&{VAR}` | YAML `&` is anchor. XML/HTML entity character. |

### Asterisk-based

| Syntax | Reason rejected |
|---|---|
| `*{VAR}` | YAML alias reference (`*anchor`). Markdown emphasis. Shell glob. |

### Exclamation-based

| Syntax | Reason rejected |
|---|---|
| `!{VAR}` | YAML tag syntax (`!custom`). Markdown image prefix (`![alt](url)`). Shell history expansion. |

### Backtick-based

| Syntax | Reason rejected |
|---|---|
| `` `VAR` `` | Shell legacy command substitution. Markdown inline code. JavaScript template literals. MySQL identifier quoting. Go raw strings. Reserved in YAML spec. |

### Tilde-based

| Syntax | Reason rejected |
|---|---|
| `~{VAR}` | Common Lisp `~{` is the FORMAT iteration directive. Possible bash edge case with tilde + brace expansion (`~{a,b}` could interact). Elixir sigils use `~` prefix. |
| `~(VAR)` | Ksh/bash extended globbing `~(...)` pattern negation. |
| `~~VAR~~` | GitHub Flavored Markdown strikethrough. CommonMark extensions. Supported by Discord, Slack, Reddit, Notion, Obsidian. |
| `~[VAR]` | No known conflict, but `[` `]` are YAML flow sequence markers and TOML array syntax. In unquoted YAML, `~[FOO]` starting a value would be parsed as null (`~`) followed by a flow sequence. |

### Caret-based

| Syntax | Reason rejected |
|---|---|
| `^{VAR}` | Git ref dereferencing (`HEAD^{commit}`). LaTeX superscript (`^{text}`). |
| `^(VAR)` | Fish shell older stderr redirection. |
| `^[VAR]` | Terminal ESC escape sequence (ASCII 27 = `^[`). |
| `^^VAR^^` | Windows CMD `^^` is escaped `^`. Pandoc/AsciiDoc `^text^` superscript. |

### Equals-based

| Syntax | Reason rejected |
|---|---|
| `==VAR==` | MediaWiki level-2 heading (`==Heading==`). Obsidian/Pandoc highlight markup (`==text==`). |
| `{=VAR=}` | Handlebars/Mustache delimiter-change syntax similarity (`{{=...=}}`). Python f-string/str.format `{:format}` similarity. |

### Plus-based

| Syntax | Reason rejected |
|---|---|
| `++VAR++` | AsciiDoc inline passthrough (`++text++` renders as literal/monospace). |
| `$++VAR++` | AsciiDoc passthrough still applies to the `++VAR++` portion. Symmetric delimiter ambiguity with defaults containing `++`. |

### Pipe-based

| Syntax | Reason rejected |
|---|---|
| `|>VAR<|` | `|` is special in YAML (block scalar), Markdown (tables), shell (pipe), regex (alternation). Elixir/F# pipe operator `|>`. |
| `{|VAR|}` | MediaWiki table syntax (`{|` opens, `|}` closes). OCaml record/quoted string syntax. |
| `(|VAR|)` | Banana brackets in formal verification tools and math notation. |

### Dollar + double-char symmetric

| Syntax | Reason rejected |
|---|---|
| `$$VAR$$` | LaTeX display math mode (`$$...$$`). Shell `$$` is PID. Make/Docker/Terraform `$$` is escaped `$`. |

### CSS-inspired keyword prefix

| Syntax | Reason rejected |
|---|---|
| `var(VAR)` | CSS custom properties function `var(--name)`. Direct conflict. |
| `env(VAR)` | CSS environment variable function `env(safe-area-inset-top)`. |

### Mixed bracket asymmetric

| Syntax | Reason rejected |
|---|---|
| `({VAR})` | No known conflict found, but `{` inside `()` may confuse bracket-matching linters and syntax highlighters. `{` is a YAML flow mapping character. |
| `{(VAR)}` | Same linter/highlighter concern. `{` starts YAML flow mapping. |
| `(=VAR=)` | No known conflict found. Viable candidate -- see survivors below. |

---

## Survivors (zero conflicts found in symbolic syntax)

| Syntax | Default syntax | Pros | Cons |
|---|---|---|---|
| **`@@VAR@@`** | `@@VAR:-default@@` | Visually unmistakable. Trivial regex. Explicitly distinct from autoconf's single `@VAR@`. Zero conflicts in any format, language, shell, CI/CD, or template engine. Already used ad-hoc by teams as a "safe" sentinel. Easy to type (Shift+2). | Visually heavy. Git unified diff uses `@@` in hunk headers (non-issue in practice -- those lines always contain `-`/`+` and spaces). Ruby `@@var` is class variables (distinct -- no closing `@@`). `@@` triggers autocomplete in some tools (Claude CLI, Discord @-mentions). |
| **`cr(VAR)`** | `cr(VAR:-default)` | Tool-namespaced. Minimal and clean. Reads like a function call. No conflict in any ecosystem. | Opaque to newcomers (`cr` = common-repo?). If tool is renamed, prefix feels wrong. Parentheses in unquoted YAML are safe but unusual. |
| **`tpl(VAR)`** | `tpl(VAR:-default)` | Generic "template" prefix. Same benefits as `cr()`. No conflicts. | Same naming fragility concern. Less branded than `cr()`. |
| **`(=VAR=)`** | `(=VAR:-default=)` | Unique asymmetric delimiters. Nothing uses `(=...=)`. | Unusual looking. Not immediately recognizable as "variable substitution" to most developers. |

## Sentinel identifier approach

All symbolic syntaxes carry inherent risk because special characters have meaning
in various ecosystems. A plain identifier sentinel avoids this entirely — it's
just a string in every format.

### Sentinel prefixes evaluated (GitHub code search)

| Prefix | GitHub hits | Why rejected / accepted |
|---|---|---|
| `__CR__` | ~25 | Carriage return escape (`\r` replacement) in multiple projects. Kubernetes "Custom Resource". STM32 C preprocessor define. BAML client registry variable. |
| `__CMR__` | ~12 | ISO country code for Cameroon — used by datamaps library as topology placeholder. Appears in geographic data configs. |
| `__COMMONREPO__` | 0 | Safe. Slightly less readable than underscore-separated version. |
| `__COMREPO__` | 0 | Safe. |
| **`__COMMON_REPO__`** | **0** | **Safe. Most readable. Chosen as final syntax.** |

## Final decision

**Syntax: `__COMMON_REPO__VARNAME__`**

- Pattern: `__COMMON_REPO__` + variable name + `__`
- Variable names: `[A-Za-z_][A-Za-z0-9_]*` — must not contain double underscores
  (`__`), since `__` is the closing delimiter. A name like `MY_VAR__NAME` would
  parse as variable `MY_VAR` with trailing `NAME__` left as literal text.
- Regex: `__COMMON_REPO__([A-Za-z_][A-Za-z0-9_]*?)__` (lazy match stops at first `__`)
- Zero GitHub results. Zero conflicts in any format, language, or tool.
- Valid as an unquoted value in YAML, TOML, JSON, INI, XML, and Markdown.
- Inline defaults (`:-default`) removed; defaults live in `template-vars` config.
- Upstream `template-vars` already provides defaults; consumers override via
  last-write-wins in operation order.

Tracked in: https://github.com/common-repo/common-repo/issues/264
