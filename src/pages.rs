//! The seven pages of the manager.

use gpui::{
    div, linear_color_stop, linear_gradient, prelude::*, px, AnyElement, ClipboardItem, Div,
    FontWeight, IntoElement, SharedString, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    notification::Notification,
    scroll::ScrollableElement as _,
    slider::Slider,
    switch::Switch,
    v_flex, Disableable as _, IconName, Sizable as _, WindowExt as _,
};
use serde_json::Value;

use crate::{
    app::{Confirm, Manager, Page},
    config::{self, Area, Level, UserGroup, Val},
    schema::{self, Category, Kind, DEFS, PRESETS, TAGS},
    server,
    shell::banner,
    theme::*,
    ui::*,
};

const MONO: &str = "Cascadia Mono";

fn category_style(cat: Category) -> (&'static str, u32) {
    match cat {
        Category::Player => ("shield", SHROUD),
        Category::Survival => ("flame", EMBER),
        Category::World => ("clock", GOOD),
        Category::Progression => ("key", FLAME),
        Category::Combat => ("swords", BAD),
    }
}

fn is_default(def: &schema::Def, val: &Val) -> bool {
    match (def.default, val) {
        (schema::Dflt::Num(d), Val::Num(v)) => (d - v).abs() < 1e-6,
        (schema::Dflt::Bool(d), Val::Bool(v)) => d == *v,
        (schema::Dflt::Choice(d), Val::Choice(v)) => d == v,
        _ => false,
    }
}

fn option_label(key: &str, opt: &str) -> String {
    match (key, opt) {
        ("tombstoneMode", "AddBackpackMaterials") => "Materials only".into(),
        ("tombstoneMode", "Everything") => "Everything".into(),
        ("tombstoneMode", "NoTombstone") => "Nothing".into(),
        ("tamingStartleRepercussion", "KeepProgress") => "Keep progress".into(),
        ("tamingStartleRepercussion", "LoseSomeProgress") => "Lose some".into(),
        ("tamingStartleRepercussion", "LoseAllProgress") => "Lose all".into(),
        _ => schema::pretty(opt),
    }
}

