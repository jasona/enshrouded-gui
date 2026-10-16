//! Declarative description of every `gameSettings` key the dedicated server understands.
//! Ranges, options and defaults come from `enshrouded_server_readme.txt` (v1.0.0.0).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Player,
    Survival,
    World,
    Progression,
    Combat,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Player,
        Category::Survival,
        Category::World,
        Category::Progression,
        Category::Combat,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Player => "Player",
            Category::Survival => "Survival",
            Category::World => "World & Time",
            Category::Progression => "Progression",
            Category::Combat => "Enemies & Bosses",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Category::Player => "How sturdy your adventurers are before the Shroud takes its toll.",
            Category::Survival => {
                "Hunger, durability, death penalties and how long you can endure the Shroud."
            }
            Category::World => {
                "Day length, weather, resources and the pace of the world around you."
            }
            Category::Progression => "Experience gain and the runes it costs to upgrade your gear.",
            Category::Combat => {
                "Enemy numbers, toughness and aggression, plus the bosses at the end of it."
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    /// A multiplier shown as a percentage (1.0 == 100%).
    Factor { min: f32, max: f32 },
    /// A duration stored by the server in nanoseconds, edited in whole minutes.
    Minutes { min: f32, max: f32 },
    /// A whole number.
    Int { min: f32, max: f32 },
    Bool,
    Choice(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug)]
