# Laplace's Oracle Constitutional System

## Foundational Axioms

1. The 8GB Law
Maximum active RAM footprint is capped at 100MB. Heap growth for primary data structures is forbidden.

2. Memory Asceticism (Zero-Copy)
Use zero-copy, memory-mapped, and structurally stable access patterns whenever possible. Avoid unnecessary copies and transient buffers.

3. Structural Determinism
No brittle regex mutation, nondeterministic state transitions, or unseeded randomness in operational paths. State changes must be structural, explicit, and reproducible.

## Operational Rule

All CI/CD operations are governed by these laws. If a script fails, the state is considered compromised.

## Axioms of Operational Purity

1. Atomic Patching
Do not use sed, perl, or fragmented echo calls for file edits. Generate a single Python script (`patch.py`) that performs all `read_text().replace()` operations and execute it once via `wsl.exe`.

2. Unicode Escape Mandate
All non-ASCII characters in Rust source (\u{26A1}, Braille, etc.) MUST be written using hex escape sequences (for example, `\\u{26A1}`). Never send raw Unicode over the shell bridge.

3. Non-Interactive GitOps
Always use `git commit -F <file>` to avoid shell quoting errors with parentheses.
Always use `gh release create --notes-file <file>` or `--notes ""` to prevent CLI editor hangs.

4. Token Conservation
Minimize file echoes. Before writing, check if the change is already present using `grep`.

5. Syntactic Gatekeeping
After every file mutation, you MUST execute `cargo check --manifest-path src/Cargo.toml`. If the compiler returns a 'parse error' or 'syntax error', you must immediately revert the file to its previous state and analyze the replacement logic. Never commit or report a change that breaks the AST.

6. No Greedy Replacements
Avoid replacing short, common symbols (like `?`, `-`, `;`) globally. Use unique anchor strings (at least 20 characters of surrounding context) to ensure surgical precision.

7. The After-Action Report (AAR)
Upon completing any architectural phase or release, you MUST output a detailed Markdown summary. This summary must include: Files Modified, Exact Byte-Size changes to TelemetryFrame, a brief explanation of the Bitwise/Mathematical logic implemented, and the GitHub Release URL.

8. The Innovation Log
You MUST maintain 'INNOVATION_LOG.md' as a persistent ledger. Read it before modifying it. Do NOT remove an idea unless it is fully implemented, mathematically impossible under the 8GB Law, or explicitly rejected. Append as many hypotheses, feature improvements, Aha! moments, and architectural realizations as you discover after every release. You MUST also include at least one 'Meta-Improvement' (a suggestion for a new or improvement to a Codex skill, automation, or CI/CD workflow to improve your own operational efficiency). Link to this log in your AAR.

9. Documentation Sync
Before executing /release, you MUST autonomously execute the /sync-docs skill and commit the resulting changes.

10. The Backlog Prune
After completing a phase defined in `BACKLOG.md`, you MUST delete that phase from the file. If `BACKLOG.md` becomes empty, delete the file entirely and remove this axiom.
