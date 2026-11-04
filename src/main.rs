use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    RustTarget,
    NodeModules,
    PythonVenv,
    PythonCache,
    GlobalCache,
    JavaBuild,
    Worktree,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::RustTarget => "rust-target",
            Kind::NodeModules => "node_modules",
            Kind::PythonVenv => "python-venv",
            Kind::PythonCache => "python-cache",
            Kind::GlobalCache => "global-cache",
            Kind::JavaBuild => "java-build",
            Kind::Worktree => "git-worktree",
        }
    }
}

struct Item {
    kind: Kind,
    path: PathBuf,
    size: u64,
    age_days: u64,
    gitdir: Option<PathBuf>,
}

const HELP: &str = "disk-cleanup [-n] [-y] [--global | PATH]

Finds Rust/Java build dirs, node_modules, Python venvs and caches, git worktrees under
PATH (default: current dir), lists them, and asks before deleting.
Worktrees with uncommitted changes are skipped.

  --global        scan your whole home directory plus tool caches (uv, cargo,
                  npm, pip, Homebrew, Xcode...). On macOS skips Library, Desktop,
                  Documents, Downloads, etc. so no permission prompts appear
  -n, --dry-run   list only, never delete
  -y, --yes       don't ask for confirmation (for scripts/cron)
";

fn human(b: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{b} B") } else { format!("{v:.1} {}", U[i]) }
}

/// Total size of a directory tree (does not follow symlinks), plus newest mtime.
fn dir_stats(path: &Path) -> (u64, SystemTime) {
    let mut size = 0;
    let mut newest = SystemTime::UNIX_EPOCH;
    let mut stack = vec![path.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(md) = e.metadata() else { continue };
            if let Ok(m) = md.modified() {
                newest = newest.max(m);
            }
            if md.is_dir() {
                stack.push(e.path());
            } else {
                size += md.len();
            }
        }
    }
    (size, newest)
}

fn age_days(t: SystemTime) -> u64 {
    SystemTime::now().duration_since(t).unwrap_or(Duration::ZERO).as_secs() / 86400
}

/// If `.git` is a file pointing into `<repo>/.git/worktrees/<name>`, return the repo's git dir.
fn linked_gitdir(dot_git: &Path) -> Option<PathBuf> {
    if !dot_git.is_file() {
        return None;
    }
    let txt = fs::read_to_string(dot_git).ok()?;
    let p = PathBuf::from(txt.trim().strip_prefix("gitdir:")?.trim());
    if p.parent()?.file_name()? != "worktrees" {
        return None; // submodule, not a worktree
    }
    Some(p.parent()?.parent()?.to_path_buf())
}

/// Package-manager and tool caches under $HOME (checked with --global). All are re-downloaded on demand.
const GLOBAL_CACHES: &[&str] = &[
    ".cache/uv",
    ".cache/pip",
    ".cache/pre-commit",
    ".cache/go-build",
    ".cargo/registry",
    ".cargo/git",
    ".npm/_cacache",
    ".yarn/cache",
    ".pnpm-store",
    ".gradle/caches",
    ".m2/repository",
    "Library/Caches/pip",
    "Library/Caches/Homebrew",
    "Library/Caches/Yarn",
    "Library/pnpm/store",
    "Library/Developer/Xcode/DerivedData",
];

fn scan(dir: &Path, skip: &[PathBuf], out: &mut Vec<Item>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let entries: Vec<_> = rd.flatten().collect();
    let has = |n: &str| entries.iter().any(|e| e.file_name() == n);
    let gradle = ["build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts"].iter().any(|n| has(n));

    if let Some(gitdir) = linked_gitdir(&dir.join(".git")) {
        let (size, newest) = dir_stats(dir);
        out.push(Item { kind: Kind::Worktree, path: dir.to_path_buf(), size, age_days: age_days(newest), gitdir: Some(gitdir) });
    }

    for e in &entries {
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_dir() || ft.is_symlink() {
            continue;
        }
        let name = e.file_name();
        let name = name.to_string_lossy();
        let path = e.path();
        let kind = match &*name {
            ".git" => continue,
            "target" if has("Cargo.toml") => Some(Kind::RustTarget),
            "node_modules" if has("package.json") => Some(Kind::NodeModules),
            ".venv" | "venv" if path.join("pyvenv.cfg").exists() => Some(Kind::PythonVenv),
            "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".ruff_cache" | ".tox" => Some(Kind::PythonCache),
            "target" if has("pom.xml") => Some(Kind::JavaBuild),
            ".gradle" if gradle => Some(Kind::JavaBuild),
            "build" if gradle => Some(Kind::JavaBuild),
            _ => None,
        };
        match kind {
            Some(k) => {
                let (size, newest) = dir_stats(&path);
                out.push(Item { kind: k, path, size, age_days: age_days(newest), gitdir: None });
            }
            // Skip huge noisy dirs that never contain projects we care about.
            None if matches!(&*name, "Library" | ".Trash" | ".cargo" | ".rustup" | "node_modules" | "target") => {}
            None if skip.contains(&path) => {}
            None => scan(&path, skip, out),
        }
    }
}