pub enum Dflt {
    Num(f64),
    Bool(bool),
    Choice(&'static str),
}

pub struct Def {
    pub key: &'static str,
    pub label: &'static str,
    pub desc: &'static str,
    pub kind: Kind,
    pub default: Dflt,
    pub category: Category,
}

const fn f(min: f32, max: f32) -> Kind {
    Kind::Factor { min, max }
}

use Category::*;
use Dflt::{Bool as B, Choice as C, Num as N};

pub const DEFS: &[Def] = &[
    // ---- Player -------------------------------------------------------------------------
    Def { key: "playerHealthFactor", label: "Player health", desc: "Scales maximum health.", kind: f(0.25, 4.0), default: N(1.0), category: Player },
    Def { key: "playerManaFactor", label: "Player mana", desc: "Scales maximum mana.", kind: f(0.25, 4.0), default: N(1.0), category: Player },
    Def { key: "playerStaminaFactor", label: "Player stamina", desc: "Scales maximum stamina.", kind: f(0.25, 4.0), default: N(1.0), category: Player },
    Def { key: "playerBodyHeatFactor", label: "Body heat", desc: "How long a player endures freezing areas before hypothermia sets in.", kind: f(0.5, 2.0), default: N(1.0), category: Player },
    Def { key: "playerDivingTimeFactor", label: "Diving time", desc: "Initial oxygen, and so the time available underwater.", kind: f(0.5, 2.0), default: N(1.0), category: Player },
    // ---- Survival -----------------------------------------------------------------------
    Def { key: "enableDurability", label: "Weapon durability", desc: "When off, weapons never break.", kind: Kind::Bool, default: B(true), category: Survival },
    Def { key: "enableStarvingDebuff", label: "Hunger & starvation", desc: "Starving players lose health periodically until they eat or drink.", kind: Kind::Bool, default: B(false), category: Survival },
    Def { key: "fromHungerToStarving", label: "Hungry state duration", desc: "How long the hungry state lasts before starvation begins.", kind: Kind::Minutes { min: 5.0, max: 20.0 }, default: N(10.0), category: Survival },
    Def { key: "foodBuffDurationFactor", label: "Food buff duration", desc: "Scales how long food buffs last.", kind: f(0.5, 2.0), default: N(1.0), category: Survival },
    Def { key: "shroudTimeFactor", label: "Shroud time", desc: "Scales how long characters can remain inside the Shroud.", kind: f(0.5, 2.0), default: N(1.0), category: Survival },
    Def { key: "tombstoneMode", label: "Tombstone on death", desc: "What a dying player leaves behind in a recoverable tombstone.", kind: Kind::Choice(&["AddBackpackMaterials", "Everything", "NoTombstone"]), default: C("AddBackpackMaterials"), category: Survival },
    Def { key: "enableGliderTurbulences", label: "Glider turbulence", desc: "Air turbulence while gliding. Off restores the calm gliding of older versions.", kind: Kind::Bool, default: B(true), category: Survival },
    // ---- World & time -------------------------------------------------------------------
    Def { key: "dayTimeDuration", label: "Daytime length", desc: "Real-time minutes of daylight.", kind: Kind::Minutes { min: 2.0, max: 60.0 }, default: N(30.0), category: World },
    Def { key: "nightTimeDuration", label: "Nighttime length", desc: "Real-time minutes of night.", kind: Kind::Minutes { min: 2.0, max: 60.0 }, default: N(12.0), category: World },
    Def { key: "weatherFrequency", label: "Weather frequency", desc: "How often new weather phenomena appear.", kind: Kind::Choice(&["Disabled", "Rare", "Normal", "Often"]), default: C("Normal"), category: World },
    Def { key: "fishingDifficulty", label: "Fishing difficulty", desc: "Strength of the fish during the fishing minigame.", kind: Kind::Choice(&["VeryEasy", "Easy", "Normal", "Hard", "VeryHard"]), default: C("Normal"), category: World },
    Def { key: "miningDamageFactor", label: "Mining effectiveness", desc: "More terraforming and resource yield per hit.", kind: f(0.5, 2.0), default: N(1.0), category: World },
    Def { key: "plantGrowthSpeedFactor", label: "Plant growth speed", desc: "How quickly crops grow.", kind: f(0.25, 2.0), default: N(1.0), category: World },
    Def { key: "resourceDropStackAmountFactor", label: "Resource gain", desc: "Material amount per loot stack in chests, enemies and nodes.", kind: f(0.25, 2.0), default: N(1.0), category: World },
    Def { key: "factoryProductionSpeedFactor", label: "Workstation speed", desc: "Scales production times at workshops. Higher is faster.", kind: f(0.25, 2.0), default: N(1.0), category: World },
    Def { key: "curseModifier", label: "Shroud curse", desc: "Easy turns the curse off, Hard doubles the chance of receiving it.", kind: Kind::Choice(&["Easy", "Normal", "Hard"]), default: C("Normal"), category: World },
    Def { key: "flameLevelFogRemoval", label: "Flame level for Shroud altars", desc: "Flame level required before Flame Altars can be placed in the Shroud.", kind: Kind::Int { min: 0.0, max: 20.0 }, default: N(11.0), category: World },
    // ---- Progression --------------------------------------------------------------------
    Def { key: "experienceCombatFactor", label: "Combat experience", desc: "XP received from combat.", kind: f(0.25, 2.0), default: N(1.0), category: Progression },
    Def { key: "experienceMiningFactor", label: "Mining experience", desc: "XP received from mining resources.", kind: f(0.0, 2.0), default: N(1.0), category: Progression },
    Def { key: "experienceExplorationQuestsFactor", label: "Exploration experience", desc: "XP received from exploring and completing quests.", kind: f(0.25, 2.0), default: N(1.0), category: Progression },
    Def { key: "perkCostFactor", label: "Weapon upgrade cost", desc: "Runes required to upgrade weapons.", kind: f(0.25, 2.0), default: N(1.0), category: Progression },
    Def { key: "perkUpgradeRecyclingFactor", label: "Recycling yield", desc: "Share of runes returned when salvaging upgraded weapons.", kind: f(0.0, 1.0), default: N(0.5), category: Progression },
    // ---- Enemies & bosses ---------------------------------------------------------------
    Def { key: "randomSpawnerAmount", label: "Enemy amount", desc: "How many enemies populate the world.", kind: Kind::Choice(&["Few", "Normal", "Automatic", "Many", "Extreme"]), default: C("Automatic"), category: Combat },
    Def { key: "aggroPoolAmount", label: "Simultaneous attackers", desc: "How many enemies may attack at the same time.", kind: Kind::Choice(&["Few", "Normal", "Many", "Extreme"]), default: C("Normal"), category: Combat },
    Def { key: "enemyDamageFactor", label: "Enemy damage", desc: "Damage dealt by regular enemies. Bosses excluded.", kind: f(0.25, 5.0), default: N(1.0), category: Combat },
    Def { key: "enemyHealthFactor", label: "Enemy health", desc: "Health of regular enemies. Bosses excluded.", kind: f(0.25, 4.0), default: N(1.0), category: Combat },
    Def { key: "enemyStaminaFactor", label: "Enemy stun resistance", desc: "Higher stamina makes enemies take longer to stun.", kind: f(0.5, 2.0), default: N(1.0), category: Combat },
    Def { key: "enemyPerceptionRangeFactor", label: "Enemy perception", desc: "How far enemies can see and hear you.", kind: f(0.5, 2.0), default: N(1.0), category: Combat },
    Def { key: "threatBonus", label: "Enemy attack frequency", desc: "How often enemies attack. Bosses excluded.", kind: f(0.25, 4.0), default: N(1.0), category: Combat },
    Def { key: "pacifyAllEnemies", label: "Pacified enemies", desc: "Enemies stay passive until attacked. Bosses excluded.", kind: Kind::Bool, default: B(false), category: Combat },
    Def { key: "tamingStartleRepercussion", label: "Taming failure", desc: "What happens to taming progress when wildlife is startled.", kind: Kind::Choice(&["KeepProgress", "LoseSomeProgress", "LoseAllProgress"]), default: C("LoseSomeProgress"), category: Combat },
    Def { key: "bossDamageFactor", label: "Boss damage", desc: "Damage dealt by boss attacks.", kind: f(0.2, 5.0), default: N(1.0), category: Combat },
    Def { key: "bossHealthFactor", label: "Boss health", desc: "Health of bosses.", kind: f(0.2, 5.0), default: N(1.0), category: Combat },
];

pub const PRESETS: [(&str, &str); 5] = [
    ("Default", "The classic Enshrouded experience. Recommended for new players."),
    ("Relaxed", "Fewer enemies and more loot. Built for base builders and easy exploring."),
    ("Hard", "More enemies, more aggressive. A tougher combat experience."),
    ("Survival", "Aggressive enemies plus extra survival mechanics. Be prepared to suffer."),
    ("Custom", "Tune every individual setting yourself."),
];

pub const TAGS: &[&str] = &[
    "LookingForPlayers",
    "English",
    "German",
    "French",
    "Italian",
    "Japanese",
    "Korean",
    "Polish",
    "Portuguese",
    "Russian",
    "Spanish",
    "Thai",
    "Turkish",
    "Ukrainian",
    "Chinese",
    "Taiwanese",
    "BaseBuilding",
    "Exploration",
    "Roleplay",
];

/// "LookingForPlayers" -> "Looking For Players"
pub fn pretty(word: &str) -> String {
    let mut out = String::new();
    for (i, ch) in word.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

pub const NANOS_PER_MINUTE: f64 = 60_000_000_000.0;
