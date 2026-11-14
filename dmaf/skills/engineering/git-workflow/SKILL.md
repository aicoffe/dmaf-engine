---
name: git-workflow
description: Git workflow and PR process. Branch operations, commit conventions, sync strategy, hotfix, tags, opening and handling PRs. Use when the user asks to "open a PR", "branch operations", "merge code", "hotfix", "handle review comments".
---

# Git Workflow

Trunk-based: short-lived branches + PR merges into a protected mainline.

## Stack binding (executed before this skill's workflow)

Read the project's tech-stack declaration and act according to its state — one of four cases (when no declaration is provided, treat it as declaration absent):

- **Bound**: load the stack skill directory the "Binding" section points to — `SKILL.md` is the entry point and required reading, then route by the trigger conditions in its "Reference files" table to load the `reference/` documents for the domains the current task touches (one task reads only its own domain; dumping the whole directory is forbidden); when a stack rule conflicts with this skill, the more specific stack rule wins — escalate to a human if it cannot be adjudicated
- **Binding is "none" (generalized mode)**: execute with the neutral standard + model generalization based on the stack names declared in `backend_stack` / `frontend_stack`; the verification loop follows the declaration's "Verification commands" section; label artifacts "Generalized mode (<stack name>, no binding skill)"
- **Declaration absent**: execute per this skill's neutral standard; label artifacts "No tech stack declared for this project"
- **Declaration unparseable**: report as-is; do not guess

## Workflow

### 1. Daily flow
1. Branch from the latest main: `{type}/{ticket}-{summary}` (feat/fix/chore/docs/refactor/test)
2. Small commits: Conventional Commits `{type}({scope}): {summary}`, one intent per commit
3. Rebase on the latest main locally before pushing; rebasing pushed public branches is forbidden
4. Open the PR (≤ 400 lines; split if over the limit)
5. Approved → squash or merge (one style repository-wide); delete the feature branch
6. Branch lifetime ≤ 3 days

### 2. Pre-PR self-check
- CI fully green (lint, tests, coverage gate)
- Description complete: background, change, verification method, risks, checklist
- Linked spec/defect number; the contract updated for API changes
- AI-generated portions marked

### 3. Handling review comments
- block comments must be fixed or escalated for adjudication; adopt suggest comments or give a written rationale
- Re-request review after each revision round and state what changed; empty "fixed" replies are forbidden

### 4. Hotfix
```
Branch a fix from main (or the release tag) → minimal fix + regression test → expedited review (state the production impact) → release → cherry-pick back to the mainline → retrospective adds the missing test
```

### 5. Tags and rollback
- Releases are tagged `v{major}.{minor}.{patch}` with release notes; moving a published tag is forbidden
- Erroneous merged changes are reverted with `git revert`; force push only on unshared personal branches — protected branches never

## Boundaries (not in this skill)

- Review content standards (what to look at, how to judge, comment severity levels) go through `code-review`; this skill covers only process operations

## Reference files

- `rules/standards/engineering-standard.md` — higher-level constraints

## Default values

PR ≤ 400 lines, branch lifetime ≤ 3 days, and keeping the previous scheme for one version cycle are organization defaults (adjustable per project, with the adjustment recorded).
