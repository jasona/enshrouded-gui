//! Small stateless building blocks shared by every page.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, Div, FontWeight, Hsla, IntoElement, SharedString, Window,
};
use gpui_component::{h_flex, v_flex, Icon};

use crate::theme::*;

pub fn icon(name: &str) -> Icon {
    Icon::empty().path(format!("icons/em/{name}.svg"))
}

pub fn card() -> Div {
    v_flex()
        .bg(c(CARD))
        .border_1()
        .border_color(c(BORDER))
        .rounded_xl()
        .shadow_sm()
}

pub fn heading(text: impl Into<SharedString>) -> Div {
    div()
        .font_family(HEADING_FONT)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(FG))
        .child(text.into())
}

pub fn muted(text: impl Into<SharedString>) -> Div {
    div().text_color(c(MUTED)).child(text.into())
}

/// A tinted rounded square holding an icon.
pub fn icon_tile(name: &str, color: u32, size: f32) -> Div {
    div()
        .flex_none()
        .size(px(size))
        .rounded_lg()
        .flex()
        .items_center()
        .justify_center()
        .bg(c(color).opacity(0.14))
        .border_1()
        .border_color(c(color).opacity(0.28))
        .child(icon(name).size(px(size * 0.5)).text_color(c(color)))
}

pub fn pill(label: impl Into<SharedString>, color: u32) -> Div {
    div()
        .flex_none()
        .px_2()
        .py_0p5()
        .rounded_full()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(color))
        .bg(c(color).opacity(0.13))
        .border_1()
        .border_color(c(color).opacity(0.3))
        .child(label.into())
}

pub fn dot(color: Hsla) -> Div {
    div().flex_none().size(px(8.)).rounded_full().bg(color)
}

/// Card header: icon tile, title and subtitle, optional trailing element.
pub fn card_header(
    icon_name: &str,
    color: u32,
    title: &str,
    subtitle: &str,
    trailing: Option<AnyElement>,
) -> Div {
    h_flex()
        .px_5()
        .py_4()
        .gap_3()
        .items_center()
        .border_b_1()
        .border_color(c(BORDER))
        .child(icon_tile(icon_name, color, 34.))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .child(div().font_weight(FontWeight::SEMIBOLD).text_color(c(FG)).child(title.to_string()))
                .child(muted(subtitle.to_string()).text_xs()),
        )
        .children(trailing)
}

/// One labelled setting: text on the left, control on the right.
pub fn row(
    label: impl Into<SharedString>,
    desc: impl Into<SharedString>,
    control: impl IntoElement,
    modified: bool,
) -> Div {
    h_flex()
        .px_5()
        .py_3p5()
        .gap_6()
        .items_center()
        .justify_between()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0p5()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).text_color(c(FG)).child(label.into()))
                        .when(modified, |d| d.child(dot(c(FLAME)))),
                )
                .child(muted(desc).text_xs().line_height(gpui::relative(1.45))),
        )
        .child(div().flex_none().child(control))
}

/// Stack rows with hairline separators.
pub fn rows(items: Vec<Div>) -> Div {
    let n = items.len();
    v_flex().children(items.into_iter().enumerate().map(|(i, r)| {
        if i + 1 < n {
            r.border_b_1().border_color(c(BORDER).opacity(0.7))
        } else {
            r
        }
    }))
}

/// A label above a control, with an optional hint below.
pub fn field(label: &str, hint: Option<&str>, control: impl IntoElement) -> Div {
    v_flex()
        .gap_1p5()
        .child(div().text_sm().font_weight(FontWeight::MEDIUM).text_color(c(FG)).child(label.to_string()))
        .child(control)
        .children(hint.map(|h| muted(h.to_string()).text_xs()))
}

/// Segmented control. `on_select` receives the clicked index.
pub fn segmented(
    id: &str,
    labels: Vec<String>,
    selected: usize,
    on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let on_select = Rc::new(on_select);
    h_flex()
        .p_1()
        .gap_1()
        .rounded_lg()
        .bg(c(INSET))
        .border_1()
        .border_color(c(BORDER))
        .children(labels.into_iter().enumerate().map(|(i, label)| {
            let on = on_select.clone();
            let active = i == selected;
            div()
                .id(SharedString::from(format!("{id}-{i}")))
                .px_3()
                .py_1()
                .rounded_md()
                .text_sm()
                .cursor_pointer()
                .border_1()
                .border_color(if active { c(FLAME).opacity(0.55) } else { gpui::transparent_black() })
                .bg(if active { c(FLAME).opacity(0.14) } else { gpui::transparent_black() })
                .text_color(if active { c(FLAME_HI) } else { c(MUTED) })
                .font_weight(if active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                .when(!active, |d| d.hover(|s| s.text_color(c(FG)).bg(c(CARD_HI))))
                .on_click(move |_, window, cx| on(i, window, cx))
                .child(label)
        }))
}

/// A toggleable chip with a check mark when on.
pub fn chip(
    id: SharedString,
    label: &str,
    active: bool,
    color: u32,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    h_flex()
        .id(id)
        .gap_1p5()
        .items_center()
        .px_3()
        .py_1p5()
        .rounded_full()
        .text_sm()
        .cursor_pointer()
        .border_1()
        .border_color(if active { c(color).opacity(0.6) } else { c(BORDER_HI) })
        .bg(if active { c(color).opacity(0.14) } else { c(INSET) })
        .text_color(if active { c(color) } else { c(MUTED) })
        .when(!active, |d| d.hover(|s| s.text_color(c(FG)).border_color(c(MUTED))))
        .when(active, |d| d.child(gpui_component::Icon::new(gpui_component::IconName::Check).size(px(13.))))
        .child(label.to_string())
        .on_click(move |_, window, cx| on_click(window, cx))
}
