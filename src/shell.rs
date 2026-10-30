//! Window chrome: title bar, navigation, page header, banners and the confirm modal.

use gpui::{
    div, linear_color_stop, linear_gradient, prelude::*, px, AnyElement, Div, FontWeight, IntoElement,
    SharedString, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::ScrollableElement as _,
    v_flex, Disableable as _, Root, Sizable as _, TitleBar,
};

use crate::{
    app::{Confirm, Load, Manager, Page, SaveConfig},
    config::{Area, Level},
    theme::*,
    ui::*,
};

impl Render for Manager {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.refresh_derived();

        let sig = (self.page, self.cat, self.cfg.groups.len(), self.cfg.preset == "Custom");
        if self.last_sig != Some(sig) {
            self.last_sig = Some(sig);
            let view = cx.entity();
            window.on_next_frame(move |_, cx| view.update(cx, |_, cx| cx.notify()));
        }
        let modal = self.confirm.clone().map(|c| self.render_confirm(c, cx));

        div()
            .id("app")
            .key_context("Manager")
            .on_action(cx.listener(|this, _: &SaveConfig, window, cx| this.save(window, cx)))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(c(BG))
            .text_color(c(FG))
            .text_size(px(14.))
            .child(
                TitleBar::new().child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(icon("flame").size(px(15.)).text_color(c(FLAME)))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(c(MUTED))
                                .child("Enshrouded Server Manager"),
                        ),
                ),
            )
            .child(
                // Focus is tracked here and not on the root. A focus-tracking element calls
                // prevent_default on mouse down, which on Windows stops the title bar from
                // being dragged and its caption buttons from working.
                h_flex()
                    .track_focus(&self.focus)
                    .flex_1()
                    .min_h_0()
                    .child(self.render_sidebar(cx))
                    .child(self.render_main(window, cx)),
            )
            .children(modal)
            .children(
                Root::render_notification_layer(window, cx)
                    .map(|layer| div().absolute().top(px(46.)).right_0().child(layer)),
            )
    }
}

