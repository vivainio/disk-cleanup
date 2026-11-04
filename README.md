# disk-cleanup

Dead-simple disk cleanup for developers. Scans a directory tree, lists build
artifacts and git worktrees, and deletes them after confirmation.

```
disk-cleanup [-n] [-y] [--global | PATH]
```

- `PATH` – directory to scan (default: current directory)
- `--global` – scan your whole home directory and the usual tool caches instead of `PATH`. On macOS it skips
  `Library`, `Desktop`, `Documents`, `Downloads`, `Movies`, `Music`, `Pictures`,
  `Public`, `Applications` and `.Trash`, so no permission prompts appear
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
| global-cache | with `--global`: uv, pip, cargo registry/git, npm, yarn, pnpm, gradle, maven, Go build, Homebrew and Xcode DerivedData caches (re-downloaded on demand) |
| git-worktree | linked worktrees; removed with `git worktree remove` (dirty ones are skipped) |

## Install

Download the single binary for your platform straight into `~/.local/bin`
(macOS / Linux):

```sh
mkdir -p ~/.local/bin
case "$(uname -s)" in
  Darwin) asset=disk-cleanup-macos ;;
  *)      asset=disk-cleanup-linux-$(uname -m | sed 's/arm64/aarch64/') ;;
esac
curl -fsSL "https://github.com/vivainio/disk-cleanup/releases/latest/download/$asset" -o ~/.local/bin/disk-cleanup
chmod +x ~/.local/bin/disk-cleanup
```

Make sure `~/.local/bin` is on your `PATH`, then run `disk-cleanup ~/code`.

Release assets (a release can't hold several files with the same name, so each
carries its platform suffix; the `-o` above saves it as plain `disk-cleanup`):

- `disk-cleanup-linux-x86_64`, `disk-cleanup-linux-aarch64`
- `disk-cleanup-macos` (Apple Silicon)
- `disk-cleanup.exe` (Windows x86_64)

Or build from source: `cargo install --git https://github.com/vivainio/disk-cleanup`
