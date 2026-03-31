---
description: Formalize a release by running verification, bumping the semver tag, committing, and pushing to GitHub.
---

# /release <type>

`type` is one of: `patch` (v0.x.Y → v0.x.Y+1) or `minor` (v0.X.0 → v0.X+1.0).

// turbo-all

## Steps

1. **Verify build integrity**
   ```bash
   cargo check --release && cargo test
   ```
   Abort if any step returns non-zero.

2. **Determine current version**
   ```bash
   git describe --tags --abbrev=0
   ```
   Parse output as `vMAJOR.MINOR.PATCH`.

3. **Compute next version** based on `type`:
   - `patch`: increment PATCH → `vMAJOR.MINOR.(PATCH+1)`
   - `minor`: increment MINOR, reset PATCH → `vMAJOR.(MINOR+1).0`

4. **Stage all changes**
   ```bash
   git add -A
   ```

5. **Commit** (attempt GPG signed; fall back to `[UNSIGNED-ENV-TRUSTED]` prefix if no secret key):
   ```bash
   git -c commit.gpgsign=false commit -m "[UNSIGNED-ENV-TRUSTED] arch(<scope>): release <NEXT_VER>; cargo check+test pass; <STATE_HASH>"
   ```
   If GPG is available, drop the `-c commit.gpgsign=false` override and the `[UNSIGNED-ENV-TRUSTED]` prefix.

6. **Create annotated tag**
   ```bash
   git tag -a <NEXT_VER> -m "arch(<scope>): Release <NEXT_VER>"
   ```

7. **Push branch and tags**
   ```bash
   git push origin main --tags
   ```

8. **Verify remote state**
   ```bash
   gh repo view GhrammR/laplace-s-oracle --json url,defaultBranchRef
   ```

9. **Report**: Output the GitHub release URL:
   `https://github.com/GhrammR/laplace-s-oracle/releases/tag/<NEXT_VER>`
