use std::f32::consts::TAU;
use std::time::Duration;

use gpui::{
    anchored, black, deferred, div, point, prelude::*, px, rgb, white, Animation,
    AnimationExt as _, App, FontWeight, SharedString, Window,
};
use gpui_kit::component::ActiveTheme as _;

const GRID: usize = 6;
const GRID_SIZE: f32 = 120.0;
const GAP: f32 = 3.0;
const TILE: f32 = (GRID_SIZE - GAP * (GRID as f32 - 1.0)) / GRID as f32;
const CYCLE_SECS: f32 = 2.4;
const OVERLAY_PRIORITY: usize = 1000;

const COLORS: [u32; GRID * GRID] = [
    0xF97316, 0xFB923C, 0xFDBA74, 0xFED7AA, 0xF97316, 0xEA580C, //
    0xC2410C, 0xFB923C, 0xFDBA74, 0xF97316, 0xFB923C, 0xFED7AA, //
    0xEA580C, 0xFDBA74, 0xF97316, 0xC2410C, 0xFB923C, 0xF97316, //
    0xEA580C, 0xFDBA74, 0xFED7AA, 0xC2410C, 0xF97316, 0xFB923C, //
    0xF97316, 0xFDBA74, 0xEA580C, 0xFB923C, 0xFED7AA, 0xF97316, //
    0xC2410C, 0xFDBA74, 0xFB923C, 0xFED7AA, 0xF97316, 0xEA580C,
];

/// Keyframes of the scatter-and-reassemble loop: (time, scale, scatter
/// multiplier, opacity). Rotation is left out because gpui quads cannot be
/// rotated.
const KEYFRAMES: [(f32, f32, f32, f32); 7] = [
    (0.00, 1.0, 0.0, 1.0),
    (0.15, 0.6, 1.0, 0.4),
    (0.30, 0.2, 2.0, 0.1),
    (0.50, 0.2, -1.5, 0.1),
    (0.70, 0.6, -0.5, 0.5),
    (0.85, 1.1, 0.0, 1.0),
    (1.00, 1.0, 0.0, 1.0),
];

fn ease_in_out(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

/// (scale, scatter multiplier, opacity) at `phase` in `0..1`.
fn sample(phase: f32) -> (f32, f32, f32) {
    let phase = phase.clamp(0.0, 1.0);
    let end = KEYFRAMES
        .iter()
        .position(|k| k.0 >= phase)
        .unwrap_or(KEYFRAMES.len() - 1)
        .max(1);
    let (t0, s0, m0, o0) = KEYFRAMES[end - 1];
    let (t1, s1, m1, o1) = KEYFRAMES[end];
    let k = ease_in_out(((phase - t0) / (t1 - t0)).clamp(0.0, 1.0));
    (s0 + (s1 - s0) * k, m0 + (m1 - m0) * k, o0 + (o1 - o0) * k)
}

fn delay_secs(index: usize) -> f32 {
    (index / GRID) as f32 * 0.08 + (index % GRID) as f32 * 0.06
}

/// Stable pseudo-random value in `0..1`, so the scatter pattern does not
/// change between renders and no RNG dependency is needed.
fn unit_hash(index: usize, salt: u32) -> f32 {
    let mut x = (index as u32).wrapping_mul(0x9E37_79B1) ^ salt.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFFFF) as f32 / 65536.0
}

fn scatter(index: usize) -> (f32, f32) {
    let angle = index as f32 / (GRID * GRID) as f32 * TAU + unit_hash(index, 1) * 0.8;
    let distance = 30.0 + unit_hash(index, 2) * 50.0;
    (angle.cos() * distance, angle.sin() * distance)
}

type OnCancel = Box<dyn Fn(&mut Window, &mut App) + 'static>;

const CANCELLING_LABEL: &str = "キャンセル中...";

/// Full-window overlay that blocks the UI underneath while work runs.
#[derive(IntoElement)]
pub struct PixelLoader {
    label: SharedString,
    progress: Option<u32>,
    cancelling: bool,
    on_cancel: Option<OnCancel>,
}

impl PixelLoader {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            progress: None,
            cancelling: false,
            on_cancel: None,
        }
    }

    pub fn progress(mut self, percent: u32) -> Self {
        self.progress = Some(percent);
        self
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Box::new(handler));
        self
    }

    /// Waiting for the worker to stop: the label says so and a second press is
    /// pointless, so the button is disabled.
    pub fn cancelling(mut self, cancelling: bool) -> Self {
        self.cancelling = cancelling;
        self
    }
}

