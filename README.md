# disk-cleanup

Dead-simple disk cleanup for developers. Scans a directory tree, lists build
artifacts and git worktrees, and deletes them after confirmation.

```
disk-cleanup [-n] [-y] [PATH]
```

- `PATH` – directory to scan (default: current directory)
- `-n`, `--dry-run` – list only, never delete
- `-y`, `--yes` – don't ask for confirmation (scripts/cron)

## What it finds

| Kind | Detected as |
|------|-------------|
| rust-target | `target` next to `Cargo.toml` |
| java-build | `target` next to `pom.xml`; `build`/`.gradle` next to a Gradle file |
| node_modules | `node_modules` next to `package.json` |
| python-venv | `.venv`/`venv` containing `pyvenv.cfg` |
| python-cache | `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.tox` |
| git-worktree | linked worktrees; removed with `git worktree remove` (dirty ones are skipped) |

## Install

Download a binary from the [releases page](../../releases), or:

```
cargo install --git https://github.com/vivainio/disk-cleanup
```
