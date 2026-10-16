//! Application state and behaviour. Rendering lives in `shell.rs` and `pages.rs`.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

use gpui::{
    actions, AppContext as _, Context, Entity, FocusHandle, Subscription, Window,
};
use gpui_component::{
    input::{InputEvent, InputState},
    notification::Notification,
    slider::{SliderEvent, SliderState},
    WindowExt as _,
};
use serde_json::Value;

use crate::{
    config::{self, Change, Config, Issue, Level, Val},
    schema::{Category, Kind, DEFS},
    server::{self, Backup},
};

actions!(enshrouded, [SaveConfig, ReloadConfig]);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Page {
    Overview,
    Server,
    Gameplay,
    Players,
    Bans,
    Review,
    Backups,
}

impl Page {
    pub const ALL: [Page; 7] = [
        Page::Overview,
        Page::Server,
        Page::Gameplay,
        Page::Players,
        Page::Bans,
        Page::Review,
        Page::Backups,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Page::Overview => "Overview",
            Page::Server => "Server",
            Page::Gameplay => "Gameplay",
            Page::Players => "Players",
            Page::Bans => "Bans",
            Page::Review => "Review",
            Page::Backups => "Backups",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Page::Overview => "The state of your realm at a glance.",
            Page::Server => "Identity, network, chat and discovery.",
            Page::Gameplay => "Difficulty presets and fine-grained world rules.",
            Page::Players => "Who may join, and what they may do once inside.",
            Page::Bans => "Accounts barred from the server.",
            Page::Review => "Every pending change and the exact file that will be written.",
            Page::Backups => "Automatic snapshots taken before each save.",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Page::Overview => "flame",
            Page::Server => "server",
            Page::Gameplay => "sliders",
            Page::Players => "users",
            Page::Bans => "ban",
            Page::Review => "diff",
            Page::Backups => "archive",
        }
    }
}

pub enum Load {
    Ready,
    Missing,
    Broken(String),
}

#[derive(Clone, Debug)]
pub enum Confirm {
    Discard,
    ReloadFromDisk,
    DeleteGroup(usize),
    Unban(usize),
    ForceStop,
    StopServer,
    Restore(PathBuf),
    DeleteBackup(PathBuf),
    ResetGameplay,
    ReplaceBroken,
    QuitDiscard,
}

pub struct GroupUi {
    pub name: Entity<InputState>,
    pub pass: Entity<InputState>,
    pub reserved: Entity<SliderState>,
    _subs: Vec<Subscription>,
}

pub struct Manager {
    pub path: PathBuf,
    pub cfg: Config,
    pub baseline: Value,
    pub load: Load,
    pub disk_mtime: Option<SystemTime>,
    pub disk_changed: bool,

    pub page: Page,
    pub cat: Category,
    pub running: bool,
    pub stop_requested: Option<Instant>,
    pub confirm: Option<Confirm>,
    pub backups: Vec<Backup>,
    pub focus: FocusHandle,

    // Derived on every render.
    pub dirty: bool,
    pub issues: Vec<Issue>,
    pub changes: Vec<Change>,

    pub name_in: Entity<InputState>,
    pub save_in: Entity<InputState>,
    pub log_in: Entity<InputState>,
    pub ip_in: Entity<InputState>,
    pub port_in: Entity<InputState>,
    pub slot_slider: Entity<SliderState>,
    pub sliders: Vec<Option<Entity<SliderState>>>,
    pub groups_ui: Vec<GroupUi>,
    /// Layout signature of the previous frame. Sliders only learn their width while painting,
    /// so a change in layout needs one more frame before thumbs sit in the right place.
    pub last_sig: Option<(Page, Category, usize, bool)>,
    pub allow_close: bool,
    _subs: Vec<Subscription>,
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn read_config(path: &Path) -> (Config, Load) {
    if !path.exists() {
        return (Config::fresh(), Load::Missing);
    }
    match server::read_config(path).and_then(|t| Config::parse(&t)) {
        Ok(cfg) => (cfg, Load::Ready),
        Err(e) => (Config::fresh(), Load::Broken(format!("{e:#}"))),
    }
}

fn bind_text(
    input: &Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<Manager>,
    set: impl Fn(&mut Manager, String) + 'static,
) -> Subscription {
    cx.subscribe_in(input, window, move |this, st, ev: &InputEvent, _window, cx| {
        if matches!(ev, InputEvent::Change) {
            set(this, st.read(cx).value().to_string());
            cx.notify();
        }
    })
}

fn bind_slider(
    slider: &Entity<SliderState>,
    window: &mut Window,
    cx: &mut Context<Manager>,
    set: impl Fn(&mut Manager, f32) + 'static,
) -> Subscription {
    cx.subscribe_in(slider, window, move |this, _st, ev: &SliderEvent, _window, cx| {
        let SliderEvent::Change(v) = ev;
        set(this, v.start());
        cx.notify();
    })
}

fn text_input(
    window: &mut Window,
    cx: &mut Context<Manager>,
    value: &str,
    placeholder: &str,
) -> Entity<InputState> {
    let (value, placeholder) = (value.to_string(), placeholder.to_string());
    cx.new(|cx| InputState::new(window, cx).default_value(value).placeholder(placeholder))
}

fn snap(kind: Kind, v: f32) -> f64 {
    let step = match kind {
        Kind::Factor { .. } => 0.05,
        _ => 1.0,
    };
    let snapped = (v as f64 / step).round() * step;
    (snapped * 10_000.0).round() / 10_000.0
}

impl Manager {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let path = server::config_path();
        let (cfg, load) = read_config(&path);
        let baseline = cfg.to_value();

