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

Download the single binary for your platform straight into `~/.local/bin`
(macOS / Linux):

```sh
mkdir -p ~/.local/bin
os=$(uname -s | tr A-Z a-z); [ "$os" = darwin ] && os=macos
arch=$(uname -m); [ "$arch" = arm64 ] && arch=aarch64
curl -fsSL "https://github.com/vivainio/disk-cleanup/releases/latest/download/disk-cleanup-$os-$arch" -o ~/.local/bin/disk-cleanup
chmod +x ~/.local/bin/disk-cleanup
```

Make sure `~/.local/bin` is on your `PATH`, then run `disk-cleanup ~/code`.

Release assets (a release can't hold several files with the same name, so each
carries its platform suffix; the `-o` above saves it as plain `disk-cleanup`):

- `disk-cleanup-linux-x86_64`, `disk-cleanup-linux-aarch64`
- `disk-cleanup-macos-aarch64`, `disk-cleanup-macos-x86_64`
- `disk-cleanup-windows-x86_64.exe`

Or build from source: `cargo install --git https://github.com/vivainio/disk-cleanup`
