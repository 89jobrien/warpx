use warp_core::ui::theme::Fill;
use warpui::{
    elements::{
        ClippedScrollStateHandle, ClippedScrollable, Container, CrossAxisAlignment, Element, Flex,
        MainAxisSize, ParentElement, ScrollbarWidth, Shrinkable, Text,
    },
    AppContext, Entity, SingletonEntity, View, ViewContext,
};

use crate::appearance::Appearance;

use super::model::{TestRunnerModel, TestStatus};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct TestRunnerPanel {
    model: TestRunnerModel,
    /// Which test indices are expanded (showing failure output).
    expanded: std::collections::HashSet<(usize, usize)>,
    scroll_state: ClippedScrollStateHandle,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum TestRunnerPanelAction {
    ToggleExpand { suite: usize, test: usize },
    Rerun { suite: usize, test: usize },
}

#[derive(Clone, Debug)]
pub enum TestRunnerPanelEvent {}

impl TestRunnerPanel {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        Self {
            model: TestRunnerModel::new(),
            expanded: std::collections::HashSet::new(),
            scroll_state: ClippedScrollStateHandle::default(),
        }
    }

    // -----------------------------------------------------------------------
    // Rendering helpers
    // -----------------------------------------------------------------------

    fn render_summary(model: &TestRunnerModel, appearance: &Appearance) -> Box<dyn Element> {
        let (passed, failed, skipped) = model.summary();
        let label = format!("{passed} passed, {failed} failed, {skipped} skipped");
        let color = if failed > 0 {
            Fill::error().into_solid()
        } else {
            appearance
                .theme()
                .sub_text_color(appearance.theme().background())
                .into_solid()
        };
        Container::new(
            Text::new(label, appearance.ui_font_family(), 10.)
                .with_color(color)
                .finish(),
        )
        .with_padding_left(10.)
        .with_padding_top(4.)
        .with_padding_bottom(8.)
        .finish()
    }

    fn render_status_badge(status: &TestStatus, appearance: &Appearance) -> Box<dyn Element> {
        let (label, color) = match status {
            TestStatus::Pass => (
                "\u{2713}", // ✓
                Fill::success().into_solid(),
            ),
            TestStatus::Fail => (
                "\u{2717}", // ✗
                Fill::error().into_solid(),
            ),
            TestStatus::Skip => (
                "\u{2014}", // —
                appearance
                    .theme()
                    .disabled_text_color(appearance.theme().background())
                    .into_solid(),
            ),
        };
        Container::new(
            Text::new(label.to_string(), appearance.ui_font_family(), 11.)
                .with_color(color)
                .finish(),
        )
        .with_padding_right(6.)
        .finish()
    }

    fn render_suite(
        suite_idx: usize,
        suite_name: &str,
        model: &TestRunnerModel,
        expanded: &std::collections::HashSet<(usize, usize)>,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let fg = theme.foreground().into_solid();
        let sub_color = theme.sub_text_color(theme.background()).into_solid();
        let mono_font = appearance.monospace_font_family();

        let mut col = Flex::column();

        // Suite header
        col = col.with_child(
            Container::new(
                Text::new(suite_name.to_string(), appearance.ui_font_family(), 11.)
                    .with_color(fg)
                    .finish(),
            )
            .with_padding_left(10.)
            .with_padding_top(8.)
            .with_padding_bottom(4.)
            .finish(),
        );

        let suite = &model.suites[suite_idx];
        for (test_idx, test) in suite.tests.iter().enumerate() {
            let is_expanded = expanded.contains(&(suite_idx, test_idx));

            // Test row
            let name_color = match test.status {
                TestStatus::Fail => Fill::error().into_solid(),
                TestStatus::Skip => theme.disabled_text_color(theme.background()).into_solid(),
                TestStatus::Pass => sub_color,
            };

            let test_row = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(Self::render_status_badge(&test.status, appearance))
                .with_child(
                    Shrinkable::new(
                        1.0,
                        Text::new(test.name.clone(), appearance.ui_font_family(), 11.)
                            .with_color(name_color)
                            .finish(),
                    )
                    .finish(),
                )
                .with_main_axis_size(MainAxisSize::Max)
                .finish();

            col = col.with_child(
                Container::new(test_row)
                    .with_padding_left(16.)
                    .with_padding_top(2.)
                    .with_padding_bottom(2.)
                    .with_padding_right(10.)
                    .finish(),
            );

            // Inline failure output (expanded)
            if is_expanded {
                if let Some(output) = &test.output {
                    col = col.with_child(
                        Container::new(
                            Text::new(output.clone(), mono_font, 10.)
                                .with_color(
                                    theme.disabled_text_color(theme.background()).into_solid(),
                                )
                                .finish(),
                        )
                        .with_padding_left(24.)
                        .with_padding_right(10.)
                        .with_padding_top(2.)
                        .with_padding_bottom(4.)
                        .finish(),
                    );
                }
            }
        }

        col.finish()
    }
}

// ---------------------------------------------------------------------------
// Entity / View impl
// ---------------------------------------------------------------------------

impl Entity for TestRunnerPanel {
    type Event = TestRunnerPanelEvent;
}

impl warpui::TypedActionView for TestRunnerPanel {
    type Action = TestRunnerPanelAction;

    fn handle_action(&mut self, action: &TestRunnerPanelAction, _ctx: &mut ViewContext<Self>) {
        match action {
            TestRunnerPanelAction::ToggleExpand { suite, test } => {
                let key = (*suite, *test);
                if self.expanded.contains(&key) {
                    self.expanded.remove(&key);
                } else {
                    self.expanded.insert(key);
                }
            }
            TestRunnerPanelAction::Rerun { suite, test } => {
                if let Some(s) = self.model.suites.get(*suite) {
                    if let Some(t) = s.tests.get(*test) {
                        // Command is available as t.rerun_command for the caller
                        // to inject; log it here for now.
                        log::debug!("test-runner rerun: {}", t.rerun_command);
                    }
                }
            }
        }
    }
}

impl View for TestRunnerPanel {
    fn ui_name() -> &'static str {
        "TestRunnerPanel"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        let header = Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Shrinkable::new(
                        1.0,
                        Text::new(
                            "Test Runner",
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(theme.foreground().into_solid())
                        .finish(),
                    )
                    .finish(),
                )
                .with_main_axis_size(MainAxisSize::Max)
                .finish(),
        )
        .with_padding_left(10.)
        .with_padding_right(10.)
        .with_padding_top(8.)
        .with_padding_bottom(4.)
        .finish();

        let summary = Self::render_summary(&self.model, appearance);

        let mut body_col = Flex::column().with_child(summary);

        for (suite_idx, suite) in self.model.suites.iter().enumerate() {
            body_col = body_col.with_child(Self::render_suite(
                suite_idx,
                &suite.name.clone(),
                &self.model,
                &self.expanded,
                appearance,
            ));
        }

        let scrollable = ClippedScrollable::vertical(
            self.scroll_state.clone(),
            body_col.finish(),
            ScrollbarWidth::Auto,
            theme.disabled_text_color(theme.background()).into(),
            theme.main_text_color(theme.background()).into(),
            theme.background().into(),
        )
        .finish();

        Flex::column()
            .with_child(header)
            .with_child(Shrinkable::new(1.0, scrollable).finish())
            .with_main_axis_size(MainAxisSize::Max)
            .finish()
    }
}
