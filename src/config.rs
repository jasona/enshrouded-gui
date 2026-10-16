//! Typed view of `enshrouded_server.json`.
//!
//! The original JSON document is kept around and patched on save, so keys this app
//! does not know about (and the order of the file) survive a round trip untouched.

use std::net::IpAddr;

use anyhow::{Context as _, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::{Map, Number, Value};

use crate::schema::{self, Dflt, Kind, DEFS, NANOS_PER_MINUTE};

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Num(f64),
    Bool(bool),
    Choice(String),
}

impl Val {
    pub fn from_default(d: Dflt) -> Val {
        match d {
            Dflt::Num(n) => Val::Num(n),
            Dflt::Bool(b) => Val::Bool(b),
            Dflt::Choice(c) => Val::Choice(c.to_string()),
        }
    }

    pub fn num(&self) -> f64 {
        match self {
            Val::Num(n) => *n,
            _ => 0.0,
        }
    }

    pub fn bool(&self) -> bool {
        matches!(self, Val::Bool(true))
    }

    pub fn choice(&self) -> &str {
        match self {
            Val::Choice(c) => c,
            _ => "",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserGroup {
    pub name: String,
    pub password: String,
    pub can_kick_ban: bool,
    pub can_access_inventories: bool,
    pub can_edit_world: bool,
    pub can_edit_base: bool,
    pub can_extend_base: bool,
    pub reserved_slots: u32,
    /// Keys we do not model, preserved verbatim.
    pub extra: Map<String, Value>,
}

impl UserGroup {
    pub fn new(name: &str) -> Self {
        UserGroup {
            name: name.to_string(),
            password: random_password(),
            can_kick_ban: false,
            can_access_inventories: false,
            can_edit_world: true,
            can_edit_base: false,
            can_extend_base: false,
            reserved_slots: 0,
            extra: Map::new(),
        }
    }

    fn from_value(v: &Value) -> UserGroup {
        let o = v.as_object().cloned().unwrap_or_default();
        let mut extra = o.clone();
        let mut take = |k: &str| extra.shift_remove(k);
        let name = take("name").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
        let password = take("password").and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
        let mut flag = |k: &str| take(k).and_then(|v| v.as_bool()).unwrap_or(false);
        let can_kick_ban = flag("canKickBan");
        let can_access_inventories = flag("canAccessInventories");
        let can_edit_world = flag("canEditWorld");
        let can_edit_base = flag("canEditBase");
        let can_extend_base = flag("canExtendBase");
        let reserved_slots = extra
            .shift_remove("reservedSlots")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        UserGroup {
            name,
            password,
            can_kick_ban,
            can_access_inventories,
            can_edit_world,
            can_edit_base,
            can_extend_base,
            reserved_slots,
            extra,
        }
    }

    fn to_value(&self) -> Value {
        let mut m = Map::new();
        m.insert("name".into(), self.name.clone().into());
        m.insert("password".into(), self.password.clone().into());
        m.insert("canKickBan".into(), self.can_kick_ban.into());
        m.insert("canAccessInventories".into(), self.can_access_inventories.into());
        m.insert("canEditWorld".into(), self.can_edit_world.into());
        m.insert("canEditBase".into(), self.can_edit_base.into());
        m.insert("canExtendBase".into(), self.can_extend_base.into());
        m.insert("reservedSlots".into(), self.reserved_slots.into());
        for (k, v) in &self.extra {
            m.insert(k.clone(), v.clone());
        }
        Value::Object(m)
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub name: String,
    pub save_directory: String,
    pub log_directory: String,
    pub ip: String,
    pub query_port: u32,
    pub slot_count: u32,
    pub tags: Vec<String>,
    pub voice_chat_mode: String,
    pub enable_voice_chat: bool,
    pub enable_text_chat: bool,
    pub preset: String,
    /// One entry per [`DEFS`] row, in the same order.
    pub game: Vec<Val>,
    pub groups: Vec<UserGroup>,
    pub bans: Vec<Value>,
    raw: Map<String, Value>,
}

fn s(o: &Map<String, Value>, k: &str, d: &str) -> String {
    o.get(k).and_then(Value::as_str).unwrap_or(d).to_string()
}

fn b(o: &Map<String, Value>, k: &str, d: bool) -> bool {
    o.get(k).and_then(Value::as_bool).unwrap_or(d)
}

fn u(o: &Map<String, Value>, k: &str, d: u32) -> u32 {
    o.get(k).and_then(Value::as_u64).map(|n| n as u32).unwrap_or(d)
}

/// Whole numbers are written as integers (`1`, not `1.0`), like the game does.
pub fn number(n: f64) -> Value {
    let n = (n * 10_000.0).round() / 10_000.0;
    if n.fract() == 0.0 && n.abs() < 9.0e15 {
        Value::Number(Number::from(n as i64))
    } else {
        Number::from_f64(n).map(Value::Number).unwrap_or(Value::Null)
    }
}

impl Config {
    pub fn parse(text: &str) -> Result<Config> {
        let value: Value = serde_json::from_str(text).context("the file is not valid JSON")?;
        let raw = match value {
            Value::Object(o) => o,
            _ => anyhow::bail!("the top level of the file must be a JSON object"),
        };

        let game_obj = raw.get("gameSettings").and_then(Value::as_object).cloned().unwrap_or_default();
        let game = DEFS
            .iter()
            .map(|d| match (d.kind, game_obj.get(d.key)) {
                (Kind::Bool, Some(v)) => Val::Bool(v.as_bool().unwrap_or(matches!(d.default, Dflt::Bool(true)))),
                (Kind::Choice(_), Some(v)) => v
                    .as_str()
                    .map(|c| Val::Choice(c.to_string()))
                    .unwrap_or_else(|| Val::from_default(d.default)),
                (Kind::Minutes { .. }, Some(v)) => v
                    .as_f64()
                    .map(|n| Val::Num(n / NANOS_PER_MINUTE))
                    .unwrap_or_else(|| Val::from_default(d.default)),
                (_, Some(v)) => v
                    .as_f64()
                    .map(Val::Num)
                    .unwrap_or_else(|| Val::from_default(d.default)),
                (_, None) => Val::from_default(d.default),
            })
            .collect();

        let groups = raw
            .get("userGroups")
            .and_then(Value::as_array)
            .map(|a| a.iter().map(UserGroup::from_value).collect())
            .unwrap_or_default();
        let bans = raw
            .get("bannedAccounts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let tags = raw
            .get("tags")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|t| t.as_str().map(str::to_string)).collect())
            .unwrap_or_default();

        Ok(Config {
            name: s(&raw, "name", "Enshrouded Server"),
            save_directory: s(&raw, "saveDirectory", "./savegame"),
            log_directory: s(&raw, "logDirectory", "./logs"),
            ip: s(&raw, "ip", "0.0.0.0"),
            query_port: u(&raw, "queryPort", 15637),
            slot_count: u(&raw, "slotCount", 16),
            tags,
            voice_chat_mode: s(&raw, "voiceChatMode", "Proximity"),
            enable_voice_chat: b(&raw, "enableVoiceChat", false),
            enable_text_chat: b(&raw, "enableTextChat", false),
            preset: s(&raw, "gameSettingsPreset", "Default"),
            game,
            groups,
            bans,
            raw,
        })
    }

    /// A brand new config, like the server generates on first launch, with random passwords.
    pub fn fresh() -> Config {
        let mut cfg = Config::parse(DEFAULT_JSON).expect("embedded default config is valid");
        for g in &mut cfg.groups {
            g.password = random_password();
        }
        cfg
    }

    pub fn to_value(&self) -> Value {
        let mut root = self.raw.clone();
        root.insert("name".into(), self.name.clone().into());
        root.insert("saveDirectory".into(), self.save_directory.clone().into());
        root.insert("logDirectory".into(), self.log_directory.clone().into());
        root.insert("ip".into(), self.ip.clone().into());
        root.insert("queryPort".into(), self.query_port.into());
        root.insert("slotCount".into(), self.slot_count.into());
        root.insert("tags".into(), Value::Array(self.tags.iter().cloned().map(Value::from).collect()));
        root.insert("voiceChatMode".into(), self.voice_chat_mode.clone().into());
        root.insert("enableVoiceChat".into(), self.enable_voice_chat.into());
        root.insert("enableTextChat".into(), self.enable_text_chat.into());
        root.insert("gameSettingsPreset".into(), self.preset.clone().into());

        let mut game = root
            .get("gameSettings")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (def, val) in DEFS.iter().zip(&self.game) {
            let v = match (def.kind, val) {
                (Kind::Minutes { .. }, Val::Num(n)) => Value::Number(Number::from((n * NANOS_PER_MINUTE).round() as u64)),
                (_, Val::Num(n)) => number(*n),
                (_, Val::Bool(x)) => Value::Bool(*x),
                (_, Val::Choice(c)) => Value::String(c.clone()),
            };
            game.insert(def.key.into(), v);
        }
        root.insert("gameSettings".into(), Value::Object(game));
        root.insert(
            "userGroups".into(),
            Value::Array(self.groups.iter().map(UserGroup::to_value).collect()),
        );
        root.insert("bannedAccounts".into(), Value::Array(self.bans.clone()));
        Value::Object(root)
    }

    /// Pretty JSON with tab indentation, matching what the game itself writes.
    pub fn to_json(&self) -> String {
        to_pretty(&self.to_value())
    }

    pub fn reset_game_defaults(&mut self) {
        self.game = DEFS.iter().map(|d| Val::from_default(d.default)).collect();
    }

    pub fn validate(&self) -> Vec<Issue> {
        let mut out = Vec::new();
        let mut err = |area, msg: String| out.push(Issue { level: Level::Error, area, msg });
        let mut issues: Vec<(Level, Area, String)> = Vec::new();
        let mut warn = |area, msg: String| issues.push((Level::Warning, area, msg));

        if self.name.trim().is_empty() {
            err(Area::Server, "The server needs a name.".into());
        }
        if self.save_directory.trim().is_empty() {
            err(Area::Server, "The save directory is empty.".into());
        }
        if self.log_directory.trim().is_empty() {
            err(Area::Server, "The log directory is empty.".into());
        }
        if self.ip.trim().parse::<IpAddr>().is_err() {
            err(Area::Server, format!("\"{}\" is not a valid IP address.", self.ip));
        }
        if self.query_port == 0 || self.query_port > 65535 {
            err(Area::Server, "The query port must be between 1 and 65535.".into());
        } else if self.query_port < 1024 {
            warn(Area::Server, "Ports below 1024 are usually reserved by the system.".into());
        }
        if !(1..=16).contains(&self.slot_count) {
            err(Area::Server, "Slot count must be between 1 and 16.".into());
        }

        for (i, g) in self.groups.iter().enumerate() {
            let label = if g.name.trim().is_empty() { format!("Group {}", i + 1) } else { format!("\"{}\"", g.name) };
            if g.name.trim().is_empty() {
                err(Area::Players, format!("{label} has no name."));
            }
            if g.password.is_empty() {
                err(Area::Players, format!("{label} has no password, so nobody could join with it."));
            } else if g.password.chars().count() < 8 {
                warn(Area::Players, format!("{label} has a short password."));
            }
            for (j, other) in self.groups.iter().enumerate().skip(i + 1) {
                if !g.name.is_empty() && g.name == other.name {
                    err(Area::Players, format!("Two groups are both named \"{}\".", g.name));
                }
                if !g.password.is_empty() && g.password == other.password {
                    err(
                        Area::Players,
                        format!(
                            "Groups \"{}\" and \"{}\" share a password, so the server cannot tell them apart.",
                            g.name,
                            self.groups[j].name
                        ),
                    );
                }
            }
        }
        if self.groups.is_empty() {
            warn(Area::Players, "No user groups: nobody will be able to join.".into());
        } else if !self.groups.iter().any(|g| g.can_kick_ban) {
            warn(Area::Players, "No group can kick or ban players.".into());
        }
        let reserved: u32 = self.groups.iter().map(|g| g.reserved_slots).sum();
        if reserved > self.slot_count {
            warn(
                Area::Players,
                format!("{reserved} reserved slots exceed the {} available slots.", self.slot_count),
            );
        }

        out.extend(issues.into_iter().map(|(level, area, msg)| Issue { level, area, msg }));
        out.sort_by_key(|i| i.level != Level::Error);
        out
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Error,
    Warning,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Area {
    Server,
    Players,
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub level: Level,
    pub area: Area,
    pub msg: String,
}

pub fn random_password() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(14)
        .map(char::from)
        .collect()
}

pub fn to_pretty(v: &Value) -> String {
    use serde::Serialize as _;
    let mut buf = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    v.serialize(&mut ser).expect("serializing a Value cannot fail");
    let text = String::from_utf8(buf).expect("serde_json emits UTF-8");

    // The game writes empty arrays as "[" newline "]"; mirror that so saves produce quiet diffs.
    let mut out = String::with_capacity(text.len() + 32);
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let body = line.strip_suffix(',').unwrap_or(line);
        if let Some(head) = body.strip_suffix("[]") {
            let indent: String = line.chars().take_while(|c| *c == '\t').collect();
            out.push_str(head);
            out.push('[');
            out.push('\n');
            out.push_str(&indent);
            out.push(']');
            if line.ends_with(',') {
                out.push(',');
            }
        } else {
            out.push_str(line);
        }
    }
    out
}

// ---- Change summary ----------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Change {
    pub path: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

fn flatten(v: &Value, prefix: &str, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) => {
            for (k, v) in m {
                let p = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                flatten(v, &p, out);
            }
        }
        Value::Array(a) if a.iter().all(Value::is_object) && !a.is_empty() => {
            for (i, item) in a.iter().enumerate() {
                let tag = item
                    .get("name")
                    .or_else(|| item.get("accountIDHash"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| i.to_string());
                flatten(item, &format!("{prefix}[{tag}]"), out);
            }
        }
        Value::Array(a) => {
            let items: Vec<String> = a.iter().map(|x| x.as_str().map(str::to_string).unwrap_or_else(|| x.to_string())).collect();
            out.push((prefix.to_string(), if items.is_empty() { "none".into() } else { items.join(", ") }));
        }
        Value::String(s) => out.push((prefix.to_string(), s.clone())),
        other => out.push((prefix.to_string(), other.to_string())),
    }
}

pub fn diff(old: &Value, new: &Value) -> Vec<Change> {
    let (mut a, mut b) = (Vec::new(), Vec::new());
    flatten(old, "", &mut a);
    flatten(new, "", &mut b);
    let mask = |path: &str, v: String| {
        if path.ends_with(".password") { "••••••••".to_string() } else { v }
    };
    let mut out = Vec::new();
    for (k, nv) in &b {
        match a.iter().find(|(ak, _)| ak == k) {
            Some((_, ov)) if ov == nv => {}
            Some((_, ov)) => {
                // Mask both sides, but still show *that* a password changed.
                out.push(Change { path: k.clone(), from: Some(mask(k, ov.clone())), to: Some(mask(k, nv.clone())) })
            }
            None => out.push(Change { path: k.clone(), from: None, to: Some(mask(k, nv.clone())) }),
        }
    }
    for (k, ov) in &a {
        if !b.iter().any(|(bk, _)| bk == k) {
            out.push(Change { path: k.clone(), from: Some(mask(k, ov.clone())), to: None });
        }
    }
    out
}

/// Human friendly rendering of a setting value, used in the UI.
pub fn format_value(kind: Kind, v: &Val) -> String {
    match (kind, v) {
        (Kind::Factor { .. }, Val::Num(n)) => format!("{}%", (n * 100.0).round() as i64),
        (Kind::Minutes { .. }, Val::Num(n)) => {
            if n.fract() == 0.0 { format!("{} min", *n as i64) } else { format!("{n:.1} min") }
        }
        (Kind::Int { .. }, Val::Num(n)) => format!("{}", n.round() as i64),
        (_, Val::Bool(true)) => "On".into(),
        (_, Val::Bool(false)) => "Off".into(),
        (_, Val::Choice(c)) => schema::pretty(c),
        (_, Val::Num(n)) => n.to_string(),
    }
}

const DEFAULT_JSON: &str = r#"{
	"name": "Enshrouded Server",
	"saveDirectory": "./savegame",
	"logDirectory": "./logs",
	"ip": "0.0.0.0",
	"queryPort": 15637,
	"slotCount": 16,
	"tags": [],
	"voiceChatMode": "Proximity",
	"enableVoiceChat": false,
	"enableTextChat": false,
	"gameSettingsPreset": "Default",
	"gameSettings": {
		"playerHealthFactor": 1,
		"playerManaFactor": 1,
		"playerStaminaFactor": 1,
		"playerBodyHeatFactor": 1,
		"playerDivingTimeFactor": 1,
		"enableDurability": true,
		"enableStarvingDebuff": false,
		"foodBuffDurationFactor": 1,
		"fromHungerToStarving": 600000000000,
		"shroudTimeFactor": 1,
		"tombstoneMode": "AddBackpackMaterials",
		"enableGliderTurbulences": true,
		"weatherFrequency": "Normal",
		"fishingDifficulty": "Normal",
		"miningDamageFactor": 1,
		"plantGrowthSpeedFactor": 1,
		"resourceDropStackAmountFactor": 1,
		"factoryProductionSpeedFactor": 1,
		"perkUpgradeRecyclingFactor": 0.5,
		"perkCostFactor": 1,
		"experienceCombatFactor": 1,
		"experienceMiningFactor": 1,
		"experienceExplorationQuestsFactor": 1,
		"randomSpawnerAmount": "Automatic",
		"aggroPoolAmount": "Normal",
		"enemyDamageFactor": 1,
		"enemyHealthFactor": 1,
		"enemyStaminaFactor": 1,
		"enemyPerceptionRangeFactor": 1,
		"bossDamageFactor": 1,
		"bossHealthFactor": 1,
		"threatBonus": 1,
		"pacifyAllEnemies": false,
		"tamingStartleRepercussion": "LoseSomeProgress",
		"dayTimeDuration": 1800000000000,
		"nightTimeDuration": 720000000000,
		"curseModifier": "Normal",
		"flameLevelFogRemoval": 11
	},
	"userGroups": [
		{ "name": "Admin", "password": "x", "canKickBan": true, "canAccessInventories": true, "canEditWorld": true, "canEditBase": true, "canExtendBase": true, "reservedSlots": 0 },
		{ "name": "Friend", "password": "x", "canKickBan": false, "canAccessInventories": true, "canEditWorld": true, "canEditBase": true, "canExtendBase": false, "reservedSlots": 0 },
		{ "name": "Guest", "password": "x", "canKickBan": false, "canAccessInventories": false, "canEditWorld": true, "canEditBase": false, "canExtendBase": false, "reservedSlots": 0 },
		{ "name": "Visitor", "password": "x", "canKickBan": false, "canAccessInventories": false, "canEditWorld": false, "canEditBase": false, "canExtendBase": false, "reservedSlots": 0 }
	],
	"bannedAccounts": []
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../tests/sample.json");

    #[test]
    fn roundtrip_preserves_values() {
        let cfg = Config::parse(SAMPLE).unwrap();
        let again = Config::parse(&cfg.to_json()).unwrap();
        let (a, b) = (cfg.to_value(), again.to_value());
        assert_eq!(a, b);
        // Semantically identical to the original file.
        let original: Value = serde_json::from_str(SAMPLE).unwrap();
        assert!(diff(&original, &a).is_empty(), "{:?}", diff(&original, &a));
    }

    #[test]
    fn durations_use_nanoseconds() {
        let cfg = Config::parse(SAMPLE).unwrap();
        let day = DEFS.iter().position(|d| d.key == "dayTimeDuration").unwrap();
        assert_eq!(cfg.game[day], Val::Num(30.0));
        let v = cfg.to_value();
        assert_eq!(v["gameSettings"]["dayTimeDuration"].as_u64(), Some(1_800_000_000_000));
    }

    #[test]
    fn unknown_keys_survive() {
        let mut v: Value = serde_json::from_str(SAMPLE).unwrap();
        v["somethingNew"] = Value::from("kept");
        v["userGroups"][0]["futureFlag"] = Value::from(true);
        let cfg = Config::parse(&v.to_string()).unwrap();
        let out = cfg.to_value();
        assert_eq!(out["somethingNew"], "kept");
        assert_eq!(out["userGroups"][0]["futureFlag"], true);
    }

    #[test]
    fn output_matches_the_games_own_formatting() {
        let cfg = Config::parse(SAMPLE).unwrap();
        let ours = cfg.to_json();
        let theirs = SAMPLE.replace("0.500000", "0.5").replace("\r\n", "\n");
        assert_eq!(ours.trim_end(), theirs.trim_end());
    }

    #[test]
    fn fresh_config_is_valid() {
        let cfg = Config::fresh();
        assert!(cfg.validate().iter().all(|i| i.level != Level::Error));
    }

    #[test]
    fn duplicate_passwords_are_errors() {
        let mut cfg = Config::fresh();
        cfg.groups[1].password = cfg.groups[0].password.clone();
        assert!(cfg.validate().iter().any(|i| i.level == Level::Error));
    }
}