impl Manager {
    // ---- Sidebar ------------------------------------------------------------------------

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(240.))
            .flex_none()
            .h_full()
            .bg(c(SIDEBAR))
            .border_r_1()
            .border_color(c(BORDER))
            .p_4()
            .gap_1()
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .pb_5()
                    .px_1()
                    .child(
                        div()
                            .flex_none()
                            .size(px(40.))
                            .rounded_xl()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(linear_gradient(
                                145.,
                                linear_color_stop(c(FLAME_HI), 0.),
                                linear_color_stop(c(EMBER), 1.),
                            ))
                            .shadow_md()
                            .child(icon("flame").size(px(22.)).text_color(c(0x24120a))),
                    )
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(c(FLAME))
                                    .child("ENSHROUDED"),
                            )
                            .child(heading("Server Manager").text_lg().line_height(gpui::relative(1.15))),
                    ),
            )
            .children(Page::ALL.iter().map(|p| self.nav_item(*p, cx)))
            .child(div().flex_1())
            .child(self.sidebar_status(cx))
    }

    fn nav_item(&self, page: Page, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.page == page;
        let count_for = |area: Area| {
            let errors = self.issues.iter().filter(|i| i.area == area && i.level == Level::Error).count();
            let warns = self.issues.iter().filter(|i| i.area == area && i.level == Level::Warning).count();
            if errors > 0 {
                Some((errors.to_string(), BAD))
            } else if warns > 0 {
                Some((warns.to_string(), WARN))
            } else {
                None
            }
        };
        let badge = match page {
            Page::Server => count_for(Area::Server),
            Page::Players => count_for(Area::Players),
            Page::Review if !self.changes.is_empty() => Some((self.changes.len().to_string(), FLAME)),
            Page::Bans if !self.cfg.bans.is_empty() => Some((self.cfg.bans.len().to_string(), MUTED)),
            Page::Backups if !self.backups.is_empty() => Some((self.backups.len().to_string(), MUTED)),
            _ => None,
        };

        h_flex()
            .id(SharedString::from(format!("nav-{page:?}")))
            .gap_3()
            .items_center()
            .px_3()
            .py_2p5()
            .rounded_lg()
            .cursor_pointer()
            .bg(if active { c(FLAME).opacity(0.11) } else { gpui::transparent_black() })
            .text_color(if active { c(FLAME_HI) } else { c(MUTED) })
            .when(!active, |d| d.hover(|s| s.bg(c(0x181525)).text_color(c(FG))))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.page = page;
                cx.notify();
            }))
            .child(icon(page.icon()).size(px(17.)))
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                    .child(page.title()),
            )
            .children(badge.map(|(text, color)| pill(text, color)))
    }

    fn sidebar_status(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (color, label) = if self.running { (GOOD, "Server running") } else { (DIM, "Server stopped") };
        v_flex()
            .gap_3()
            .p_3()
            .rounded_xl()
            .bg(c(CARD))
            .border_1()
            .border_color(c(BORDER))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(dot(c(color)))
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(label)),
            )
            .child(
                v_flex()
                    .gap_0p5()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c(FG))
                            .truncate()
                            .child(file_name(&self.path)),
                    )
                    .child(muted(parent_name(&self.path)).text_xs().truncate()),
            )
            .child(if self.running {
                Button::new("side-stop")
                    .small()
                    .w_full()
                    .outline()
                    .icon(icon("stop"))
                    .label("Stop server")
                    .on_click(cx.listener(|this, _, _, cx| this.ask(Confirm::StopServer, cx)))
                    .into_any_element()
            } else {
                Button::new("side-start")
                    .small()
                    .w_full()
                    .primary()
                    .icon(icon("play"))
                    .label("Start server")
                    .on_click(cx.listener(|this, _, window, cx| this.start_server(window, cx)))
                    .into_any_element()
            })
    }

    // ---- Main column --------------------------------------------------------------------

    fn render_main(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let errors = self.error_count();
        let ready = self.ready();
        let body = self.render_page(window, cx);

        let status = if !ready {
            pill("Cannot read file", BAD)
        } else if self.dirty {
            pill(format!("{} unsaved change{}", self.changes.len(), if self.changes.len() == 1 { "" } else { "s" }), FLAME)
        } else if matches!(self.load, Load::Missing) {
            pill("New file", SHROUD)
        } else {
            pill("All changes saved", GOOD)
        };

        v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(
                h_flex()
                    .px_8()
                    .pt_6()
                    .pb_4()
                    .gap_4()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(heading(self.page.title()).text_size(px(30.)).line_height(gpui::relative(1.1)))
                            .child(muted(self.page.subtitle()).text_sm()),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(status)
                            .when(self.dirty, |d| {
                                d.child(
                                    Button::new("revert")
                                        .ghost()
                                        .icon(icon("rotate"))
                                        .label("Revert")
                                        .tooltip("Discard all unsaved changes")
                                        .on_click(cx.listener(|this, _, _, cx| this.ask(Confirm::Discard, cx))),
                                )
                            })
                            .child(
                                Button::new("save")
                                    .primary()
                                    .icon(icon("save"))
                                    .label("Save changes")
                                    .disabled(!ready || (!self.dirty && !matches!(self.load, Load::Missing)) || errors > 0)
                                    .tooltip(if errors > 0 { "Fix the problems first" } else { "Write to disk  (Ctrl+S)" })
                                    .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                            ),
                    ),
            )
            .child(self.render_banners(cx))
            .child(
                div()
                    .id(SharedString::from(format!("scroll-{:?}", self.page)))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .px_8()
                            .pt_2()
                            .pb_10()
                            .child(v_flex().w_full().max_w(px(1040.)).gap_5().child(body)),
                    ),
            )
    }

    fn render_banners(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut banners: Vec<AnyElement> = Vec::new();

        if let Load::Broken(msg) = &self.load {
            banners.push(
                banner(
                    BAD,
                    "Config file could not be read",
                    format!("{msg}. Editing is paused so nothing is overwritten by accident."),
                    vec![
                        Button::new("b-retry")
                            .small()
                            .outline()
                            .label("Retry")
                            .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx)))
                            .into_any_element(),
                        Button::new("b-replace")
                            .small()
                            .danger()
                            .label("Replace with defaults")
                            .on_click(cx.listener(|this, _, _, cx| this.ask(Confirm::ReplaceBroken, cx)))
                            .into_any_element(),
                    ],
                )
                .into_any_element(),
            );
        }
        if matches!(self.load, Load::Missing) {
            banners.push(
                banner(
                    SHROUD,
                    "No config file yet",
                    "Nothing exists at the configured path. Review the defaults below, then save to create the file with fresh random passwords.".to_string(),
                    vec![],
                )
                .into_any_element(),
            );
        }
        if self.disk_changed {
            banners.push(
                banner(
                    WARN,
                    "The file changed on disk",
                    "Something else modified the config while you had unsaved edits. Saving will overwrite those changes.".to_string(),
                    vec![Button::new("b-disk")
                        .small()
                        .outline()
                        .label("Load disk version")
                        .on_click(cx.listener(|this, _, _, cx| this.ask(Confirm::ReloadFromDisk, cx)))
                        .into_any_element()],
                )
                .into_any_element(),
            );
        }
        if self.running && self.page != Page::Overview {
            banners.push(
                banner(
                    WARN,
                    "The server is running",
                    "Saved changes only apply after a restart, and a running server may rewrite its own config on shutdown.".to_string(),
                    vec![],
                )
                .into_any_element(),
            );
        }

        v_flex().px_8().gap_3().when(!banners.is_empty(), |d| d.pb_3()).children(banners)
    }

    // ---- Confirm modal ------------------------------------------------------------------

    fn render_confirm(&self, confirm: Confirm, cx: &mut Context<Self>) -> impl IntoElement {
        let (title, body, label, danger): (&str, String, &str, bool) = match &confirm {
            Confirm::Discard => ("Discard changes?", "All unsaved edits will be lost and the file will be reloaded from disk.".into(), "Discard", true),
            Confirm::ReloadFromDisk => ("Load the disk version?", "Your unsaved edits will be replaced with what is on disk now.".into(), "Load from disk", true),
            Confirm::DeleteGroup(i) => (
                "Delete this user group?",
                format!(
                    "\"{}\" will be removed. Players using its password will no longer be able to join.",
                    self.cfg.groups.get(*i).map(|g| g.name.as_str()).unwrap_or("")
                ),
                "Delete group",
                true,
            ),
            Confirm::Unban(i) => (
                "Lift this ban?",
                format!(
                    "{} will be able to join again once you save and restart the server.",
                    self.ban_label(*i)
                ),
                "Lift ban",
                false,
            ),
            Confirm::ForceStop => ("Force the server to stop?", "The process will be killed immediately. Progress since the last autosave is lost.".into(), "Force stop", true),
            Confirm::StopServer => ("Stop the server?", "Connected players are disconnected. The server saves the world before it exits.".into(), "Stop server", true),
            Confirm::Restore(_) => ("Load this backup?", "It replaces what is in the editor. Nothing is written until you save.".into(), "Load backup", false),
            Confirm::DeleteBackup(_) => ("Delete this backup?", "The snapshot file is removed permanently.".into(), "Delete", true),
            Confirm::ResetGameplay => ("Reset all gameplay settings?", "Every individual setting returns to its default value.".into(), "Reset all", true),
            Confirm::QuitDiscard => ("Quit with unsaved changes?", "You have edits that were not saved. Closing now discards them.".into(), "Discard and quit", true),
            Confirm::ReplaceBroken => ("Replace the unreadable file?", "The broken file is backed up first, then a fresh default config with new random passwords is written.".into(), "Replace", true),
        };

        div()
            .id("modal-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::hsla(250. / 360., 0.45, 0.03, 0.74))
            .on_click(cx.listener(|this, _, _, cx| {
                this.confirm = None;
                cx.notify();
            }))
            .child(
                v_flex()
                    .id("modal")
                    .w(px(450.))
                    .p_6()
                    .gap_4()
                    .rounded_xl()
                    .bg(c(CARD_HI))
                    .border_1()
                    .border_color(c(BORDER_HI))
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(heading(title).text_xl())
                    .child(muted(body).text_sm().line_height(gpui::relative(1.5)))
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_end()
                            .child(
                                Button::new("c-cancel")
                                    .ghost()
                                    .label("Cancel")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.confirm = None;
                                        cx.notify();
                                    })),
                            )
                            .child({
                                let b = Button::new("c-ok").label(label);
                                let b = if danger { b.danger() } else { b.primary() };
                                b.on_click(cx.listener(|this, _, window, cx| this.run_confirm(window, cx)))
                            }),
                    ),
            )
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

fn parent_name(path: &std::path::Path) -> String {
    path.parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Full-width notice with a coloured accent.
pub fn banner(color: u32, title: &str, body: String, actions: Vec<AnyElement>) -> Div {
    h_flex()
        .gap_3()
        .items_center()
        .px_4()
        .py_3()
        .rounded_xl()
        .bg(c(color).opacity(0.09))
        .border_1()
        .border_color(c(color).opacity(0.32))
        .child(dot(c(color)))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0p5()
                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(c(color)).child(title.to_string()))
                .child(muted(body).text_xs().line_height(gpui::relative(1.45))),
        )
        .children(actions)
}
