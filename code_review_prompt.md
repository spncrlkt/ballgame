# Code Review Prompt

Use this prompt during audits to perform a structured code quality review.

---

Review the codebase for code quality issues. Focus on these specific categories in order of priority:

1. **Duplication** - Find repeated code patterns that could be extracted into shared functions/helpers
2. **Complexity** - Systems or functions that are hard to follow; suggest simplifications
3. **Naming** - Components, resources, systems, or variables with misleading or outdated names
4. **Structure** - Code that's in the wrong module, or modules that have unclear boundaries
5. **Pattern violations** - Code that doesn't follow the patterns documented in CLAUDE.md

For each issue found:
- File and line number
- What the problem is (1 sentence)
- Suggested fix (code snippet if helpful)

Scope constraints:
- Don't suggest adding tests, docs, or comments
- Don't suggest architectural rewrites - only incremental improvements
- Focus on the src/ directory, skip constants.rs unless naming is unclear

After the review, append findings to `code_review_audits.md` with the current date.