impl RenderOnce for PixelLoader {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let viewport = window.viewport_size();
        let mono = cx.theme().mono_font_family.clone();

        let tiles = (0..GRID * GRID).map(|i| {
            let (col, row) = ((i % GRID) as f32, (i / GRID) as f32);
            let center_x = col * (TILE + GAP) + TILE / 2.0;
            let center_y = row * (TILE + GAP) + TILE / 2.0;
            let (dx, dy) = scatter(i);
            let delay = delay_secs(i);

            div().absolute().with_animation(
                ("pixel", i),
                Animation::new(Duration::from_secs_f32(CYCLE_SECS)).repeat(),
                move |tile, t| {
                    let phase = (t - delay / CYCLE_SECS).rem_euclid(1.0);
                    let (scale, spread, opacity) = sample(phase);
                    let size = TILE * scale;
                    let x = center_x + dx * spread * scale - size / 2.0;
                    let y = center_y + dy * spread * scale - size / 2.0;
                    tile.left(px(x))
                        .top(px(y))
                        .w(px(size))
                        .h(px(size))
                        .rounded(px(3.0 * scale))
                        .bg(rgb(COLORS[i]))
                        .opacity(opacity)
                },
            )
        });

        deferred(
            anchored().position(point(px(0.), px(0.))).child(
                div()
                    .id("pixel-loader")
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(viewport.width)
                    .h(viewport.height)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(24.))
                    .bg(black().opacity(0.6))
                    .occlude()
                    .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .relative()
                            .w(px(GRID_SIZE))
                            .h(px(GRID_SIZE))
                            .children(tiles),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(white())
                            .child(if self.cancelling {
                                SharedString::from(CANCELLING_LABEL)
                            } else {
                                self.label
                            }),
                    )
                    .children(self.progress.map(|percent| {
                        div()
                            .text_size(px(12.))
                            .font_family(mono)
                            .text_color(white().opacity(0.6))
                            .child(format!("{percent}%"))
                    }))
                    .children(self.on_cancel.map(|handler| {
                        let disabled = self.cancelling;
                        div()
                            .id("pixel-loader-cancel")
                            .px(px(20.))
                            .h(px(36.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(white().opacity(0.6))
                            .text_size(px(14.))
                            .text_color(white())
                            .child("キャンセル")
                            .when(disabled, |this| this.opacity(0.4))
                            .when(!disabled, |this| {
                                this.cursor_pointer()
                                    .hover(|style| style.bg(white().opacity(0.15)))
                                    .on_click(move |_, window, cx| handler(window, cx))
                            })
                    })),
            ),
        )
        .with_priority(OVERLAY_PRIORITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_starts_and_ends_assembled() {
        assert_eq!(sample(0.0), (1.0, 0.0, 1.0));
        assert_eq!(sample(1.0), (1.0, 0.0, 1.0));
    }

    #[test]
    fn keyframe_times_reproduce_their_values() {
        for (t, scale, spread, opacity) in KEYFRAMES {
            let (s, m, o) = sample(t);
            assert!((s - scale).abs() < 1e-5, "scale at {t}");
            assert!((m - spread).abs() < 1e-5, "spread at {t}");
            assert!((o - opacity).abs() < 1e-5, "opacity at {t}");
        }
    }

    #[test]
    fn midway_between_keyframes_is_strictly_between_their_values() {
        let (scale, _, _) = sample(0.075);
        assert!(scale < 1.0 && scale > 0.6);
    }

    #[test]
    fn tile_delay_grows_along_row_and_column() {
        assert_eq!(delay_secs(0), 0.0);
        assert!((delay_secs(1) - 0.06).abs() < 1e-6);
        assert!((delay_secs(6) - 0.08).abs() < 1e-6);
        assert!((delay_secs(35) - (5.0 * 0.08 + 5.0 * 0.06)).abs() < 1e-5);
    }

    #[test]
    fn scatter_distance_stays_within_the_designed_range() {
        for i in 0..GRID * GRID {
            let (dx, dy) = scatter(i);
            let distance = (dx * dx + dy * dy).sqrt();
            assert!((30.0..=80.0).contains(&distance), "tile {i}: {distance}");
        }
    }
}
