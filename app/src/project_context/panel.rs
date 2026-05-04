use warp_core::ui::theme::Fill;
use warp_core::ui::Icon;
use warpui::{
    elements::{
        ClippedScrollStateHandle, ClippedScrollable, Container, CrossAxisAlignment, Element, Flex,
        MainAxisSize, MouseStateHandle, ParentElement, ScrollbarWidth, Shrinkable, Text,
    },
    ui_components::components::UiComponent,
    AppContext, Entity, SingletonEntity, View, ViewContext,
};

use crate::appearance::Appearance;
use crate::ui_components::buttons::icon_button;

use super::model::{
    HandoffSection, LoadState, ProjectContextModel, ProjectContextModelEvent, ProjectContextState,
    TodoSection,
};

pub struct ProjectContextPanel {
    model: warpui::ModelHandle<ProjectContextModel>,
    refresh_mouse_state: MouseStateHandle,
    scroll_state: ClippedScrollStateHandle,
}

#[derive(Clone, Debug)]
pub enum ProjectContextPanelAction {
    Refresh,
}

#[derive(Clone, Debug)]
pub enum ProjectContextPanelEvent {}

impl ProjectContextPanel {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let model = ctx.add_model(ProjectContextModel::new);

        ctx.subscribe_to_model(&model, |_me, _, event, ctx| match event {
            ProjectContextModelEvent::Updated | ProjectContextModelEvent::Error(_) => ctx.notify(),
        });

        model.update(ctx, |m, ctx| {
            m.load(ctx);
        });

