#!/bin/bash
# Security audit for prompt injection and related vulnerabilities
# Run as part of Tier 2 audits or standalone

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

echo "=== Security Audit ==="
echo ""

ISSUES_FOUND=0
WARNINGS=0

# 1. Prompt injection patterns
echo "1. Checking for prompt injection patterns..."
INJECTION_MATCHES=$(grep -rniE "ignore.*previous|disregard.*instruction|forget.*instruction|you are now|new instruction|system prompt" \
    --include="*.rs" --include="*.txt" --include="*.json" --include="*.toml" 2>/dev/null | grep -v security_audit || true)
if [ -n "$INJECTION_MATCHES" ]; then
    echo "   WARNING: Potential prompt injection patterns found:"
    echo "$INJECTION_MATCHES" | head -5
    ISSUES_FOUND=$((ISSUES_FOUND + 1))
else
    echo "   OK: No prompt injection patterns"
fi
echo ""

# 2. Dynamic SQL
echo "2. Checking for dynamic SQL..."
# Known safe patterns:
#   - analytics/requests.rs - intentional dynamic SQL for analysis tool
#   - simulation/db.rs - builds dynamic WHERE but uses parameterized values (?)
DYNAMIC_SQL=$(grep -rn '\.prepare(&' src/ --include="*.rs" 2>/dev/null || true)
DYNAMIC_COUNT=$(echo "$DYNAMIC_SQL" | grep -c 'prepare' || echo "0")
KNOWN_SAFE=2  # analytics/requests.rs + simulation/db.rs (parameterized)
if [ "$DYNAMIC_COUNT" -gt "$KNOWN_SAFE" ]; then
    echo "   WARNING: Found $DYNAMIC_COUNT dynamic SQL calls (expected $KNOWN_SAFE)"
    echo "$DYNAMIC_SQL"
    WARNINGS=$((WARNINGS + 1))
else
    echo "   OK: $DYNAMIC_COUNT dynamic SQL calls (all known/documented)"
fi
echo ""

# 3. Parameterized queries (good pattern)
echo "3. Checking for parameterized queries (good pattern)..."
PARAMS_COUNT=$(grep -rn 'params!\[' src/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
echo "   Found $PARAMS_COUNT parameterized queries (safe pattern)"
echo ""

# 4. Base64/encoding patterns
echo "4. Checking for encoding patterns..."
ENCODING=$(grep -rniE "base64|atob|btoa|decode.*string" --include="*.rs" src/ 2>/dev/null || true)
if [ -n "$ENCODING" ]; then
    echo "   WARNING: Encoding patterns found - review for obfuscation:"
    echo "$ENCODING" | head -3
    WARNINGS=$((WARNINGS + 1))
else
    echo "   OK: No suspicious encoding patterns"
fi
echo ""

# 5. Unicode escape sequences
echo "5. Checking for Unicode escape sequences..."
UNICODE=$(grep -rniE '\\u00|\\x[0-9a-f]{2}' --include="*.rs" --include="*.txt" src/ config/ 2>/dev/null || true)
if [ -n "$UNICODE" ]; then
    echo "   WARNING: Unicode escapes found - review for homoglyph attacks:"
    echo "$UNICODE" | head -3
    WARNINGS=$((WARNINGS + 1))
else
    echo "   OK: No suspicious Unicode escapes"
fi
echo ""

# 6. Command execution
echo "6. Checking for command execution..."
CMD_EXEC=$(grep -rn 'Command::new\|std::process::Command' src/ --include="*.rs" 2>/dev/null | grep -v test || true)
if [ -n "$CMD_EXEC" ]; then
    echo "   Review needed: Command execution found"
    echo "$CMD_EXEC" | head -3
else
    echo "   OK: No command execution in main source"
fi
echo ""

# 7. File operations with dynamic paths
echo "7. Checking file operations..."
FILE_OPS=$(grep -rn 'fs::read_to_string\|fs::write\|File::open\|File::create' src/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
echo "   Found $FILE_OPS file operations - manual review recommended for path validation"
echo ""

# 8. Format strings embedding user data
echo "8. Checking format strings in output generation..."
FORMAT_CALLS=$(grep -rn 'push_str(&format!' src/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
echo "   Found $FORMAT_CALLS format! calls in string building"
echo "   Known: training/analysis.rs embeds profile names (validated config)"
echo ""

# Summary
echo "=== Summary ==="
echo "Critical issues: $ISSUES_FOUND"
echo "Warnings:        $WARNINGS"
echo ""

if [ $ISSUES_FOUND -gt 0 ]; then
    echo "FAIL: $ISSUES_FOUND critical issues found"
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo "REVIEW: $WARNINGS warnings need attention"
    exit 0
else
    echo "PASS: No security issues found"
    exit 0
fi
