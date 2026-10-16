//! Shroud-and-flame palette: deep indigo blacks with warm amber accents.

use gpui::{px, rgb, App, Hsla};
use gpui_component::{Theme, ThemeMode};

pub fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

pub const BG: u32 = 0x0e0d14;
pub const SIDEBAR: u32 = 0x0a090f;
pub const CARD: u32 = 0x15131f;
pub const CARD_HI: u32 = 0x1c1929;
pub const INSET: u32 = 0x100f18;
pub const BORDER: u32 = 0x282437;
pub const BORDER_HI: u32 = 0x3b3552;
pub const FG: u32 = 0xede9f6;
pub const MUTED: u32 = 0x8f89a6;
pub const DIM: u32 = 0x625d78;

pub const FLAME: u32 = 0xf2a93b;
pub const FLAME_HI: u32 = 0xffc065;
pub const FLAME_LO: u32 = 0xc67f1c;
pub const EMBER: u32 = 0xe8672f;
pub const SHROUD: u32 = 0x9d8fff;
pub const GOOD: u32 = 0x5ad49c;
pub const BAD: u32 = 0xf2636f;
pub const WARN: u32 = 0xf4b64e;

pub const HEADING_FONT: &str = "Georgia";

/// Install the palette on top of the component library's dark theme.
pub fn apply(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
    let t = Theme::global_mut(cx);
    t.radius = px(8.);
    t.radius_lg = px(12.);
    t.shadow = true;
    t.mono_font_family = "Cascadia Mono".into();

    let k = &mut t.colors;
    k.background = c(BG);
    k.foreground = c(FG);
    k.border = c(BORDER);
    k.input = c(BORDER_HI);
    k.muted = c(CARD_HI);
    k.muted_foreground = c(MUTED);
    k.caret = c(FLAME_HI);
    k.ring = c(FLAME).opacity(0.65);
    k.selection = c(FLAME).opacity(0.28);
    k.link = c(FLAME_HI);
    k.link_hover = c(FLAME_HI);
    k.link_active = c(FLAME);

    k.primary = c(FLAME);
    k.primary_hover = c(FLAME_HI);
    k.primary_active = c(FLAME_LO);
    k.primary_foreground = c(0x1b1205);

    k.secondary = c(CARD_HI);
    k.secondary_hover = c(0x262238);
    k.secondary_active = c(0x2d2943);
    k.secondary_foreground = c(FG);

    k.accent = c(0x262238);
    k.accent_foreground = c(FG);

    k.danger = c(BAD);
    k.danger_hover = c(0xf67e88);
    k.danger_active = c(0xd04a56);
    k.danger_foreground = c(0x1d0709);
    k.success = c(GOOD);
    k.success_hover = c(0x7ae0b0);
    k.success_active = c(0x3fb882);
    k.success_foreground = c(0x04170e);
    k.warning = c(WARN);
    k.warning_hover = c(0xf8c673);
    k.warning_active = c(0xd6973a);
    k.warning_foreground = c(0x1d1203);
    k.info = c(SHROUD);
    k.info_hover = c(0xb2a7ff);
    k.info_active = c(0x7f70e6);
    k.info_foreground = c(0x0c0a1f);

    k.popover = c(CARD_HI);
    k.popover_foreground = c(FG);
    k.list = c(CARD);
    k.list_hover = c(0x201d30);
    k.list_active = c(0x2a2540);
    k.list_active_border = c(FLAME);
    k.list_even = c(CARD);
    k.list_head = c(CARD);
    k.group_box = c(CARD);
    k.overlay = gpui::hsla(250. / 360., 0.4, 0.03, 0.72);

    k.slider_bar = c(FLAME);
    k.slider_thumb = c(FLAME_HI);
    k.switch = c(0x2f2b44);
    k.switch_thumb = c(0xf6f3ff);

    k.scrollbar = gpui::transparent_black();
    k.scrollbar_thumb = c(0x3a3552);
    k.scrollbar_thumb_hover = c(0x544c75);

    k.sidebar = c(SIDEBAR);
    k.sidebar_border = c(BORDER);
    k.sidebar_foreground = c(FG);
    k.sidebar_accent = c(0x1f1b2e);
    k.sidebar_accent_foreground = c(FG);
    k.sidebar_primary = c(FLAME);
    k.sidebar_primary_foreground = c(0x1b1205);

    k.title_bar = c(SIDEBAR);
    k.title_bar_border = c(BORDER);
    k.window_border = c(BORDER_HI);
    k.tab = c(0x00000000);
    k.tab_active = c(CARD_HI);
    k.tab_active_foreground = c(FG);
    k.tab_foreground = c(MUTED);
    k.tab_bar = c(INSET);
}
