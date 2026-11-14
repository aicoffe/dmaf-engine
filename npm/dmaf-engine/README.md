# @aicoffe/dmaf-engine

[dmaf](https://github.com/aicoffe/dmaf-engine) — a deterministic multi-agent routing
engine for AI-native SDLC workflows: the engine is the only routing authority, session
state and audit live entirely on disk, and there is zero harness dependency at runtime.

## Install

```bash
npm i -g @aicoffe/dmaf-engine
```

The matching platform binary (`darwin-arm64` / `darwin-x64` / `linux-arm64` /
`linux-x64` / `win32-x64`) is installed automatically via `optionalDependencies` —
no download at install time.

## Verify

```bash
dmaf --version
dmaf doctor
```

## Getting started

```bash
dmaf new --dir my-project                                # scaffold a project
dmaf add https://dmaf.aicoffe.com/aicoffe/dmaf-sdlc      # install the runnable sample workflow
dmaf start --scope sdlc-standard --task "..." --var topic=demo
```

## Links

- Repository & docs: <https://github.com/aicoffe/dmaf-engine>
- Engine manual: `docs/engine-manual.md` in the repository
- Official site: <https://dmaf.aicoffe.com>
