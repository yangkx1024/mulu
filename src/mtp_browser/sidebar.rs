use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::animation::{Transition, ease_in_out_cubic};
use gpui_component::progress::Progress;
use gpui_component::*;

use super::MtpBrowser;
use crate::format::format_size;

impl MtpBrowser {
    pub(super) fn render_sidebar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let collapsed = self.collapsed;
        let expanded_w = px(240.);

        let prev_col = window.use_keyed_state("sidebar-prev-col", cx, |_, _| collapsed);
        let anim_widths = window.use_keyed_state("sidebar-anim-w", cx, |_, _| {
            let w = if collapsed { px(0.) } else { expanded_w };
            (w, w)
        });
        if *prev_col.read(cx) != collapsed {
            let (from, to) = if collapsed {
                (expanded_w, px(0.))
            } else {
                (px(0.), expanded_w)
            };
            anim_widths.update(cx, |v, _| *v = (from, to));
            prev_col.update(cx, |v, _| *v = collapsed);
        }
        let (from_w, to_w) = *anim_widths.read(cx);

        let session_location = self.session.as_ref().map(|s| s.device_location);
        let mut device_rows: Vec<AnyElement> = Vec::new();

        for device in &self.devices {
            let location_id = device.location_id;
            let label = device.label.clone();
            let is_active = session_location == Some(location_id);

            let row = div()
                .id(ElementId::Integer(location_id))
                .w_full()
                .px_2()
                .py_1p5()
                .cursor_pointer()
                .hover(|s| s.bg(cx.theme().sidebar_accent.opacity(0.4)))
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .text_sm()
                        .font_medium()
                        .text_color(cx.theme().foreground)
                        .child(label),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.open_device(location_id, cx);
                }))
                .into_any_element();

            device_rows.push(row);

            if is_active {
                if let Some(session) = &self.session {
                    let active_storage = session.client.active();
                    for storage in &session.storages {
                        let storage_id = storage.id;
                        let storage_name = storage.description.clone();
                        let is_storage_active = storage_id == active_storage;
                        let free = storage.free_bytes;
                        let max = storage.max_bytes;

                        let progress_val = if max > 0 {
                            ((max - free) as f64 / max as f64 * 100.0) as f32
                        } else {
                            0.0
                        };

                        let free_str: SharedString =
                            format!("{} free of {}", format_size(free), format_size(max)).into();

                        let storage_row = v_flex()
                            .id(ElementId::Integer(storage_id.0 as u64))
                            .mx_2()
                            .px_2()
                            .py_1p5()
                            .cursor_pointer()
                            .rounded(cx.theme().radius)
                            .when(is_storage_active, |s| s.bg(cx.theme().sidebar_accent))
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .items_center()
                                    .text_sm()
                                    .when(is_storage_active, |s| {
                                        s.font_medium()
                                            .text_color(cx.theme().sidebar_accent_foreground)
                                    })
                                    .when(!is_storage_active, |s| {
                                        s.text_color(cx.theme().foreground)
                                    })
                                    .child(Icon::new(IconName::HardDrive).size_3p5().text_color(
                                        if is_storage_active {
                                            cx.theme().blue
                                        } else {
                                            cx.theme().muted_foreground
                                        },
                                    ))
                                    .child(storage_name.clone()),
                            )
                            .when(is_storage_active, |this| {
                                this.child(
                                    v_flex()
                                        .gap_1()
                                        .pt_2()
                                        .pb_1()
                                        .child(
                                            Progress::new("storage-progress")
                                                .value(progress_val)
                                                .color(cx.theme().blue),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(free_str),
                                        ),
                                )
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_storage(storage_id, storage_name.clone(), cx);
                            }))
                            .into_any_element();

                        device_rows.push(storage_row);
                    }
                }
            }
        }

        let sidebar_content = v_flex()
            .w(expanded_w)
            .h_full()
            .border_r_1()
            .border_color(cx.theme().sidebar_border)
            .bg(cx.theme().sidebar)
            .pt_3()
            .pb_2()
            .child(
                div()
                    .px_2()
                    .pb_1()
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().muted_foreground)
                    .child("Devices"),
            )
            .children(device_rows);

        let sidebar_wrapper = div()
            .id("sidebar-wrapper")
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .child(sidebar_content);

        Transition::new(Duration::from_millis(200))
            .ease(ease_in_out_cubic)
            .width(from_w, to_w)
            .apply(
                sidebar_wrapper,
                ElementId::NamedInteger("sidebar-w".into(), collapsed as u64),
            )
            .into_any_element()
    }
}