        let name_in = text_input(window, cx, &cfg.name, "Enshrouded Server");
        let save_in = text_input(window, cx, &cfg.save_directory, "./savegame");
        let log_in = text_input(window, cx, &cfg.log_directory, "./logs");
        let ip_in = text_input(window, cx, &cfg.ip, "0.0.0.0");
        let port_in = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(cfg.query_port.to_string())
                .pattern(regex::Regex::new(r"^\d{0,5}$").expect("static regex"))
                .placeholder("15637")
        });
        let slot_slider = cx.new(|_| {
            SliderState::new().min(1.).max(16.).step(1.).default_value(cfg.slot_count as f32)
        });

        let mut subs = vec![
            bind_text(&name_in, window, cx, |m, v| m.cfg.name = v),
            bind_text(&save_in, window, cx, |m, v| m.cfg.save_directory = v),
            bind_text(&log_in, window, cx, |m, v| m.cfg.log_directory = v),
            bind_text(&ip_in, window, cx, |m, v| m.cfg.ip = v),
            bind_text(&port_in, window, cx, |m, v| m.cfg.query_port = v.trim().parse().unwrap_or(0)),
            bind_slider(&slot_slider, window, cx, |m, v| m.cfg.slot_count = v.round() as u32),
        ];

        let mut sliders = Vec::new();
        for (i, def) in DEFS.iter().enumerate() {
            let slider = match def.kind {
                Kind::Factor { min, max } | Kind::Minutes { min, max } | Kind::Int { min, max } => {
                    let step = if matches!(def.kind, Kind::Factor { .. }) { 0.05 } else { 1.0 };
                    let value = cfg.game[i].num() as f32;
                    let s = cx.new(|_| SliderState::new().min(min).max(max).step(step).default_value(value));
                    let kind = def.kind;
                    subs.push(bind_slider(&s, window, cx, move |m, v| m.cfg.game[i] = Val::Num(snap(kind, v))));
                    Some(s)
                }
                _ => None,
            };
            sliders.push(slider);
        }

        let focus = cx.focus_handle();
        let mut this = Manager {
            disk_mtime: mtime(&path),
            backups: server::list_backups(&path),
            path,
            cfg,
            baseline,
            load,
            disk_changed: false,
            page: Page::Overview,
            cat: Category::Player,
            running: false,
            stop_requested: None,
            confirm: None,
            focus,
            dirty: false,
            issues: Vec::new(),
            changes: Vec::new(),
            name_in,
            save_in,
            log_in,
            ip_in,
            port_in,
            slot_slider,
            sliders,
            groups_ui: Vec::new(),
            last_sig: None,
            allow_close: false,
            _subs: subs,
        };
        this.build_group_ui(window, cx);
        this.start_polling(window, cx);
        window.focus(&this.focus);

        // Refuse to close the window while there are unsaved edits, and ask first.
        let view = cx.entity();
        window.on_window_should_close(cx, move |_window, cx| {
            view.update(cx, |m, cx| {
                m.refresh_derived();
                if m.dirty && !m.allow_close {
                    m.confirm = Some(Confirm::QuitDiscard);
                    cx.notify();
                    false
                } else {
                    true
                }
            })
        });
        this
    }

    /// Recompute dirty flag, validation issues and the change list. Called once per render.
    pub fn refresh_derived(&mut self) {
        let now = self.cfg.to_value();
        self.dirty = now != self.baseline;
        self.changes = if self.dirty { config::diff(&self.baseline, &now) } else { Vec::new() };
        self.issues = self.cfg.validate();
    }

    pub fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.level == Level::Error).count()
    }

    pub fn ready(&self) -> bool {
        matches!(self.load, Load::Ready | Load::Missing)
    }

    // ---- Controls <-> config ------------------------------------------------------------

    pub fn build_group_ui(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.groups_ui.clear();
        for i in 0..self.cfg.groups.len() {
            let g = self.cfg.groups[i].clone();
            let name = text_input(window, cx, &g.name, "Group name");
            let pass = cx.new(|cx| {
                InputState::new(window, cx).default_value(g.password.clone()).masked(true).placeholder("Password")
            });
            let reserved = cx.new(|_| {
                SliderState::new().min(0.).max(16.).step(1.).default_value(g.reserved_slots as f32)
            });
            let subs = vec![
                bind_text(&name, window, cx, move |m, v| {
                    if let Some(g) = m.cfg.groups.get_mut(i) {
                        g.name = v;
                    }
                }),
                bind_text(&pass, window, cx, move |m, v| {
                    if let Some(g) = m.cfg.groups.get_mut(i) {
                        g.password = v;
                    }
                }),
                bind_slider(&reserved, window, cx, move |m, v| {
                    if let Some(g) = m.cfg.groups.get_mut(i) {
                        g.reserved_slots = v.round() as u32;
                    }
                }),
            ];
            self.groups_ui.push(GroupUi { name, pass, reserved, _subs: subs });
        }
    }

    /// Push the current config values into every input and slider.
    pub fn sync_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cfg = self.cfg.clone();
        let set = |e: &Entity<InputState>, v: String, window: &mut Window, cx: &mut Context<Self>| {
            e.update(cx, |s, cx| s.set_value(v, window, cx));
        };
        set(&self.name_in, cfg.name.clone(), window, cx);
        set(&self.save_in, cfg.save_directory.clone(), window, cx);
        set(&self.log_in, cfg.log_directory.clone(), window, cx);
        set(&self.ip_in, cfg.ip.clone(), window, cx);
        set(&self.port_in, cfg.query_port.to_string(), window, cx);
        self.slot_slider
            .update(cx, |s, cx| s.set_value(cfg.slot_count as f32, window, cx));
        self.sync_game_sliders(window, cx);
        self.build_group_ui(window, cx);
    }

    pub fn sync_game_sliders(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for (i, slider) in self.sliders.iter().enumerate() {
            if let Some(s) = slider {
                let v = self.cfg.game[i].num() as f32;
                s.update(cx, |s, cx| s.set_value(v, window, cx));
            }
        }
    }

    // ---- Load / save --------------------------------------------------------------------

    pub fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (cfg, load) = read_config(&self.path);
        self.baseline = cfg.to_value();
        self.cfg = cfg;
        self.load = load;
        self.disk_changed = false;
        self.disk_mtime = mtime(&self.path);
        self.backups = server::list_backups(&self.path);
        self.sync_controls(window, cx);
        cx.notify();
    }

    pub fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_derived();
        if !self.ready() {
            return;
        }
        let errors = self.error_count();
        if errors > 0 {
            window.push_notification(
                Notification::error(format!(
                    "Fix {errors} problem{} before saving.",
                    if errors == 1 { "" } else { "s" }
                ))
                .title("Cannot save"),
                cx,
            );
            return;
        }
        if !self.dirty && !matches!(self.load, Load::Missing) {
            return;
        }
        let text = self.cfg.to_json();
        let result = (|| -> anyhow::Result<bool> {
            let unchanged = std::fs::read_to_string(&self.path).map(|t| t == text).unwrap_or(false);
            let mut backed_up = false;
            if !unchanged {
                backed_up = server::make_backup(&self.path)?.is_some();
            }
            server::write_config(&self.path, &text)?;
            Ok(backed_up)
        })();
        match result {
            Ok(backed_up) => {
                self.baseline = self.cfg.to_value();
                self.load = Load::Ready;
                self.disk_changed = false;
                self.disk_mtime = mtime(&self.path);
                self.backups = server::list_backups(&self.path);
                let mut msg = String::from("Configuration written to disk.");
                if backed_up {
                    msg.push_str(" The previous file was backed up.");
                }
                if self.running {
                    msg.push_str(" Restart the server to apply it.");
                }
                window.push_notification(Notification::success(msg).title("Saved"), cx);
            }
            Err(e) => {
                let hint = if format!("{e:#}").to_lowercase().contains("denied") {
                    "\nTry running the manager as administrator, or move the server out of Program Files."
                } else {
                    ""
                };
                window.push_notification(
                    Notification::error(format!("{e:#}{hint}")).title("Save failed").autohide(false),
                    cx,
                );
            }
        }
        cx.notify();
    }

    pub fn backup_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match server::make_backup(&self.path) {
            Ok(Some(_)) => {
                self.backups = server::list_backups(&self.path);
                window.push_notification(Notification::success("Snapshot of the current file saved."), cx);
            }
            Ok(None) => window.push_notification(Notification::warning("There is no config file to back up yet."), cx),
            Err(e) => window.push_notification(Notification::error(format!("{e:#}")), cx),
        }
        cx.notify();
    }

    pub fn restore_backup(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        match server::read_config(path).and_then(|t| Config::parse(&t)) {
            Ok(cfg) => {
                self.cfg = cfg;
                self.sync_controls(window, cx);
                self.page = Page::Review;
                window.push_notification(
                    Notification::info("Backup loaded into the editor. Review it, then save to apply.")
                        .title("Backup loaded"),
                    cx,
                );
            }
            Err(e) => window.push_notification(Notification::error(format!("{e:#}")).title("Backup unreadable"), cx),
        }
        cx.notify();
    }

    // ---- Server process -----------------------------------------------------------------

    pub fn start_server(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match server::start(&self.path) {
            Ok(()) => {
                self.running = true;
                let extra = if self.dirty { " Unsaved changes are not included." } else { "" };
                window.push_notification(
                    Notification::success(format!("The server console is opening.{extra}")).title("Starting server"),
                    cx,
                );
            }
            Err(e) => window.push_notification(Notification::error(format!("{e:#}")).title("Could not start"), cx),
        }
        cx.notify();
    }

    pub fn stop_server(&mut self, force: bool, window: &mut Window, cx: &mut Context<Self>) {
        match server::stop(force) {
            Ok(()) => {
                self.stop_requested = Some(Instant::now());
                window.push_notification(
                    Notification::info(if force {
                        "The server process was terminated."
                    } else {
                        "Asked the server to shut down. It saves the world first."
                    })
                    .title("Stopping server"),
                    cx,
                );
            }
            Err(e) => window.push_notification(Notification::error(format!("{e:#}")).title("Could not stop"), cx),
        }
        cx.notify();
    }

    fn start_polling(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path = self.path.clone();
        cx.spawn_in(window, async move |this, cx| {
            loop {
                let p = path.clone();
                let (running, modified) = cx
                    .background_executor()
                    .spawn(async move { (server::is_running(), mtime(&p)) })
                    .await;
                let alive = this
                    .update_in(cx, |this, window, cx| this.on_poll(running, modified, window, cx))
                    .is_ok();
                if !alive {
                    break;
                }
                cx.background_executor().timer(Duration::from_secs(2)).await;
            }
        })
        .detach();
    }

    fn on_poll(&mut self, running: bool, modified: Option<SystemTime>, window: &mut Window, cx: &mut Context<Self>) {
        if running != self.running {
            self.running = running;
            if !running {
                self.stop_requested = None;
            }
            cx.notify();
        }
        if modified != self.disk_mtime {
            self.refresh_derived();
            if self.dirty {
                self.disk_changed = true;
                self.disk_mtime = modified;
            } else {
                let changed = match read_config(&self.path) {
                    (cfg, Load::Ready) => cfg.to_value() != self.baseline,
                    _ => true,
                };
                self.reload(window, cx);
                if changed {
                    window.push_notification(
                        Notification::info("The config file changed on disk, so it was reloaded.").title("Reloaded"),
                        cx,
                    );
                }
            }
            cx.notify();
        }
    }

    // ---- Confirmations ------------------------------------------------------------------

    pub fn ask(&mut self, c: Confirm, cx: &mut Context<Self>) {
        self.confirm = Some(c);
        cx.notify();
    }

    pub fn run_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(c) = self.confirm.take() else { return };
        match c {
            Confirm::Discard | Confirm::ReloadFromDisk => self.reload(window, cx),
            Confirm::DeleteGroup(i) => {
                if i < self.cfg.groups.len() {
                    self.cfg.groups.remove(i);
                    self.build_group_ui(window, cx);
                }
            }
            Confirm::Unban(i) => {
                if i < self.cfg.bans.len() {
                    self.cfg.bans.remove(i);
                }
            }
            Confirm::ForceStop => self.stop_server(true, window, cx),
            Confirm::StopServer => self.stop_server(false, window, cx),
            Confirm::Restore(p) => self.restore_backup(&p, window, cx),
            Confirm::DeleteBackup(p) => {
                let _ = std::fs::remove_file(&p);
                self.backups = server::list_backups(&self.path);
            }
            Confirm::ResetGameplay => {
                self.cfg.reset_game_defaults();
                self.sync_game_sliders(window, cx);
            }
            Confirm::QuitDiscard => {
                self.allow_close = true;
                window.remove_window();
            }
            Confirm::ReplaceBroken => {
                self.cfg = Config::fresh();
                self.sync_controls(window, cx);
                self.load = Load::Missing;
                self.save(window, cx);
            }
        }
        cx.notify();
    }
}
