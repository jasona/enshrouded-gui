//! Everything that touches the outside world: the config file, backups and the server process.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context as _, Result};

pub const DEFAULT_CONFIG_PATH: &str =
    "C:/Program Files (x86)/Steam/steamapps/common/EnshroudedServer/enshrouded_server.json";
pub const SERVER_EXE: &str = "enshrouded_server.exe";

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(windows)]
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

/// Config path from the first command line argument, falling back to the Steam default.
pub fn config_path() -> PathBuf {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH))
}

pub fn server_dir(config: &Path) -> PathBuf {
    config.parent().map(Path::to_path_buf).unwrap_or_default()
}

pub fn backup_dir(config: &Path) -> PathBuf {
    server_dir(config).join("config-backups")
}

/// Resolve a directory from the config (which may be relative to the server folder).
pub fn resolve_dir(config: &Path, configured: &str) -> PathBuf {
    let p = Path::new(configured);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    // Drop "." components so "./savegame" reads as ...\EnshroudedServer\savegame.
    let tidy: PathBuf = p.components().filter(|c| !matches!(c, std::path::Component::CurDir)).collect();
    server_dir(config).join(tidy)
}

pub fn read_config(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("could not read {}", path.display()))
}

/// Write via a temporary file in the same folder so a crash never leaves a half-written config.
pub fn write_config(path: &Path, text: &str) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).with_context(|| format!("could not write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| {
        let _ = fs::remove_file(&tmp);
        format!("could not replace {}", path.display())
    })
}

// ---- Backups -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Backup {
    pub path: PathBuf,
    pub file_name: String,
    pub when: String,
    pub size: u64,
}

/// Copy the current config into the backup folder. Returns `None` if there was nothing to back up.
pub fn make_backup(config: &Path) -> Result<Option<PathBuf>> {
    if !config.exists() {
        return Ok(None);
    }
    let dir = backup_dir(config);
    fs::create_dir_all(&dir).with_context(|| format!("could not create {}", dir.display()))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let target = dir.join(format!("enshrouded_server-{stamp}.json"));
    fs::copy(config, &target).with_context(|| format!("could not write {}", target.display()))?;
    prune_backups(config, 30);
    Ok(Some(target))
}

fn prune_backups(config: &Path, keep: usize) {
    let mut all = list_backups(config);
    while all.len() > keep {
        if let Some(old) = all.pop() {
            let _ = fs::remove_file(old.path);
        }
    }
}

/// Newest first.
pub fn list_backups(config: &Path) -> Vec<Backup> {
    let Ok(rd) = fs::read_dir(backup_dir(config)) else {
        return Vec::new();
    };
    let mut out: Vec<Backup> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let file_name = e.file_name().to_string_lossy().to_string();
            let when = file_name
                .trim_start_matches("enshrouded_server-")
                .trim_end_matches(".json")
                .to_string();
            let when = chrono::NaiveDateTime::parse_from_str(&when, "%Y%m%d-%H%M%S")
                .map(|d| d.format("%b %e, %Y  %H:%M:%S").to_string())
                .unwrap_or(when);
            Some(Backup { path: e.path(), file_name, when, size: meta.len() })
        })
        .collect();
    out.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    out
}

// ---- Server process ----------------------------------------------------------------------

fn hidden(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

pub fn is_running() -> bool {
    hidden("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {SERVER_EXE}"), "/FO", "CSV", "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains(SERVER_EXE))
        .unwrap_or(false)
}

pub fn start(config: &Path) -> Result<()> {
    let dir = server_dir(config);
    let exe = dir.join(SERVER_EXE);
    anyhow::ensure!(exe.exists(), "{} was not found next to the config file", SERVER_EXE);
    let mut cmd = Command::new(&exe);
    cmd.current_dir(&dir);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NEW_CONSOLE);
    cmd.spawn().with_context(|| format!("could not start {}", exe.display()))?;
    Ok(())
}

/// Ask the server to close (like clicking the X on its console window).
pub fn stop(force: bool) -> Result<()> {
    let mut cmd = hidden("taskkill");
    cmd.args(["/IM", SERVER_EXE, "/T"]);
    if force {
        cmd.arg("/F");
    }
    let out = cmd.output().context("could not run taskkill")?;
    anyhow::ensure!(out.status.success(), "taskkill: {}", String::from_utf8_lossy(&out.stdout).trim());
    Ok(())
}

pub fn open_folder(path: &Path) {
    let _ = fs::create_dir_all(path);
    let _ = Command::new("explorer").arg(path).spawn();
}

pub fn reveal_file(path: &Path) {
    let _ = Command::new("explorer").arg(format!("/select,{}", path.display().to_string().replace('/', "\\"))).spawn();
}

pub fn human_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{bytes} B") } else { format!("{:.1} KB", bytes as f64 / 1024.0) }
}