        Self {
            model,
            refresh_mouse_state: MouseStateHandle::default(),
            scroll_state: ClippedScrollStateHandle::default(),
        }
    }

    fn render_section_header(title: &str, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(
            Text::new(title.to_string(), appearance.ui_font_family(), 11.)
                .with_color(appearance.theme().foreground().into_solid())
                .finish(),
        )
        .with_padding_left(10.)
        .with_padding_top(10.)
        .with_padding_bottom(4.)
        .finish()
    }

    fn render_list_item(text: &str, appearance: &Appearance) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();
        Container::new(
            Text::new(text.to_string(), appearance.ui_font_family(), 11.)
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

    fn render_git_section(
        state: &ProjectContextState,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();
        let fg = appearance.theme().foreground().into_solid();

        let mut col = Flex::column();
        col = col.with_child(Self::render_section_header("Git", appearance));

        let dirty_label = if state.git.dirty_count > 0 {
            format!("{} dirty", state.git.dirty_count)
        } else {
            "clean".to_string()
        };

        let branch_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(
                    Text::new("branch".to_string(), appearance.ui_font_family(), 10.)
                        .with_color(sub_color)
                        .finish(),
                )
                .with_padding_right(6.)
                .finish(),
            )
            .with_child(
                Shrinkable::new(
                    1.0,
                    Text::new(state.git.branch.clone(), appearance.ui_font_family(), 10.)
                        .with_color(fg)
                        .finish(),
                )
                .finish(),
            )
            .with_main_axis_size(MainAxisSize::Max)
            .finish();

        let tree_row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(
                    Text::new("tree".to_string(), appearance.ui_font_family(), 10.)
                        .with_color(sub_color)
                        .finish(),
                )
                .with_padding_right(6.)
                .finish(),
            )
            .with_child(
                Shrinkable::new(
                    1.0,
                    Text::new(dirty_label, appearance.ui_font_family(), 10.)
                        .with_color(fg)
                        .finish(),
                )
                .finish(),
            )
            .with_main_axis_size(MainAxisSize::Max)
            .finish();

        col = col.with_child(
            Container::new(branch_row)
                .with_padding_left(10.)
                .with_padding_right(10.)
                .finish(),
        );
        col = col.with_child(
            Container::new(tree_row)
                .with_padding_left(10.)
                .with_padding_right(10.)
                .finish(),
        );

        for commit in state.git.recent_commits.iter().take(5) {
            col = col.with_child(Self::render_list_item(commit, appearance));
        }

        col.finish()
    }

    fn render_ai_section(state: &ProjectContextState, appearance: &Appearance) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();
        let fg = appearance.theme().foreground().into_solid();

        let mut col = Flex::column();
        col = col.with_child(Self::render_section_header("AI Context", appearance));

        if !state.ai.claude_md_files.is_empty() {
            let rules_row = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Container::new(
                        Text::new("rules".to_string(), appearance.ui_font_family(), 10.)
                            .with_color(sub_color)
                            .finish(),
                    )
                    .with_padding_right(6.)
                    .finish(),
                )
                .with_child(
                    Shrinkable::new(
                        1.0,
                        Text::new(
                            format!("{} files", state.ai.claude_md_files.len()),
                            appearance.ui_font_family(),
                            10.,
                        )
                        .with_color(fg)
                        .finish(),
                    )
                    .finish(),
                )
                .with_main_axis_size(MainAxisSize::Max)
                .finish();
            col = col.with_child(
                Container::new(rules_row)
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
            let mcp_row = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Container::new(
                        Text::new("MCP".to_string(), appearance.ui_font_family(), 10.)
                            .with_color(sub_color)
                            .finish(),
                    )
                    .with_padding_right(6.)
                    .finish(),
                )
                .with_child(
                    Shrinkable::new(
                        1.0,
                        Text::new(
                            state.ai.mcp_servers.join(", "),
                            appearance.ui_font_family(),
                            10.,
                        )
                        .with_color(fg)
                        .finish(),
                    )
                    .finish(),
                )
                .with_main_axis_size(MainAxisSize::Max)
                .finish();
            col = col.with_child(
                Container::new(mcp_row)
                    .with_padding_left(10.)
                    .with_padding_right(10.)
                    .finish(),
            );
        }

        col.finish()
    }

    fn render_handoff_section(
        handoff: &HandoffSection,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();
        let mut col = Flex::column();
        col = col.with_child(Self::render_section_header("Handoff", appearance));

        if handoff.items.is_empty() {
            col = col.with_child(Self::render_list_item("No handoff items", appearance));
        } else {
            for item in handoff.items.iter().filter(|i| i.status != "done") {
                let row = Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(Self::render_priority_badge(&item.priority, appearance))
                    .with_child(
                        Shrinkable::new(
                            1.0,
                            Text::new(item.summary.clone(), appearance.ui_font_family(), 11.)
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

    fn render_todos_section(todos: &TodoSection, appearance: &Appearance) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();
        let mut col = Flex::column();
        col = col.with_child(Self::render_section_header("Todos", appearance));

        if todos.pending.is_empty() {
            col = col.with_child(Self::render_list_item("No pending items", appearance));
        } else {
            for item in todos.pending.iter().take(10) {
                let row = Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(Self::render_priority_badge(&item.priority, appearance))
                    .with_child(
                        Shrinkable::new(
                            1.0,
                            Text::new(item.name.clone(), appearance.ui_font_family(), 11.)
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

impl Entity for ProjectContextPanel {
    type Event = ProjectContextPanelEvent;
}

impl warpui::TypedActionView for ProjectContextPanel {
    type Action = ProjectContextPanelAction;

    fn handle_action(&mut self, action: &ProjectContextPanelAction, ctx: &mut ViewContext<Self>) {
        match action {
            ProjectContextPanelAction::Refresh => {
                self.model.update(ctx, |m, ctx| {
                    m.load(ctx);
                });
            }
        }
    }
}

impl View for ProjectContextPanel {
    fn ui_name() -> &'static str {
        "ProjectContextPanel"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let model = self.model.as_ref(app);
        let theme = appearance.theme();

        let sub_color = theme.sub_text_color(theme.background()).into_solid();

        let refresh_btn = icon_button(
            appearance,
            Icon::Refresh,
            false,
            self.refresh_mouse_state.clone(),
        )
        .build()
        .on_click(|ctx, _, _| {
            ctx.dispatch_typed_action(ProjectContextPanelAction::Refresh);
        })
        .finish();

        let header = Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Shrinkable::new(
                        1.0,
                        Text::new(
                            "Project Context",
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .with_color(theme.foreground().into_solid())
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
                Text::new("Loading\u{2026}", appearance.ui_font_family(), 11.)
                    .with_color(sub_color)
                    .finish(),
            )
            .with_padding_left(10.)
            .with_padding_top(8.)
            .finish(),

            LoadState::Error(e) => Container::new(
                Text::new(
                    format!("No context found \u{2014} run warp-context-gen ({e})"),
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
                    .with_child(Self::render_handoff_section(
                        &model.state.handoff,
                        appearance,
                    ))
                    .with_child(Self::render_todos_section(&model.state.todos, appearance));

                ClippedScrollable::vertical(
                    self.scroll_state.clone(),
                    col.finish(),
                    ScrollbarWidth::Auto,
                    theme.disabled_text_color(theme.background()).into(),
                    theme.main_text_color(theme.background()).into(),
                    theme.background().into(),
                )
                .finish()
            }
        };

        Flex::column()
            .with_child(header)
            .with_child(Shrinkable::new(1.0, body).finish())
            .with_main_axis_size(MainAxisSize::Max)
            .finish()
    }
}