impl Manager {
    pub fn render_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if !self.ready() {
            return self.page_unreadable(cx).into_any_element();
        }
        match self.page {
            Page::Overview => self.page_overview(cx).into_any_element(),
            Page::Server => self.page_server(cx).into_any_element(),
            Page::Gameplay => self.page_gameplay(cx).into_any_element(),
            Page::Players => self.page_players(cx).into_any_element(),
            Page::Bans => self.page_bans(cx).into_any_element(),
            Page::Review => self.page_review(window, cx).into_any_element(),
            Page::Backups => self.page_backups(cx).into_any_element(),
        }
    }

    // ---- Small helpers ------------------------------------------------------------------

    fn toggle(
        &self,
        id: &str,
        value: bool,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Manager, bool) + 'static,
    ) -> Switch {
        let view = cx.entity();
        Switch::new(SharedString::from(id.to_string())).checked(value).on_click(move |v, _, cx| {
            let v = *v;
            view.update(cx, |m, cx| {
                set(m, v);
                cx.notify();
            });
        })
    }

    fn seg(
        &self,
        id: &str,
        labels: Vec<String>,
        selected: usize,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Manager, usize) + 'static,
    ) -> impl IntoElement {
        let view = cx.entity();
        segmented(id, labels, selected, move |i, _, cx| {
            view.update(cx, |m, cx| {
                set(m, i);
                cx.notify();
            });
        })
    }

    pub fn ban_label(&self, i: usize) -> String {
        let Some(b) = self.cfg.bans.get(i) else { return "This account".into() };
        let get = |k: &str| b.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
        get("displayName")
            .or_else(|| get("characterName"))
            .unwrap_or_else(|| "This account".to_string())
    }

    fn page_unreadable(&self, cx: &mut Context<Self>) -> impl IntoElement {
        card()
            .p_8()
            .gap_3()
            .items_center()
            .child(icon_tile("file", BAD, 52.))
            .child(heading("The config file can't be edited right now").text_xl())
            .child(
                muted("Fix the JSON by hand, or replace it with a fresh default config. A backup of the broken file is made first.")
                    .text_sm()
                    .max_w(px(520.)),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("u-reveal")
                            .outline()
                            .icon(IconName::FolderOpen)
                            .label("Show file")
                            .on_click(cx.listener(|this, _, _, _| server::reveal_file(&this.path))),
                    )
                    .child(
                        Button::new("u-retry")
                            .primary()
                            .icon(icon("reload"))
                            .label("Retry")
                            .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
                    ),
            )
    }

    // ---- Overview -----------------------------------------------------------------------

    fn page_overview(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let running = self.running;
        let force = self.running
            && self.stop_requested.is_some_and(|t| t.elapsed() > std::time::Duration::from_secs(8));

        let hero = div()
            .rounded_xl()
            .border_1()
            .border_color(c(BORDER_HI))
            .shadow_md()
            .bg(linear_gradient(
                120.,
                linear_color_stop(c(0x2a1f2c), 0.),
                linear_color_stop(c(0x17142a), 0.55),
            ))
            .child(
                h_flex()
                    .p_6()
                    .gap_6()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap_2()
                            .min_w_0()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(if running { pill("Running", GOOD) } else { pill("Stopped", MUTED) })
                                    .child(pill(format!("{} preset", self.cfg.preset), SHROUD)),
                            )
                            .child(
                                heading(if self.cfg.name.trim().is_empty() { "Unnamed server".to_string() } else { self.cfg.name.clone() })
                                    .text_size(px(32.))
                                    .line_height(gpui::relative(1.15))
                                    .truncate(),
                            )
                            .child(
                                muted(format!(
                                    "Up to {} players  ·  query port {}  ·  listening on {}",
                                    self.cfg.slot_count, self.cfg.query_port, self.cfg.ip
                                ))
                                .text_sm(),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_none()
                            .child(if running {
                                Button::new("o-stop")
                                    .outline()
                                    .icon(icon("stop"))
                                    .label("Stop")
                                    .on_click(cx.listener(|this, _, _, cx| this.ask(Confirm::StopServer, cx)))
                                    .into_any_element()
                            } else {
                                Button::new("o-start")
                                    .primary()
                                    .large()
                                    .icon(icon("play"))
                                    .label("Start server")
                                    .on_click(cx.listener(|this, _, window, cx| this.start_server(window, cx)))
                                    .into_any_element()
                            })
                            .when(force, |d| {
                                d.child(
                                    Button::new("o-force")
                                        .danger()
                                        .label("Force stop")
                                        .tooltip("The server is taking long to exit")
                                        .on_click(cx.listener(|this, _, _, cx| this.ask(Confirm::ForceStop, cx))),
                                )
                            }),
                    ),
            );

        let tile = |label: &str, value: String, sub: String, color: u32| {
            card()
                .flex_1()
                .min_w(px(150.))
                .p_4()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(color))
                        .child(label.to_uppercase()),
                )
                .child(div().text_size(px(26.)).font_weight(FontWeight::BOLD).line_height(gpui::relative(1.2)).child(value))
                .child(muted(sub).text_xs())
        };
        let reserved: u32 = self.cfg.groups.iter().map(|g| g.reserved_slots).sum();
        let stats = h_flex()
            .gap_4()
            .flex_wrap()
            .child(tile("Slots", self.cfg.slot_count.to_string(), format!("{reserved} reserved"), FLAME))
            .child(tile("Port", self.cfg.query_port.to_string(), "query port".into(), SHROUD))
            .child(tile("Groups", self.cfg.groups.len().to_string(), "password protected".into(), GOOD))
            .child(tile("Bans", self.cfg.bans.len().to_string(), "accounts barred".into(), BAD))
            .child(tile("Tags", self.cfg.tags.len().to_string(), "discovery filters".into(), EMBER));

        // Attention card.
        let issues: Vec<Div> = self
            .issues
            .iter()
            .enumerate()
            .map(|(i, issue)| {
                let color = if issue.level == Level::Error { BAD } else { WARN };
                let target = match issue.area {
                    Area::Server => Page::Server,
                    Area::Players => Page::Players,
                };
                h_flex()
                    .px_5()
                    .py_3()
                    .gap_3()
                    .items_center()
                    .child(dot(c(color)))
                    .child(div().flex_1().text_sm().child(issue.msg.clone()))
                    .child(
                        Button::new(SharedString::from(format!("fix-{i}")))
                            .ghost()
                            .small()
                            .label("Go")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.page = target;
                                cx.notify();
                            })),
                    )
            })
            .collect();
        let attention = card()
            .flex_1()
            .min_w(px(340.))
            .child(card_header(
                "shield",
                if issues.is_empty() { GOOD } else { WARN },
                "Health check",
                if issues.is_empty() { "No problems found." } else { "Things worth a second look." },
                None,
            ))
            .child(if issues.is_empty() {
                h_flex()
                    .px_5()
                    .py_5()
                    .gap_3()
                    .items_center()
                    .child(gpui_component::Icon::new(IconName::CircleCheck).size(px(20.)).text_color(c(GOOD)))
                    .child(muted("Names, ports and passwords all check out.").text_sm())
                    .into_any_element()
            } else {
                rows(issues).into_any_element()
            });

        // Locations card.
        let server_dir = server::server_dir(&self.path);
        let save_dir = server::resolve_dir(&self.path, &self.cfg.save_directory);
        let log_dir = server::resolve_dir(&self.path, &self.cfg.log_directory);
        let loc = |id: &'static str, label: &str, path: std::path::PathBuf| {
            let shown = path.display().to_string().replace('/', "\\");
            h_flex()
                .px_5()
                .py_3()
                .gap_3()
                .items_center()
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(label.to_string()))
                        .child(muted(shown).text_xs().truncate()),
                )
                .child(
                    Button::new(id)
                        .ghost()
                        .small()
                        .icon(IconName::FolderOpen)
                        .tooltip("Open in Explorer")
                        .on_click(move |_, _, _| server::open_folder(&path)),
                )
        };
        let locations = card()
            .flex_1()
            .min_w(px(340.))
            .child(card_header("file", SHROUD, "Locations", "Jump to the folders that matter.", None))
            .child(rows(vec![
                loc("l-server", "Server folder", server_dir),
                loc("l-save", "Save games", save_dir),
                loc("l-logs", "Logs", log_dir),
            ]));

        v_flex()
            .gap_5()
            .child(hero)
            .child(stats)
            .child(h_flex().gap_5().items_start().flex_wrap().child(attention).child(locations))
            .child(
                muted(format!(
                    "Players outside your network need port {} allowed through your firewall and forwarded on your router.",
                    self.cfg.query_port
                ))
                .text_xs(),
            )
    }

    // ---- Server -------------------------------------------------------------------------

    fn page_server(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let identity = card()
            .child(card_header("server", FLAME, "Identity", "How your server appears in the Join menu.", None))
            .child(
                v_flex()
                    .p_5()
                    .gap_5()
                    .child(field(
                        "Server name",
                        Some("Shown in the server list. Keep it friendly."),
                        Input::new(&self.name_in),
                    ))
                    .child(field(
                        "Player slots",
                        Some("The server supports between 1 and 16 simultaneous players."),
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(div().flex_1().child(Slider::new(&self.slot_slider)))
                            .child(
                                div()
                                    .w(px(96.))
                                    .text_right()
                                    .font_family(MONO)
                                    .text_color(c(FLAME_HI))
                                    .child(format!("{} players", self.cfg.slot_count)),
                            ),
                    )),
            );

        let network = card()
            .child(card_header("server", SHROUD, "Network", "Change these only if you know you need to.", None))
            .child(
                h_flex()
                    .p_5()
                    .gap_5()
                    .items_start()
                    .child(div().flex_1().child(field("IP address", Some("0.0.0.0 listens on every network interface."), Input::new(&self.ip_in))))
                    .child(div().w(px(200.)).child(field("Query port", Some("Default 15637."), Input::new(&self.port_in)))),
            );

        let dir_row = |label: &str, input: &gpui::Entity<gpui_component::input::InputState>, dir: std::path::PathBuf, id: &'static str| {
            field(
                label,
                None,
                h_flex().gap_2().child(div().flex_1().child(Input::new(input))).child(
                    Button::new(id)
                        .outline()
                        .icon(IconName::FolderOpen)
                        .tooltip("Open in Explorer")
                        .on_click(move |_, _, _| server::open_folder(&dir)),
                ),
            )
        };
        let storage = card()
            .child(card_header("archive", GOOD, "Storage", "Relative paths start in the server folder.", None))
            .child(
                v_flex()
                    .p_5()
                    .gap_5()
                    .child(dir_row("Save directory", &self.save_in, server::resolve_dir(&self.path, &self.cfg.save_directory), "d-save"))
                    .child(dir_row("Log directory", &self.log_in, server::resolve_dir(&self.path, &self.cfg.log_directory), "d-log")),
            );

        let voice_on = self.cfg.enable_voice_chat;
        let mode_idx = usize::from(self.cfg.voice_chat_mode == "Global");
        let chat = card()
            .child(card_header("users", EMBER, "Chat & voice", "Communication between players.", None))
            .child(rows(vec![
                row(
                    "Text chat",
                    "Let players type to each other in game.",
                    self.toggle("sw-text", self.cfg.enable_text_chat, cx, |m, v| m.cfg.enable_text_chat = v),
                    false,
                ),
                row(
                    "Voice chat",
                    "Switch voice communication on for the whole server.",
                    self.toggle("sw-voice", voice_on, cx, |m, v| m.cfg.enable_voice_chat = v),
                    false,
                ),
                row(
                    "Voice range",
                    "Proximity only carries to nearby players. Global reaches everyone on the server.",
                    div().opacity(if voice_on { 1.0 } else { 0.4 }).child(self.seg(
                        "seg-voice",
                        vec!["Proximity".into(), "Global".into()],
                        mode_idx,
                        cx,
                        |m, i| m.cfg.voice_chat_mode = if i == 0 { "Proximity".into() } else { "Global".into() },
                    )),
                    false,
                ),
            ]));

        let tag_chips = |tags: &[&'static str], cx: &mut Context<Self>| -> Vec<AnyElement> {
            tags.iter()
                .map(|tag| {
                    let on = self.cfg.tags.iter().any(|t| t == tag);
                    let view = cx.entity();
                    let tag = *tag;
                    chip(
                        SharedString::from(format!("tag-{tag}")),
                        &schema::pretty(tag),
                        on,
                        FLAME,
                        move |_, cx| {
                            view.update(cx, |m, cx| {
                                if let Some(p) = m.cfg.tags.iter().position(|t| t == tag) {
                                    m.cfg.tags.remove(p);
                                } else {
                                    m.cfg.tags.push(tag.to_string());
                                    m.cfg.tags.sort_by_key(|t| TAGS.iter().position(|x| x == t).unwrap_or(usize::MAX));
                                }
                                cx.notify();
                            })
                        },
                    )
                    .into_any_element()
                })
                .collect()
        };
        let (focus, langs): (Vec<&'static str>, Vec<&'static str>) = TAGS
            .iter()
            .partition(|t| matches!(**t, "LookingForPlayers" | "BaseBuilding" | "Exploration" | "Roleplay"));
        let discovery = card()
            .child(card_header(
                "flame",
                SHROUD,
                "Discovery tags",
                "Help players find a server that fits them when filtering the list.",
                Some(pill(format!("{} selected", self.cfg.tags.len()), FLAME).into_any_element()),
            ))
            .child(
                v_flex()
                    .p_5()
                    .gap_5()
                    .child(field("Focus", None, h_flex().gap_2().flex_wrap().children(tag_chips(&focus, cx))))
                    .child(field("Languages", None, h_flex().gap_2().flex_wrap().children(tag_chips(&langs, cx)))),
            );

        v_flex().gap_5().child(identity).child(network).child(storage).child(chat).child(discovery)
    }

    // ---- Gameplay -----------------------------------------------------------------------

    fn page_gameplay(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let custom = self.cfg.preset == "Custom";

        let presets = card()
            .child(card_header(
                "swords",
                FLAME,
                "Difficulty preset",
                "Pick a ready-made experience, or choose Custom to tune everything.",
                None,
            ))
            .child(
                h_flex().p_5().gap_3().flex_wrap().children(PRESETS.iter().map(|(name, desc)| {
                    let active = self.cfg.preset == *name;
                    let view = cx.entity();
                    let name_s = name.to_string();
                    v_flex()
                        .id(SharedString::from(format!("preset-{name}")))
                        .flex_1()
                        .min_w(px(170.))
                        .gap_1p5()
                        .p_4()
                        .rounded_lg()
                        .cursor_pointer()
                        .border_1()
                        .border_color(if active { c(FLAME).opacity(0.7) } else { c(BORDER) })
                        .bg(if active { c(FLAME).opacity(0.1) } else { c(INSET) })
                        .when(!active, |d| d.hover(|s| s.border_color(c(BORDER_HI)).bg(c(CARD_HI))))
                        .on_click(move |_, _, cx| {
                            let n = name_s.clone();
                            view.update(cx, |m, cx| {
                                m.cfg.preset = n;
                                cx.notify();
                            });
                        })
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    div()
                                        .font_family(HEADING_FONT)
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(if active { c(FLAME_HI) } else { c(FG) })
                                        .child(name.to_string()),
                                )
                                .when(active, |d| {
                                    d.child(gpui_component::Icon::new(IconName::CircleCheck).size(px(16.)).text_color(c(FLAME)))
                                }),
                        )
                        .child(muted(*desc).text_xs().line_height(gpui::relative(1.45)))
                })),
            );

        let notice = (!custom).then(|| {
            banner(
                SHROUD,
                &format!("The {} preset is active", self.cfg.preset),
                "The server uses the preset and ignores the individual values below. Switch to Custom to edit them.".to_string(),
                vec![Button::new("to-custom")
                    .small()
                    .primary()
                    .label("Switch to Custom")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.cfg.preset = "Custom".into();
                        cx.notify();
                    }))
                    .into_any_element()],
            )
        });

        // Category switcher with a count of modified settings.
        let labels: Vec<String> = Category::ALL
            .iter()
            .map(|cat| {
                let n = DEFS
                    .iter()
                    .enumerate()
                    .filter(|(i, d)| d.category == *cat && !is_default(d, &self.cfg.game[*i]))
                    .count();
                if n > 0 { format!("{}  ·  {n}", cat.label()) } else { cat.label().to_string() }
            })
            .collect();
        let selected = Category::ALL.iter().position(|c| *c == self.cat).unwrap_or(0);
        let switcher = self.seg("seg-cat", labels, selected, cx, |m, i| m.cat = Category::ALL[i]);

        let cat = self.cat;
        let (cat_icon, cat_color) = category_style(cat);
        let any_modified = DEFS
            .iter()
            .enumerate()
            .any(|(i, d)| d.category == cat && !is_default(d, &self.cfg.game[i]));
        let list_rows: Vec<Div> = DEFS
            .iter()
            .enumerate()
            .filter(|(_, d)| d.category == cat)
            .map(|(i, _)| self.setting_row(i, cx))
            .collect();

        let list = card()
            .relative()
            .child(card_header(
                cat_icon,
                cat_color,
                cat.label(),
                cat.blurb(),
                Some(
                    Button::new("reset-cat")
                        .ghost()
                        .small()
                        .icon(icon("rotate"))
                        .label("Reset category")
                        .disabled(!any_modified)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            for (i, d) in DEFS.iter().enumerate() {
                                if d.category == cat {
                                    this.cfg.game[i] = Val::from_default(d.default);
                                }
                            }
                            this.sync_game_sliders(window, cx);
                            cx.notify();
                        }))
                        .into_any_element(),
                ),
            ))
            .child(div().opacity(if custom { 1.0 } else { 0.4 }).child(rows(list_rows)))
            .when(!custom, |d| d.child(div().id("lock").absolute().inset_0().occlude().cursor_not_allowed()));

        v_flex()
            .gap_5()
            .child(presets)
            .children(notice)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap_3()
                    .flex_wrap()
                    .child(switcher)
                    .child(
                        Button::new("reset-all")
                            .ghost()
                            .small()
                            .label("Reset everything to defaults")
                            .disabled(!custom)
                            .on_click(cx.listener(|this, _, _, cx| this.ask(Confirm::ResetGameplay, cx))),
                    ),
            )
            .child(list)
    }

    fn setting_row(&self, i: usize, cx: &mut Context<Self>) -> Div {
        let def = &DEFS[i];
        let val = self.cfg.game[i].clone();
        let modified = !is_default(def, &val);

        let control: AnyElement = match def.kind {
            Kind::Bool => self
                .toggle(&format!("sw-{}", def.key), val.bool(), cx, move |m, v| m.cfg.game[i] = Val::Bool(v))
                .into_any_element(),
            Kind::Choice(options) => {
                let selected = options.iter().position(|o| *o == val.choice()).unwrap_or(0);
                let labels = options.iter().map(|o| option_label(def.key, o)).collect();
                self.seg(&format!("seg-{}", def.key), labels, selected, cx, move |m, n| {
                    m.cfg.game[i] = Val::Choice(options[n].to_string());
                })
                .into_any_element()
            }
            Kind::Factor { .. } | Kind::Minutes { .. } | Kind::Int { .. } => {
                let slider = self.sliders[i].clone().expect("numeric settings have sliders");
                let default = Val::from_default(def.default);
                h_flex()
                    .w(px(400.))
                    .gap_3()
                    .items_center()
                    .child(div().flex_1().child(Slider::new(&slider)))
                    .child(
                        div()
                            .w(px(76.))
                            .text_right()
                            .text_sm()
                            .font_family(MONO)
                            .text_color(if modified { c(FLAME_HI) } else { c(FG) })
                            .child(config::format_value(def.kind, &val)),
                    )
                    .child(
                        div().w(px(28.)).child(
                            Button::new(SharedString::from(format!("reset-{}", def.key)))
                                .ghost()
                                .small()
                                .icon(icon("rotate"))
                                .tooltip(format!("Reset to {}", config::format_value(def.kind, &default)))
                                .when(!modified, |b| b.invisible())
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.cfg.game[i] = Val::from_default(DEFS[i].default);
                                    this.sync_game_sliders(window, cx);
                                    cx.notify();
                                })),
                        ),
                    )
                    .into_any_element()
            }
        };

        row(def.label, def.desc, control, modified)
    }

    // ---- Players ------------------------------------------------------------------------

    fn page_players(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let reserved: u32 = self.cfg.groups.iter().map(|g| g.reserved_slots).sum();
        let top = h_flex()
            .gap_4()
            .items_center()
            .justify_between()
            .child(
                muted(format!(
                    "Players join by entering a group's password. {} group{}, {} of {} slots reserved.",
                    self.cfg.groups.len(),
                    if self.cfg.groups.len() == 1 { "" } else { "s" },
                    reserved,
                    self.cfg.slot_count
                ))
                .text_sm(),
            )
            .child(
                Button::new("add-group")
                    .primary()
                    .icon(IconName::Plus)
                    .label("Add group")
                    .on_click(cx.listener(|this, _, window, cx| {
                        let mut name = "New Group".to_string();
                        let mut n = 2;
                        while this.cfg.groups.iter().any(|g| g.name == name) {
                            name = format!("New Group {n}");
                            n += 1;
                        }
                        this.cfg.groups.push(UserGroup::new(&name));
                        this.build_group_ui(window, cx);
                        cx.notify();
                    })),
            );

        let cards: Vec<Div> = (0..self.cfg.groups.len().min(self.groups_ui.len()))
            .map(|i| self.group_card(i, cx))
            .collect();

        v_flex()
            .gap_5()
            .child(top)
            .children(cards)
            .when(self.cfg.groups.is_empty(), |d| {
                d.child(
                    card()
                        .p_8()
                        .items_center()
                        .gap_2()
                        .child(icon_tile("users", MUTED, 48.))
                        .child(heading("No user groups").text_lg())
                        .child(muted("Without a group nobody can join. Add one to get started.").text_sm()),
                )
            })
    }

    fn group_card(&self, i: usize, cx: &mut Context<Self>) -> Div {
        type Perm = (&'static str, fn(&UserGroup) -> bool, fn(&mut UserGroup, bool));
        const PERMS: [Perm; 5] = [
            ("Kick & ban", |g| g.can_kick_ban, |g, v| g.can_kick_ban = v),
            ("Open containers", |g| g.can_access_inventories, |g, v| g.can_access_inventories = v),
            ("Edit the world", |g| g.can_edit_world, |g, v| g.can_edit_world = v),
            ("Edit bases", |g| g.can_edit_base, |g, v| g.can_edit_base = v),
            ("Extend bases", |g| g.can_extend_base, |g, v| g.can_extend_base = v),
        ];

        let g = &self.cfg.groups[i];
        let ui = &self.groups_ui[i];
        let power = PERMS.iter().filter(|(_, get, _)| get(g)).count();
        let tint = match power {
            5 => FLAME,
            3..=4 => GOOD,
            1..=2 => SHROUD,
            _ => MUTED,
        };
        let password = g.password.clone();
        let pass_input = ui.pass.clone();

        let chips = PERMS.iter().enumerate().map(|(n, (label, get, set))| {
            let view = cx.entity();
            let set = *set;
            chip(SharedString::from(format!("perm-{i}-{n}")), label, get(g), GOOD, move |_, cx| {
                view.update(cx, |m, cx| {
                    if let Some(g) = m.cfg.groups.get_mut(i) {
                        let current = PERMS[n].1(g);
                        set(g, !current);
                    }
                    cx.notify();
                })
            })
        });

        card()
            .child(
                h_flex()
                    .px_5()
                    .py_4()
                    .gap_3()
                    .items_center()
                    .border_b_1()
                    .border_color(c(BORDER))
                    .child(icon_tile("shield", tint, 34.))
                    .child(div().w(px(280.)).child(Input::new(&ui.name)))
                    .child(div().flex_1())
                    .child(pill(format!("{power} of 5 permissions"), tint))
                    .child(
                        Button::new(SharedString::from(format!("del-group-{i}")))
                            .ghost()
                            .icon(icon("trash"))
                            .tooltip("Delete group")
                            .on_click(cx.listener(move |this, _, _, cx| this.ask(Confirm::DeleteGroup(i), cx))),
                    ),
            )
            .child(
                v_flex()
                    .p_5()
                    .gap_5()
                    .child(field(
                        "Password",
                        Some("Players type this when joining to receive this group's permissions. It must be unique."),
                        h_flex()
                            .gap_2()
                            .child(div().flex_1().child(Input::new(&ui.pass).mask_toggle()))
                            .child(
                                Button::new(SharedString::from(format!("gen-{i}")))
                                    .outline()
                                    .icon(icon("dice"))
                                    .tooltip("Generate a random password")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        let p = config::random_password();
                                        if let Some(ui) = this.groups_ui.get(i) {
                                            ui.pass.update(cx, |s, cx| s.set_value(p, window, cx));
                                        }
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("copy-{i}")))
                                    .outline()
                                    .icon(IconName::Copy)
                                    .tooltip("Copy password")
                                    .on_click(move |_, window, cx| {
                                        let live = pass_input.read(cx).value().to_string();
                                        let text = if live.is_empty() { password.clone() } else { live };
                                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                                        window.push_notification(Notification::success("Password copied to the clipboard."), cx);
                                    }),
                            ),
                    ))
                    .child(field(
                        "Permissions",
                        Some("Containers include chests, factories and collections inside bases. Extending bases covers placing, removing and upgrading Flame Altars."),
                        h_flex().gap_2().flex_wrap().children(chips),
                    ))
                    .child(field(
                        "Reserved slots",
                        Some("Keeps this many slots free so members can still join when the server is busy."),
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(div().flex_1().child(Slider::new(&ui.reserved)))
                            .child(
                                div()
                                    .w(px(40.))
                                    .text_right()
                                    .font_family(MONO)
                                    .text_color(if g.reserved_slots > 0 { c(FLAME_HI) } else { c(FG) })
                                    .child(g.reserved_slots.to_string()),
                            ),
                    )),
            )
    }

    // ---- Bans ---------------------------------------------------------------------------

    fn page_bans(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let rows_: Vec<Div> = self
            .cfg
            .bans
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let get = |k: &str| b.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
                let display = get("displayName").unwrap_or_else(|| "Unknown account".into());
                let character = get("characterName");
                let hash = get("accountIDHash").unwrap_or_default();
                let when = b
                    .get("banDate")
                    .and_then(Value::as_i64)
                    .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
                    .map(|d| d.with_timezone(&chrono::Local).format("%b %e, %Y").to_string());
                let short = if hash.len() > 14 { format!("{}…", &hash[..14]) } else { hash.clone() };

                h_flex()
                    .px_5()
                    .py_3p5()
                    .gap_4()
                    .items_center()
                    .child(icon_tile("ban", BAD, 36.))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(div().font_weight(FontWeight::MEDIUM).child(display))
                            .child(
                                muted(match character {
                                    Some(ch) => format!("Character: {ch}"),
                                    None => "No character recorded".into(),
                                })
                                .text_xs(),
                            ),
                    )
                    .children(when.map(|w| pill(format!("Banned {w}"), MUTED)))
                    .child(div().w(px(130.)).text_xs().font_family(MONO).text_color(c(DIM)).child(short))
                    .child(
                        Button::new(SharedString::from(format!("unban-{i}")))
                            .outline()
                            .small()
                            .label("Lift ban")
                            .on_click(cx.listener(move |this, _, _, cx| this.ask(Confirm::Unban(i), cx))),
                    )
            })
            .collect();

        let body = if rows_.is_empty() {
            card()
                .p_10()
                .items_center()
                .gap_2()
                .child(icon_tile("shield", GOOD, 52.))
                .child(heading("Nobody is banned").text_xl())
                .child(
                    muted("Players with kick and ban permission can ban accounts in game from the Social tab. They appear here.")
                        .text_sm()
                        .max_w(px(480.)),
                )
        } else {
            card()
                .child(card_header(
                    "ban",
                    BAD,
                    "Banned accounts",
                    "Identified by a hashed Steam account ID, so no personal data is stored.",
                    Some(pill(self.cfg.bans.len().to_string(), BAD).into_any_element()),
                ))
                .child(rows(rows_))
        };
        v_flex().gap_5().child(body)
    }

    // ---- Review -------------------------------------------------------------------------

    fn page_review(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let changes: Vec<Div> = self
            .changes
            .iter()
            .map(|ch| {
                let (tag, color) = match (&ch.from, &ch.to) {
                    (None, Some(_)) => ("added", GOOD),
                    (Some(_), None) => ("removed", BAD),
                    _ => ("changed", FLAME),
                };
                v_flex()
                    .px_5()
                    .py_3()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(pill(tag, color))
                            .child(div().flex_1().min_w_0().truncate().text_xs().font_family(MONO).text_color(c(MUTED)).child(ch.path.clone())),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .text_sm()
                            .font_family(MONO)
                            .flex_wrap()
                            .children(ch.from.clone().map(|f| div().text_color(c(DIM)).line_through().child(f)))
                            .children(ch.from.as_ref().zip(ch.to.as_ref()).map(|_| div().text_color(c(DIM)).child("→")))
                            .children(ch.to.clone().map(|t| div().text_color(c(color)).child(t))),
                    )
            })
            .collect();

        let left = card()
            .w(px(400.))
            .flex_none()
            .child(card_header(
                "diff",
                FLAME,
                "Pending changes",
                if changes.is_empty() { "Nothing differs from the file on disk." } else { "Compared with the file on disk." },
                (!changes.is_empty()).then(|| pill(changes.len().to_string(), FLAME).into_any_element()),
            ))
            .child(if changes.is_empty() {
                div()
                    .p_8()
                    .text_center()
                    .child(muted("You're all caught up.").text_sm())
                    .into_any_element()
            } else {
                rows(changes).into_any_element()
            });

        // Syntax-coloured JSON preview with passwords masked.
        let json = self.cfg.to_json();
        let lines: Vec<Div> = json.lines().map(json_line).collect();
        let copy_json = self.cfg.to_json();
        let right = card()
            .flex_1()
            .min_w_0()
            .child(card_header(
                "file",
                SHROUD,
                "enshrouded_server.json",
                "Exactly what will be written. Passwords are masked here.",
                Some(
                    Button::new("copy-json")
                        .ghost()
                        .small()
                        .icon(IconName::Copy)
                        .label("Copy")
                        .tooltip("Copies the real file contents, passwords included")
                        .on_click(move |_, window, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_json.clone()));
                            window.push_notification(Notification::success("JSON copied to the clipboard."), cx);
                        })
                        .into_any_element(),
                ),
            ))
            .child(
                div()
                    .id("json-scroll")
                    .h(px(640.))
                    .overflow_y_scrollbar()
                    .child(v_flex().p_5().gap_0p5().bg(c(INSET)).children(lines)),
            );

        let _ = cx;
        h_flex().gap_5().items_start().child(left).child(right)
    }

    // ---- Backups ------------------------------------------------------------------------

    fn page_backups(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let items: Vec<Div> = self
            .backups
            .iter()
            .map(|b| {
                let path = b.path.clone();
                let (p1, p2, p3) = (path.clone(), path.clone(), path.clone());
                h_flex()
                    .px_5()
                    .py_3p5()
                    .gap_4()
                    .items_center()
                    .child(icon_tile("clock", SHROUD, 36.))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(div().font_weight(FontWeight::MEDIUM).child(b.when.clone()))
                            .child(muted(format!("{}  ·  {}", b.file_name, server::human_size(b.size))).text_xs().font_family(MONO)),
                    )
                    .child(
                        Button::new(SharedString::from(format!("show-{}", b.file_name)))
                            .ghost()
                            .small()
                            .icon(IconName::FolderOpen)
                            .tooltip("Show in Explorer")
                            .on_click(move |_, _, _| server::reveal_file(&p1)),
                    )
                    .child(
                        Button::new(SharedString::from(format!("restore-{}", b.file_name)))
                            .outline()
                            .small()
                            .icon(icon("reload"))
                            .label("Restore")
                            .on_click(cx.listener(move |this, _, _, cx| this.ask(Confirm::Restore(p2.clone()), cx))),
                    )
                    .child(
                        Button::new(SharedString::from(format!("rm-{}", b.file_name)))
                            .ghost()
                            .small()
                            .icon(icon("trash"))
                            .tooltip("Delete snapshot")
                            .on_click(cx.listener(move |this, _, _, cx| this.ask(Confirm::DeleteBackup(p3.clone()), cx))),
                    )
            })
            .collect();

        let dir = server::backup_dir(&self.path);
        let count = items.len();
        card()
            .child(card_header(
                "archive",
                SHROUD,
                "Snapshots",
                "A copy of the file is stored before every save. The newest 30 are kept.",
                Some(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("open-backups")
                                .ghost()
                                .small()
                                .icon(IconName::FolderOpen)
                                .label("Open folder")
                                .on_click(move |_, _, _| server::open_folder(&dir)),
                        )
                        .child(
                            Button::new("backup-now")
                                .outline()
                                .small()
                                .icon(icon("save"))
                                .label("Back up now")
                                .on_click(cx.listener(|this, _, window, cx| this.backup_now(window, cx))),
                        )
                        .into_any_element(),
                ),
            ))
            .child(if count == 0 {
                div()
                    .p_10()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(icon_tile("archive", MUTED, 48.))
                    .child(heading("No snapshots yet").text_lg())
                    .child(muted("They appear here after your first save.").text_sm())
                    .into_any_element()
            } else {
                rows(items).into_any_element()
            })
    }
}

fn json_line(line: &str) -> Div {
    let indent = line.chars().take_while(|c| *c == '\t').count();
    let t = line.trim_start_matches('\t');

    let value_span = |v: &str| {
        let core = v.trim_end_matches(',');
        let color = if core.starts_with('"') {
            GOOD
        } else if core == "true" || core == "false" {
            EMBER
        } else if core.parse::<f64>().is_ok() {
            FLAME_HI
        } else {
            MUTED
        };
        div().text_color(c(color)).child(v.to_string())
    };

    let mut row = h_flex().pl(px(indent as f32 * 18.)).font_family(MONO).text_xs().whitespace_nowrap();
    if t.starts_with('"') {
        if let Some(pos) = t.find("\": ") {
            let key = &t[..pos + 1];
            let rest = &t[pos + 3..];
            let rest = if key == "\"password\"" {
                let comma = if rest.ends_with(',') { "," } else { "" };
                format!("\"••••••••\"{comma}")
            } else {
                rest.to_string()
            };
            row = row
                .child(div().text_color(c(SHROUD)).child(key.to_string()))
                .child(div().text_color(c(DIM)).child(": "))
                .child(value_span(&rest));
            return row;
        }
    }
    row.child(value_span(t))
}
