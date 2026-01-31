# Security Audit Checklist

Automated security checks for prompt injection, SQL injection, and related vulnerabilities.

## Quick Reference

```bash
# Run all security checks
./scripts/security_audit.sh

# Individual checks
grep -rn "ignore.*previous\|disregard\|forget.*instruction" --include="*.rs" --include="*.txt" --include="*.md"
grep -rn "prepare(&\|execute(&" src/ --include="*.rs"
```

---

## Automated Checks

### 1. Prompt Injection Patterns

Search for strings that look like prompt injection attempts:

```bash
# Suspicious instruction patterns
grep -rniE "ignore.*previous|disregard.*instruction|forget.*instruction|you are now|new instruction|system prompt|jailbreak" \
  --include="*.rs" --include="*.txt" --include="*.md" --include="*.json" --include="*.toml"

# Base64 patterns (potential obfuscation)
grep -rniE "base64|atob|btoa|decode.*string" --include="*.rs"

# Unicode escape sequences (potential homoglyph attacks)
grep -rniE "\\\\u00|\\\\x[0-9a-f]{2}" --include="*.rs" --include="*.txt"
```

**Expected:** No matches (clean codebase)

### 2. SQL Injection Vectors

Search for dynamic SQL construction:

```bash
# Direct SQL string interpolation (DANGER)
grep -rn 'format!.*SELECT\|format!.*INSERT\|format!.*UPDATE\|format!.*DELETE' src/ --include="*.rs"

# Prepare with dynamic SQL (review each case)
grep -rn 'prepare(&[^"]' src/ --include="*.rs"
grep -rn '\.prepare(&' src/ --include="*.rs"

# Safe patterns (parameterized queries)
grep -rn 'params!\[' src/ --include="*.rs"
```

**Known Issues:**
- `src/analytics/requests.rs:102` - Uses `prepare(&query.sql)` where `query.sql` comes from config file
  - **Status:** Accepted risk (local-only tool, documented behavior)
  - **Mitigation:** Config file is trusted, not user-facing

### 3. Unescaped User Data in Output

Search for format strings that embed user data without escaping:

```bash
# Markdown generation with user data
grep -rn 'push_str(&format!' src/ --include="*.rs" | grep -v test

# Review: Are profile names, player notes, or other user inputs embedded?
```

**Known Issues:**
- `src/training/analysis.rs` - Embeds AI profile names in markdown
  - **Status:** Low risk (profile names come from config, not arbitrary user input)
  - **Mitigation:** Validate profile names against allowed pattern `^[A-Za-z0-9_-]+$`

### 4. Path Traversal

Search for file operations with user-controlled paths:

```bash
# File reads/writes with dynamic paths
grep -rn 'fs::read\|fs::write\|File::open\|File::create' src/ --include="*.rs"

# Path joining (review for traversal)
grep -rn '\.join(' src/ --include="*.rs" | grep -v test
```

**Expected:** All paths should be:
- Hardcoded constants
- Derived from validated config
- Validated against allowed directories

### 5. Command Injection

Search for shell command execution:

```bash
# Process spawning
grep -rn 'Command::new\|std::process' src/ --include="*.rs"

# Shell execution
grep -rn 'sh -c\|bash -c\|system(' src/ --include="*.rs"
```

**Expected:** No matches or only with hardcoded commands

### 6. Deserialization Safety

Search for unsafe deserialization:

```bash
# Serde deserialization (generally safe, but review custom deserializers)
grep -rn '#\[derive.*Deserialize' src/ --include="*.rs"

# Custom deserialization
grep -rn 'impl.*Deserialize' src/ --include="*.rs"
```

**Expected:** Only standard serde derives, no unsafe custom deserializers

---

## Manual Review Checklist

Run these checks during deep audits:

### Config File Security

- [ ] `config/analysis_requests.json` - Contains SQL queries; ensure file permissions are restricted
- [ ] `config/ai_profiles.txt` - Profile names should match `^[A-Za-z0-9_-]+$`
- [ ] `config/levels.txt` - Level names should be sanitized
- [ ] All config files - No executable code, no hidden instructions

