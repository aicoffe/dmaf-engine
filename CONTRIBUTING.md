# Contributing

Thanks for your interest in contributing to D-MAF! **Issues and PRs in both English and Chinese are welcome.**

## Development setup

- Rust toolchain: rustup 1.85+ (`edition = "2021"`)
- The repository ships `.cargo/config.toml` with a USTC sparse registry mirror; adjust or remove it if it is slow in your region.

```bash
cargo build --release   # target/release/dmaf
cargo test              # all 180 tests are fully offline; everything must stay green
```

## Ground rules (see AGENTS.md for the full conventions, in Chinese)

- **Determinism is the product**: `compute_next` must never introduce clock, randomness, or IO. `tests/test_determinism.rs` guards this with randomized-trajectory regression.
- **Audit before state**: audit shards are written before state transitions, in an append-only hash chain that must never be rewritten.
- **Compiled artifacts are committed**: after changing `dmaf/workflows/`, run `dmaf compile` and commit `dmaf/compiled/`; `dmaf compile --check` must stay clean.
- **The definition layer is the single source of truth**: `dmaf/agents/` and `dmaf/skills/` changes reach harnesses only via `dmaf install --harness <name>` — never hand-edit harness directories.
- **No references to internal archives from tracked assets**: `docs-dev/` is a local, gitignored development archive (implementation plans, decision records). Source comments, AGENTS.md, and package assets must not point at specific files inside it.

## Documentation conventions

| Content | Truth source | Mirror / notes |
|---------|--------------|----------------|
| `README.md` | English (this is the primary README) | `README.zh-CN.md` is the Chinese mirror — keep both in sync |
| `docs/` top level | English default pages (`engine-manual.md`, `flow-definition-guide.md`, `phase-stage-scope.md`) | Chinese versions live in `docs/zh/` and are the **editing source of truth** for their content; when changing a Chinese doc, update its English page in the same PR |
| `docs/schemas/*.json` | Contract assets, not translated | Keep in sync with `src/schema_check.rs` |
| `dmaf/` definition layer | English prose (skills/agents/workflows) | Artifact language follows the user's task language; the conductor restates in the user's language |
| `docs-dev/` | Chinese, local-only | Never referenced by file path from tracked assets |

Every bilingual page pair carries a language-switch line at the top; keep it intact when editing.

## Pull requests

1. Branch from `master`.
2. Run `cargo test` and `dmaf compile --check` before opening the PR — CI runs the same (`cargo build`, `cargo test`, `cargo run -- compile --check`).
3. If your change affects workflow semantics, update the sample definitions, the compiled artifacts, `docs/zh/流程定义指南.md` and its English mirror in one PR.
4. Keep commit messages descriptive; the repository convention is Chinese commit messages summarizing scope, verification evidence, and rationale — follow the existing style (`git log --oneline`).

## License

By contributing you agree that your contributions are dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
