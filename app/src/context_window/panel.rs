use warp_core::ui::theme::Fill;
use warpui::{
    elements::{
        Container, CrossAxisAlignment, Element, Flex, MainAxisSize, MouseStateHandle,
        ParentElement, Shrinkable, Text,
    },
    ui_components::components::UiComponent,
    AppContext, Entity, SingletonEntity, View, ViewContext,
};

use crate::appearance::Appearance;
use crate::ui_components::buttons::icon_button;
use warp_core::ui::Icon;

use super::model::{ContextState, CtxWindowModel, CtxWindowModelEvent, LoadState};

pub struct CtxWindowPanel {
    model: warpui::ModelHandle<CtxWindowModel>,
    refresh_mouse_state: MouseStateHandle,
}

#[derive(Clone, Debug)]
pub enum CtxWindowPanelAction {
    Refresh,
}

#[derive(Clone, Debug)]
pub enum CtxWindowPanelEvent {}

impl CtxWindowPanel {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let model = ctx.add_model(CtxWindowModel::new);

        ctx.subscribe_to_model(&model, |_me, _, event, ctx| match event {
            CtxWindowModelEvent::Updated | CtxWindowModelEvent::Error(_) => ctx.notify(),
        });

        model.update(ctx, |m, ctx| {
            m.load(ctx);
        });

        Self {
            model,
            refresh_mouse_state: MouseStateHandle::default(),
        }
    }

    fn render_section_header(title: &str, appearance: &Appearance) -> Box<dyn Element> {
        let color = appearance.theme().foreground().into_solid();
        Container::new(
            Text::new(title.to_string(), appearance.ui_font_family(), 11.)
                .with_color(color)
                .finish(),
        )
        .with_padding_left(10.)
        .with_padding_top(10.)
        .with_padding_bottom(4.)
        .finish()
    }

    fn render_kv_row(key: &str, value: &str, appearance: &Appearance) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();
        let fg = appearance.theme().foreground().into_solid();

        Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(
                    Text::new(key.to_string(), appearance.ui_font_family(), 10.)
                        .with_color(sub_color)
                        .finish(),
                )
                .with_padding_right(6.)
                .finish(),
            )
            .with_child(
                Shrinkable::new(
                    1.0,
                    Text::new(value.to_string(), appearance.ui_font_family(), 10.)
                        .with_color(fg)
                        .finish(),
                )
                .finish(),
            )
            .with_main_axis_size(MainAxisSize::Max)
            .finish()
    }

    fn render_list_item(text: &str, appearance: &Appearance) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();
        Container::new(
            Text::new(text.to_string(), appearance.ui_font_family(), 10.)
                .with_color(sub_color)
                .finish(),
        )
        .with_padding_left(16.)
        .with_padding_top(1.)
        .with_padding_bottom(1.)
        .finish()
    }

    fn render_priority_badge(priority: &str, appearance: &Appearance) -> Box<dyn Element> {
        let color = match priority {
            "P0" => Fill::error().into_solid(),
            "P1" => Fill::warn().into_solid(),
            _ => appearance
                .theme()
                .sub_text_color(appearance.theme().background())
                .into_solid(),
        };
        Container::new(
            Text::new(priority.to_string(), appearance.ui_font_family(), 9.)
                .with_color(color)
                .finish(),
        )
        .with_padding_right(6.)
        .finish()
    }

    fn render_git_section(state: &ContextState, appearance: &Appearance) -> Box<dyn Element> {
        let mut col = Flex::column();
        col = col.with_child(Self::render_section_header("Git", appearance));

        let dirty_label = if state.git.dirty_count > 0 {
            format!("{} dirty", state.git.dirty_count)
        } else {
            "clean".to_string()
        };
        col = col.with_child(
            Container::new(Self::render_kv_row("branch", &state.git.branch, appearance))
                .with_padding_left(10.)
                .with_padding_right(10.)
                .finish(),
        );
        col = col.with_child(
            Container::new(Self::render_kv_row("tree", &dirty_label, appearance))
                .with_padding_left(10.)
                .with_padding_right(10.)
                .finish(),
        );

        for commit in state.git.recent_commits.iter().take(5) {
            col = col.with_child(Self::render_list_item(commit, appearance));
        }

        col.finish()
    }

    fn render_ai_section(state: &ContextState, appearance: &Appearance) -> Box<dyn Element> {
        let mut col = Flex::column();
        col = col.with_child(Self::render_section_header("AI Context", appearance));

        if !state.ai.claude_md_files.is_empty() {
            col = col.with_child(
                Container::new(Self::render_kv_row(
                    "rules",
                    &format!("{} files", state.ai.claude_md_files.len()),
                    appearance,
                ))
                .with_padding_left(10.)
                .with_padding_right(10.)
                .finish(),
            );
            for path in &state.ai.claude_md_files {
                let display = path
                    .strip_prefix(&std::env::var("HOME").unwrap_or_default())
                    .map(|p| format!("~{p}"))
                    .unwrap_or_else(|| path.clone());
                col = col.with_child(Self::render_list_item(&display, appearance));
            }
        }

        if !state.ai.mcp_servers.is_empty() {
            col = col.with_child(
                Container::new(Self::render_kv_row(
                    "MCP",
                    &state.ai.mcp_servers.join(", "),
                    appearance,
                ))
                .with_padding_left(10.)
                .with_padding_right(10.)
                .finish(),
            );
        }

        col.finish()
    }

    fn render_handoff_section(state: &ContextState, appearance: &Appearance) -> Box<dyn Element> {
        let mut col = Flex::column();
        col = col.with_child(Self::render_section_header("Handoff", appearance));

        if state.handoff.items.is_empty() {
            col = col.with_child(Self::render_list_item("No items", appearance));
        } else {
            for item in &state.handoff.items {
                let sub_color = appearance
                    .theme()
                    .sub_text_color(appearance.theme().background())
                    .into_solid();
                let row = Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(Self::render_priority_badge(&item.priority, appearance))
                    .with_child(
                        Shrinkable::new(
                            1.0,
                            Text::new(item.summary.clone(), appearance.ui_font_family(), 10.)
                                .with_color(sub_color)
                                .finish(),
                        )
                        .finish(),
                    )
                    .with_main_axis_size(MainAxisSize::Max)
                    .finish();
                col = col.with_child(
                    Container::new(row)
                        .with_padding_left(16.)
                        .with_padding_top(2.)
                        .with_padding_bottom(2.)
                        .with_padding_right(10.)
                        .finish(),
                );
            }
        }

        col.finish()
    }

    fn render_todos_section(state: &ContextState, appearance: &Appearance) -> Box<dyn Element> {
        let mut col = Flex::column();
        col = col.with_child(Self::render_section_header("Todos", appearance));

        if state.todos.pending.is_empty() {
            col = col.with_child(Self::render_list_item("No pending items", appearance));
        } else {
            for item in state.todos.pending.iter().take(10) {
                let sub_color = appearance
                    .theme()
                    .sub_text_color(appearance.theme().background())
                    .into_solid();
                let row = Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(Self::render_priority_badge(&item.priority, appearance))
                    .with_child(
                        Shrinkable::new(
                            1.0,
                            Text::new(item.name.clone(), appearance.ui_font_family(), 10.)
                                .with_color(sub_color)
                                .finish(),
                        )
                        .finish(),
                    )
                    .with_main_axis_size(MainAxisSize::Max)
                    .finish();
                col = col.with_child(
                    Container::new(row)
                        .with_padding_left(16.)
                        .with_padding_top(2.)
                        .with_padding_bottom(2.)
                        .with_padding_right(10.)
                        .finish(),
                );
            }
        }

        col.finish()
    }
}

