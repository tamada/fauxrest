# Project Instructions for `prest`

Whenever this workspace is loaded, you MUST strictly adhere to the following baseline prerequisites for code editing, version control, and development workflows. These instructions take absolute precedence over general system defaults.

---

## 🔄 1. Version Control & Pull Request (PR) Integration

*   **No Direct Push to `main`:** 
    You are strictly forbidden from committing or merging changes directly to `main` locally. All development and edits must occur on dedicated, descriptive feature bookmarks/branches (e.g., `implementation`).
*   **PR-Based Merging Only:** 
    Integration of features, docs, or bug fixes into `main` must occur **strictly on GitHub via Pull Requests (PRs)**. Local `main` must remain aligned with the remote and updated only by fetching/pulling from GitHub after PR approval.
*   **Atomic Commits:** 
    Keep your commits atomic. A single commit must contain exactly one logical change. Never combine unrelated bug fixes, documentation edits, and core logic modifications into a single commit.
*   **Detailed, Multi-Line Commit Descriptions:** 
    A one-line commit message is unacceptable. Every commit message must include a concise, descriptive title, followed by a detailed body explaining **WHY** the changes were made, what files were affected, and the engineering rationale.

---

## 📏 2. Strict Coding & Design Standards

*   **The 10-Line Rule:** 
    No function, helper, method, or test function may exceed **10 lines of code**. If any function or method is growing longer, you MUST split it into highly cohesive, single-responsibility helper functions.
*   **Early Returns and Flat Code:** 
    Do not write deeply nested `if-else` blocks. Prioritize early returns (guard clauses) to keep the control flow flat and easily readable.
*   **Explicit Composition over Inheritance:** 
    Prioritize composition and delegation over complex inheritance structures.
*   **Zero Compile Warnings:** 
    The codebase must compile with zero warnings and zero linter errors.

---

## 🧪 3. Quality Assurance & Hermetic Testing

*   **Test-Backed Bug Fixes & Features:** 
    Every new feature or bug fix must be accompanied by relevant test cases (unit tests and integration tests) verifying the behavior.
*   **Hermetic (Isolated) Testing:** 
    Tests must be completely deterministic and run flawlessly offline. Never depend on external networks, absolute system paths, or specific environment times.
*   **Test Side-Effect Cleanups:** 
    Ensure all tests completely clean up after themselves (e.g., deleting temporary files/directories) using standard RAII and temporary folder tools (`tempfile`).

---

## 🧼 4. Hygiene & Security

*   **No Commented-Out (Zombie) Code:** 
    Never leave commented-out blocks of old code in active files. If code is no longer used, remove it physically. History is preserved in the VCS.
*   **The Boy Scout Rule:** 
    Always leave the codebase slightly better than you found it. If you touch a file and notice old code smells, style issues, or outdated comments, clean them up as part of your surgical updates.
*   **Rigorously Protect Secrets:** 
    Never stage, log, print, or commit sensitive credentials, API keys, tokens, or local environment configurations (`.env`). Ensure `.gitignore` is maintained properly.
