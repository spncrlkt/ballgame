# Code Review Process

Run this process during every audit to perform a comprehensive code quality review.

---

## Overview

The code review creates a dated file (`code_review_YYYY-MM-DD.md`) that:
1. Grades each technical area
2. Documents anti-patterns found
3. Builds/updates the best practices library
4. Creates prioritized improvement tasks

---

## Step 1: Gather Context

```bash
# Get current commit
git log -1 --format="%H %s" HEAD

# Count lines per file
find src -name "*.rs" | xargs wc -l | sort -n

# Check compilation
cargo check
cargo clippy 2>&1 | head -50
```

Read key files:
- `src/main.rs` - App setup, system registration
- `src/ai/decision.rs` - AI logic (often largest/most complex)
- `src/player/physics.rs` - Movement and physics
- `src/ball/physics.rs` - Ball physics
- Largest files by line count

---

## Step 2: Research Best Practices

Search for authoritative sources on:
- Game physics patterns (Fix Your Timestep, determinism)
- AI decision systems (FSM vs behavior trees)
- ECS architecture (Bevy-specific patterns)
- Performance (allocations, query optimization)
- Game design (genre-specific, balance)

Add new resources to `code_review_guidelines.md` References section.

---

## Step 3: Grade Each Area

| Area | What to Check | Grade Criteria |
|------|---------------|----------------|
| **Physics/Timing** | FixedUpdate usage, delta_secs, collision epsilon | A: All correct, B: Minor issues, C: Frame-rate dependent |
| **Input Handling** | Buffering pattern, consumption in FixedUpdate | A: All buffered, B: Most buffered, C: Raw input in FixedUpdate |
| **ECS Architecture** | Component design, query efficiency, system ordering | A: Clean separation, B: Some complexity, C: Misuse patterns |
| **AI Decision** | Goal clarity, scalability, debugging support | A: Modular/testable, B: Works but monolithic, C: Spaghetti |
| **Performance** | Allocations, RNG, hot paths | A: No issues, B: Minor hot path allocations, C: Per-frame allocations |
| **Game Design** | Balance, feel, feedback | A: Polished, B: Functional, C: Needs work |

---

## Step 4: Find Anti-Patterns

Check for these specific issues:

**God Functions** (>200 lines)
```bash
# Find large functions
rg "^pub fn|^fn" src/ -A 1 | head -100
```

**Scattered RNG**
```bash
rg "thread_rng\(\)" src/
```

**Complex Queries** (>6 tuple elements)
```bash
rg "Query<\(" src/ -A 2
```

**String Allocations in Hot Paths**
```bash
rg "format!\(|to_string\(\)" src/ --type rust
```

**Missing Time Scaling**
```bash
rg "velocity.*=" src/ | grep -v "delta_secs\|powf"
```

---

## Step 5: Document Findings

Create `code_review_YYYY-MM-DD.md` with sections:

1. **Executive Summary** - Grades table, key findings
2. **Best Practices Library** - Patterns with code examples and sources
3. **Anti-Patterns Analysis** - Issues found with file:line references
4. **Codebase Deep Dive** - File-by-file analysis of problem areas
5. **Game Design Notes** - Balance, feel, genre considerations
6. **Review Process Gaps** - What guidelines are missing
7. **Improvement Plan** - Prioritized tasks (P0-P3)
8. **Resources** - Links to authoritative sources

---

## Step 6: Update Documentation

1. **`code_review_guidelines.md`** - Add new patterns/resources discovered
2. **`code_review_audits.md`** - Append summary of findings with date
3. **`audit_record.md`** - Log session with commit reference
4. **`todo.md`** - Add improvement tasks from P0-P3 list

---

## Quick Categories (for `code_review_audits.md`)

For each issue found, categorize as:

1. **Duplication** - Repeated code patterns
2. **Complexity** - Hard to follow systems/functions
3. **Naming** - Misleading or outdated names
4. **Structure** - Code in wrong module
5. **Pattern violations** - Doesn't follow CLAUDE.md patterns
6. **Anti-patterns** - Game dev specific issues (god functions, scattered RNG, etc.)

Format:
```markdown
| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `file.rs:123` | Description | Fix suggestion |
```

---

## Example Output Structure

```
code_review_2026-01-25.md
├── Executive Summary (grades, key findings)
├── Best Practices Library
│   ├── Physics & Timing
│   ├── Input Handling
│   ├── ECS Architecture
│   ├── AI Decision Systems
│   └── Performance Patterns
├── Anti-Patterns Analysis
│   ├── Found in Codebase
│   └── Not Found (Good!)
├── Codebase Deep Dive
│   ├── High Priority Files
│   └── Duplication Patterns
├── Game Design Fundamentals
├── Review Process Gaps
├── Improvement Plan
│   ├── Short-Term (P0-P1)
│   ├── Medium-Term (P2-P3)
│   └── Long-Term
└── Resources & References
```

---

## Reference Documents

- `code_review_guidelines.md` - Detailed patterns and checklists
- `code_review_audits.md` - History of findings
- `audit_record.md` - Session logs
- Previous `code_review_*.md` files - Past deep reviews