impl Entity for CtxWindowPanel {
    type Event = CtxWindowPanelEvent;
}

impl warpui::TypedActionView for CtxWindowPanel {
    type Action = CtxWindowPanelAction;

    fn handle_action(&mut self, action: &CtxWindowPanelAction, ctx: &mut ViewContext<Self>) {
        match action {
            CtxWindowPanelAction::Refresh => {
                self.model.update(ctx, |m, ctx| {
                    m.load(ctx);
                });
            }
        }
    }
}

impl View for CtxWindowPanel {
    fn ui_name() -> &'static str {
        "CtxWindowPanel"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let model = self.model.as_ref(app);

        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();

        let refresh_btn = icon_button(
            appearance,
            Icon::Refresh,
            false,
            self.refresh_mouse_state.clone(),
        )
        .build()
        .on_click(|ctx, _, _| {
            ctx.dispatch_typed_action(CtxWindowPanelAction::Refresh);
        })
        .finish();

        let header = Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Shrinkable::new(
                        1.0,
                        Text::new(
                            "Context Window",
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(appearance.theme().foreground().into_solid())
                        .finish(),
                    )
                    .finish(),
                )
                .with_child(refresh_btn)
                .with_main_axis_size(MainAxisSize::Max)
                .finish(),
        )
        .with_padding_left(10.)
        .with_padding_right(10.)
        .with_padding_top(8.)
        .with_padding_bottom(4.)
        .finish();

        let body: Box<dyn Element> = match &model.load_state {
            LoadState::NotLoaded => Container::new(
                Text::new("Loading\u{2026}", appearance.ui_font_family(), 12.)
                    .with_color(sub_color)
                    .finish(),
            )
            .with_padding_left(10.)
            .with_padding_top(8.)
            .finish(),

            LoadState::Error(e) => Container::new(
                Text::new(
                    format!("Run warp-context-gen to populate: {e}"),
                    appearance.ui_font_family(),
                    11.,
                )
                .with_color(sub_color)
                .finish(),
            )
            .with_padding_left(10.)
            .with_padding_top(8.)
            .finish(),

            LoadState::Loaded => {
                let col = Flex::column()
                    .with_child(Self::render_git_section(&model.state, appearance))
                    .with_child(Self::render_ai_section(&model.state, appearance))
                    .with_child(Self::render_handoff_section(&model.state, appearance))
                    .with_child(Self::render_todos_section(&model.state, appearance));

                Shrinkable::new(1.0, col.finish()).finish()
            }
        };

        Flex::column()
            .with_child(header)
            .with_child(Shrinkable::new(1.0, body).finish())
            .with_main_axis_size(MainAxisSize::Max)
            .finish()
    }
}
