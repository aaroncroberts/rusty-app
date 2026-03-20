# Release Process

This document defines the mandatory steps for every rusty-app release.
Follow them in order. Do not skip steps.

---

## Philosophy

Work is visible from day one. The draft PR is opened at the **start** of the
release cycle, not the end. Commits accumulate on `aaron/agentic-coder` and
are pushed continuously. Contributors can participate via the open PR. The PR
stays open until all quality gates pass, then it is merged, tagged, and released.

```
Open draft PR → commit & push loop → quality gates pass → mark ready → merge → tag → release
```

All work happens on `aaron/agentic-coder`. **Never delete this branch.**

> **Note:** rusty-app does not currently have GitHub Actions CI. Quality gates
> are enforced locally before merge. See the CI setup note in Step 5.

---

## Step 1 — Open the draft PR

At the start of any release cycle, open a draft PR from `aaron/agentic-coder`
targeting `main`. Do this before writing any code.

```bash
gh pr create \
  --base main \
  --draft \
  --title "feat: vX.Y.Z — <brief summary of the release>" \
  --body "$(cat <<'EOF'
## Summary
<!-- Fill in as work progresses -->

## Changes
<!-- Updated as commits land -->

## Test plan
- [ ] cargo build --workspace succeeds
- [ ] cargo fmt --all clean
- [ ] cargo clippy --workspace -- -D warnings clean
- [ ] cargo test --workspace passes
- [ ] CHANGELOG updated
- [ ] Version bumped in Cargo.toml

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Step 2 — Development loop

Work proceeds in normal commit cycles on `aaron/agentic-coder`. After each
logical unit of work:

```bash
git add <files>
git commit -m "feat|fix|chore|docs: description"
git push origin aaron/agentic-coder
```

**Picking up main's changes** (do this whenever main advances):

```bash
git fetch origin main
git merge origin/main --no-edit
# Resolve any conflicts locally, then:
git push origin aaron/agentic-coder
```

Never rebase a branch with an open PR — merge only.

---

## Step 3 — Local quality gates (before marking ready)

When all planned work is complete, run the full quality gate locally. Fix every
failure before proceeding.

```bash
# 1. Build
cargo build --workspace

# 2. Format (auto-fix, then verify clean)
cargo fmt --all
cargo fmt --check

# 3. Lint (zero warnings)
cargo clippy --workspace -- -D warnings

# 4. Tests
cargo test --workspace
```

All four must exit 0. Do not proceed with any red check.

For integration tests requiring database containers (see `TESTING.md`):

```bash
# Start containers
podman-compose up -d

# Run integration tests (sequential — required for DB tests)
cargo test --workspace -- --test-threads=1

# Stop containers
podman-compose down
```

---

## Step 4 — Mark the PR ready for review

```bash
gh pr ready <pr-number>
```

Update the PR body with the final summary and change list.

---

## Step 5 — Quality gate before merge

Since rusty-app does not yet have GitHub Actions CI, the merge gate is
enforced manually: **all quality gates from Step 3 must pass locally before
merging.** Do not merge with a failing check.

To add GitHub Actions CI in the future, see `docs/draft-pr-workflow.md` for
the full CI-gated workflow pattern.

---

## Step 6 — Merge the PR

Once all quality gates pass:

```bash
gh pr merge <pr-number> --squash --delete-branch=false
```

`--delete-branch=false` ensures `aaron/agentic-coder` is preserved on the
remote. Never use `--delete-branch` or `--delete-branch=true`.

---

## Step 7 — Tag the release

After the PR is merged:

```bash
git checkout main
git pull origin main
git tag vX.Y.Z
git push origin vX.Y.Z
```

---

## Step 8 — Verify the release

```bash
gh release view vX.Y.Z
```

Confirm:
- GitHub release exists with the correct tag
- Release notes match the CHANGELOG `[X.Y.Z]` section
- Expected build artifacts are attached (if release automation is configured)

---

## Step 9 — Return to aaron/agentic-coder

Sync the working branch with the release commit so the next cycle starts clean:

```bash
git checkout aaron/agentic-coder
git fetch origin main
git merge origin/main --no-edit
git push origin aaron/agentic-coder
```

---

## Branch rules

| Branch | Purpose | Delete after release? |
| :--- | :--- | :--- |
| `main` | Release branch — always green, always releasable | Never |
| `aaron/agentic-coder` | Primary working branch | **Never** |

---

## Quick reference card

```bash
# Step 1 — Open draft PR
gh pr create --base main --draft --title "feat: vX.Y.Z — ..."

# Step 2 — Development loop
git commit -m "..." && git push origin aaron/agentic-coder

# Merge main changes
git fetch origin main && git merge origin/main --no-edit && git push

# Step 3 — Quality gates
cargo fmt --all && cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace

# Step 4 — Mark ready
gh pr ready <pr-number>

# Step 6 — Merge (only when quality gates pass)
gh pr merge <pr-number> --squash --delete-branch=false

# Step 7 — Tag
git checkout main && git pull origin main
git tag vX.Y.Z && git push origin vX.Y.Z

# Step 8 — Verify
gh release view vX.Y.Z

# Step 9 — Sync working branch
git checkout aaron/agentic-coder
git fetch origin main && git merge origin/main --no-edit
git push origin aaron/agentic-coder
```