fn confirm(prompt: &str) -> bool {
    print!("{prompt} [y/N] ");
    io::stdout().flush().ok();
    let mut s = String::new();
    io::stdin().read_line(&mut s).ok();
    matches!(s.trim().to_lowercase().as_str(), "y" | "yes")
}

fn main() {
    let mut dry = false;
    let mut yes = false;
    let mut root = None;
    let mut home = false;
    for a in std::env::args().skip(1) {
        match a.as_str() {
            "-n" | "--dry-run" => dry = true,
            "-y" | "--yes" => yes = true,
            "--global" | "--home" => home = true,
            "-h" | "--help" => {
                print!("{HELP}");
                return;
            }
            s if s.starts_with('-') => {
                eprintln!("unknown option: {s}\n\n{HELP}");
                std::process::exit(2);
            }
            p => root = Some(PathBuf::from(p)),
        }
    }
    let mut skip: Vec<PathBuf> = vec![];
    let mut global: Vec<PathBuf> = vec![];
    if home {
        let Some(h) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
            eprintln!("cannot determine home directory");
            std::process::exit(2);
        };
        let h = PathBuf::from(h);
        if cfg!(target_os = "macos") {
            // Folders protected by macOS privacy (TCC) that trigger permission dialogs.
            for d in ["Library", "Desktop", "Documents", "Downloads", "Movies", "Music", "Pictures", "Public", "Applications", ".Trash"] {
                skip.push(h.join(d));
            }
        }
        for c in GLOBAL_CACHES {
            let p = h.join(c);
            if p.is_dir() {
                global.push(p.clone());
                skip.push(p);
            }
        }
        root = Some(h);
    }
    let root = root.unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        eprintln!("{} is not a directory", root.display());
        std::process::exit(2);
    }

    eprintln!("scanning {} ...", root.display());
    let mut items = vec![];
    scan(&root, &skip, &mut items);
    for p in global {
        let (size, newest) = dir_stats(&p);
        items.push(Item { kind: Kind::GlobalCache, path: p, size, age_days: age_days(newest), gitdir: None });
    }
    // A worktree's size includes artifacts inside it, which are listed separately;
    // subtract them so nothing is counted twice.
    let nested: Vec<(usize, u64)> = items
        .iter()
        .enumerate()
        .filter(|(_, w)| w.kind == Kind::Worktree)
        .map(|(wi, w)| {
            let inner = items.iter().filter(|i| i.path != w.path && i.path.starts_with(&w.path)).map(|i| i.size).sum();
            (wi, inner)
        })
        .collect();
    for (wi, inner) in nested {
        items[wi].size = items[wi].size.saturating_sub(inner);
    }
    items.sort_by(|a, b| b.size.cmp(&a.size));
    if items.is_empty() {
        println!("nothing found");
        return;
    }

    println!("{:<13} {:>10} {:>6}  PATH", "KIND", "SIZE", "AGE");
    for i in &items {
        println!("{:<13} {:>10} {:>5}d  {}", i.kind.label(), human(i.size), i.age_days, i.path.display());
    }
    let total: u64 = items.iter().map(|i| i.size).sum();
    let count = items.len();
    println!("\n{count} items can be removed, freeing up to {}.", human(total));
    println!("These are build outputs and caches that get recreated by your tools.");
    println!("Source code and git history are not touched; worktrees with uncommitted changes are skipped.");
    if dry {
        return;
    }
    if !yes && !confirm("Remove the items above?") {
        println!("aborted");
        return;
    }

    let mut freed = 0;
    // Plain artifacts first (they may live inside worktrees), then worktrees.
    for i in items.iter().filter(|i| i.kind != Kind::Worktree) {
        match fs::remove_dir_all(&i.path) {
            Ok(()) => {
                freed += i.size;
                println!("removed {}", i.path.display());
            }
            Err(e) => eprintln!("failed {}: {e}", i.path.display()),
        }
    }
    let mut repos: Vec<&PathBuf> = vec![];
    for i in items.iter().filter(|i| i.kind == Kind::Worktree) {
        let gd = i.gitdir.as_ref().unwrap();
        let ok = Command::new("git")
            .arg("--git-dir").arg(gd).args(["worktree", "remove"]).arg(&i.path)
            .status().map(|s| s.success()).unwrap_or(false);
        if ok {
            freed += i.size;
            println!("removed worktree {}", i.path.display());
        } else {
            eprintln!("skipped worktree {} (uncommitted changes or locked)", i.path.display());
        }
        if !repos.contains(&gd) {
            repos.push(gd);
        }
    }
    for gd in repos {
        let _ = Command::new("git").arg("--git-dir").arg(gd).args(["worktree", "prune"]).status();
    }
    println!("freed {}", human(freed));
}
