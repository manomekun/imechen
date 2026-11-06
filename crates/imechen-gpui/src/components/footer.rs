use gpui::{div, prelude::*, px, App, SharedString, Window};

use super::{GradientButton, ProgressBar};
use crate::theme::Tokens;

#[derive(Clone, Copy, Debug, Default)]
pub struct BatchState {
    pub file_count: usize,
    pub converting: bool,
    pub progress: u32,
    pub result_count: usize,
    pub error_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchStatus {
    pub text: String,
    pub error: bool,
    pub show_progress: bool,
}

pub fn batch_status(s: BatchState) -> BatchStatus {
    let show_progress = s.converting || s.progress > 0;
    let (text, error) = if s.converting {
        (format!("変換中... {}%", s.progress), false)
    } else if s.progress > 0 {
        match (s.error_count, s.result_count) {
            (0, _) => ("完了".to_string(), false),
            (errors, 0) => (format!("失敗 ({errors}件)"), true),
            (errors, _) => (format!("完了 ({errors}件失敗)"), true),
        }
    } else {
        (format!("{} ファイル", s.file_count), false)
    };
    BatchStatus {
        text,
        error,
        show_progress,
    }
}

type OnCta = Box<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Footer {
    status: BatchStatus,
    progress: u32,
    cta_label: SharedString,
    cta_disabled: bool,
    tokens: Tokens,
    on_cta: Option<OnCta>,
}

impl Footer {
    pub fn new(
        status: BatchStatus,
        progress: u32,
        cta_label: impl Into<SharedString>,
        tokens: Tokens,
    ) -> Self {
        Self {
            status,
            progress,
            cta_label: cta_label.into(),
            cta_disabled: false,
            tokens,
            on_cta: None,
        }
    }

    pub fn cta_disabled(mut self, disabled: bool) -> Self {
        self.cta_disabled = disabled;
        self
    }

    pub fn on_cta(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cta = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Footer {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let t = self.tokens;
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(24.))
            .pt(px(16.))
            .pr(px(18.))
            .pb(px(16.))
            .pl(px(24.))
            .bg(t.bg_footer)
            .border_t_1()
            .border_color(t.border)
            .child(
                div()
                    .flex_1()
                    .max_w(px(360.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .text_color(if self.status.error {
                                t.error
                            } else {
                                t.text_primary
                            })
                            .child(self.status.text),
                    )
                    .when(self.status.show_progress, |this| {
                        this.child(ProgressBar::new(self.progress, t))
                    }),
            )
            .child({
                let button =
                    GradientButton::new("cta", self.cta_label, t).disabled(self.cta_disabled);
                match self.on_cta {
                    Some(handler) => button.on_click(handler),
                    None => button,
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> BatchState {
        BatchState::default()
    }

    #[test]
    fn idle_batch_shows_the_file_count_without_progress() {
        let status = batch_status(BatchState {
            file_count: 3,
            ..state()
        });
        assert_eq!(status.text, "3 ファイル");
        assert!(!status.error);
        assert!(!status.show_progress);
    }

    #[test]
    fn converting_batch_shows_percent_and_progress() {
        let status = batch_status(BatchState {
            converting: true,
            progress: 40,
            file_count: 5,
            ..state()
        });
        assert_eq!(status.text, "変換中... 40%");
        assert!(!status.error);
        assert!(status.show_progress);
    }

    #[test]
    fn converting_takes_precedence_over_earlier_errors() {
        let status = batch_status(BatchState {
            converting: true,
            progress: 10,
            error_count: 2,
            ..state()
        });
        assert_eq!(status.text, "変換中... 10%");
        assert!(!status.error);
    }

    #[test]
    fn finished_batch_without_errors_is_complete() {
        let status = batch_status(BatchState {
            progress: 100,
            result_count: 2,
            ..state()
        });
        assert_eq!(status.text, "完了");
        assert!(!status.error);
        assert!(status.show_progress);
    }

    #[test]
    fn finished_batch_where_everything_failed_reports_failure_count() {
        let status = batch_status(BatchState {
            progress: 100,
            error_count: 3,
            ..state()
        });
        assert_eq!(status.text, "失敗 (3件)");
        assert!(status.error);
    }

    #[test]
    fn finished_batch_with_partial_failure_reports_both() {
        let status = batch_status(BatchState {
            progress: 100,
            result_count: 1,
            error_count: 2,
            ..state()
        });
        assert_eq!(status.text, "完了 (2件失敗)");
        assert!(status.error);
    }
}
