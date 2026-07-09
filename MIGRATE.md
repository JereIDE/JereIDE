# Migration Plan: JereIDE_Pro → JereIDE org (preserving the 3 stars)

Goal: make the JereIDE code public **in the JereIDE org**, carrying the 3 stars
currently on `jeremy-qian/jereide`, while keeping an archive of the old
`jeremy-qian/jereide` content.

Key fact: GitHub stars are tied to the **repo identity**, not the code. They
transfer with the repo on rename/transfer. So the star-bearing repo
(`jeremy-qian/jereide`) must *become* the destination; stars cannot be copied
onto a fresh repo.

## Pre-checks

- You are an **owner** of the JereIDE org.
- Org settings allow **public repos**.
- `JereIDE/jereide` and `jeremy-qian/jereide-archive` do **not** already exist.
- `jeremy-qian/jereide` is the 3-star repo and is accessible.

## Steps

1. **Archive the old content** (using your existing local clone of the star repo):
   - In that local clone: `git remote set-url origin git@github.com:jeremy-qian/jereide-archive.git`
   - Create the empty repo `jeremy-qian/jereide-archive` on GitHub first.
   - `git push` (or `git push --mirror` to preserve all branches/tags).
   - On GitHub, mark `jeremy-qian/jereide-archive` as **Archived** (read-only).
   - The GitHub repo `jeremy-qian/jereide` (3 stars) is untouched — only its
     local remote moved.

2. **Load JereIDE_Pro code into the star repo** (stars survive):
   - In the JereIDE_Pro working copy: `git remote set-url origin git@github.com:jeremy-qian/jereide.git`
   - `git push --force-with-lease origin main`
     (force is required — histories diverge; stars are repo-level and survive.)
   - If the transferred repo has branch protection, disable it for the push,
     then re-enable.

3. **Transfer into the org** (stars preserved):
   - GitHub UI: transfer `jeremy-qian/jereide` → JereIDE org.
   - Becomes `JereIDE/jereide`, still 3 stars. Old URL now redirects.
   - Set visibility to **Public**.

4. **Fix hardcoded URLs**:
   - `Cargo.toml` `workspace.package`: `repository` / `homepage` / `documentation`
     → `https://github.com/JereIDE/jereide` (currently `Jeremy-Qian/JereIDE_Pro`,
     lines 11–13).
   - Workflows already use `github.repository` (no hardcoded owner) — no change.
   - Commit and push the Cargo.toml change.

5. **Verify & finalize**:
   - `JereIDE/jereide` is public, shows 3 stars, contains the JereIDE code, README
     renders (screenshot at `.github/images/screenshot.png`).
   - Keep `Jeremy-Qian/JereIDE_Pro` private as a backup, or delete once confident.
     Live source of truth is now `JereIDE/jereide`.

## Notes / risks

- Low risk: every destructive step is backed by the Phase-1 archive, and
  transfers / force-pushes are reversible.
- Do **not** rename `jeremy-qian/jereide` into the archive — stars would follow
  the rename and land on the archive (new code would get 0 stars).
- After migration, point your local `origin` at `JereIDE/jereide`.