### LLM Output Security (if applicable)

- [ ] Generated markdown files don't contain prompt injection
- [ ] User notes/comments are not embedded in LLM prompts without escaping
- [ ] Analysis output is treated as untrusted data

### Database Security

- [ ] SQLite databases are local-only
- [ ] No sensitive data stored (credentials, tokens, etc.)
- [ ] Query results are escaped before display

---

## Integration with Audit Tiers

Add to **Tier 2: Standard Audit (every ~10 changes)**:
- [ ] Run `./scripts/security_audit.sh` (when created)
- [ ] Review any new `format!` calls embedding user data
- [ ] Check for new SQL queries

Add to **Tier 3: Deep Audit (weekly or before release)**:
- [ ] Full manual review of config file handling
- [ ] Review all file I/O operations
- [ ] Verify no new command execution

---

## Known Accepted Risks

| Issue | Location | Risk | Justification |
|-------|----------|------|---------------|
| Dynamic SQL | `analytics/requests.rs:102` | Medium | Local-only tool, config is trusted |
| Dynamic SQL | `simulation/db.rs:543` | Safe | Uses parameterized values with `?` placeholders |
| Unescaped profiles | `training/analysis.rs` | Low | Profiles from validated config |
| Command exec | `generate/gif_*.rs` | Safe | Hardcoded ffmpeg calls for asset generation |

---

## Creating the Automation Script

Save as `scripts/security_audit.sh`:

```bash
#!/bin/bash
# Security audit for prompt injection and related vulnerabilities

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

echo "=== Security Audit ==="
echo ""

ISSUES_FOUND=0

# 1. Prompt injection patterns
echo "Checking for prompt injection patterns..."
if grep -rniE "ignore.*previous|disregard.*instruction|forget.*instruction|you are now|new instruction" \
    --include="*.rs" --include="*.txt" --include="*.json" --include="*.toml" 2>/dev/null | grep -v security_audit.md; then
    echo "WARNING: Potential prompt injection patterns found"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    echo "  OK: No prompt injection patterns"
fi
echo ""

# 2. Dynamic SQL (excluding known safe patterns)
echo "Checking for dynamic SQL..."
DYNAMIC_SQL=$(grep -rn 'prepare(&[^"]' src/ --include="*.rs" 2>/dev/null | grep -v 'params!' || true)
if [ -n "$DYNAMIC_SQL" ]; then
    echo "  Review needed (known issue in analytics/requests.rs):"
    echo "$DYNAMIC_SQL" | head -5
else
    echo "  OK: No unexpected dynamic SQL"
fi
echo ""

# 3. Unescaped format strings in markdown
echo "Checking for unescaped user data in output..."
# This is informational - manual review needed
MARKDOWN_FORMATS=$(grep -rn 'push_str(&format!' src/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
echo "  Found $MARKDOWN_FORMATS format! calls in push_str - review for user data embedding"
echo ""

# 4. Base64/encoding (potential obfuscation)
echo "Checking for encoding patterns..."
if grep -rniE "base64|atob|btoa" --include="*.rs" src/ 2>/dev/null; then
    echo "WARNING: Encoding patterns found - review for obfuscation"
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    echo "  OK: No suspicious encoding patterns"
fi
echo ""

# 5. Command execution
echo "Checking for command execution..."
if grep -rn 'Command::new' src/ --include="*.rs" 2>/dev/null | grep -v test; then
    echo "  Review: Command execution found"
else
    echo "  OK: No command execution in main source"
fi
echo ""

# Summary
echo "=== Summary ==="
if [ $ISSUES_FOUND -eq 0 ]; then
    echo "No critical issues found"
    exit 0
else
    echo "$ISSUES_FOUND potential issues need review"
    exit 1
fi
```

Make executable: `chmod +x scripts/security_audit.sh`
