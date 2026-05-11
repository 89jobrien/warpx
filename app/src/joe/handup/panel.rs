use std::collections::{HashMap, HashSet};

use warp_core::ui::theme::Fill;
use warp_core::ui::Icon;
use warpui::{
    elements::{
        ClippedScrollStateHandle, ClippedScrollable, Container, CrossAxisAlignment, Element, Flex,
        Hoverable, MainAxisSize, MouseStateHandle, ParentElement, ScrollbarWidth, Shrinkable, Text,
    },
    ui_components::components::UiComponent,
    AppContext, Entity, SingletonEntity, View, ViewContext,
};

use crate::appearance::Appearance;
use crate::ui_components::buttons::icon_button;

use super::model::{HandupCheckpoint, HandupModel, HandupModelEvent, LoadState};

pub struct HandupPanel {
    model: warpui::ModelHandle<HandupModel>,
    expanded_projects: HashSet<String>,
    expanded_rows: HashSet<i64>,
    refresh_mouse_state: MouseStateHandle,
    scroll_state: ClippedScrollStateHandle,
}

#[derive(Clone, Debug)]
pub enum HandupPanelAction {
    Refresh,
    ToggleProject(String),
    ToggleRow(i64),
}

#[derive(Clone, Debug)]
pub enum HandupPanelEvent {
    #[allow(dead_code)]
    Refreshed,
}

impl HandupPanel {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let model = ctx.add_model(HandupModel::new);

        ctx.subscribe_to_model(&model, |_me, _, event, ctx| match event {
            HandupModelEvent::Loaded | HandupModelEvent::Error(_) => ctx.notify(),
        });

        model.update(ctx, |m, ctx| {
            m.load(ctx);
        });

        Self {
            model,
            expanded_projects: HashSet::new(),
            expanded_rows: HashSet::new(),
            refresh_mouse_state: MouseStateHandle::default(),
            scroll_state: ClippedScrollStateHandle::default(),
        }
    }

    fn render_date_badge(date: &str, appearance: &Appearance) -> Box<dyn Element> {
        let color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();
        Container::new(
            Text::new(date.to_string(), appearance.ui_font_family(), 9.)
                .with_color(color)
                .finish(),
        )
        .with_padding_right(6.)
        .finish()
    }

    fn render_checkpoint(
        &self,
        cp: &HandupCheckpoint,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let is_expanded = self.expanded_rows.contains(&cp.id);
        let cp_id = cp.id;
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();

        let title_text = cp
            .recommendation
            .clone()
            .unwrap_or_else(|| "(no recommendation)".to_string());

        let row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Self::render_date_badge(&cp.generated, appearance))
            .with_child(
                Shrinkable::new(
                    1.0,
                    Text::new(title_text, appearance.ui_font_family(), 11.)
                        .with_color(sub_color)
                        .finish(),
                )
                .finish(),
            )
            .with_main_axis_size(MainAxisSize::Max)
            .finish();

        let row_mouse = MouseStateHandle::default();
        let clickable_row = Hoverable::new(row_mouse, move |_| row)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(HandupPanelAction::ToggleRow(cp_id));
            })
            .finish();

        let mut col = Flex::column().with_child(
            Container::new(clickable_row)
                .with_padding_left(16.)
                .with_padding_top(2.)
                .with_padding_bottom(2.)
                .with_padding_right(10.)
                .finish(),
        );

        if is_expanded {
            if let Some(rec) = &cp.recommendation {
                let trimmed = rec.trim();
                if !trimmed.is_empty() {
                    col = col.with_child(
                        Container::new(
                            Text::new(trimmed.to_string(), appearance.ui_font_family(), 10.)
                                .with_color(sub_color)
                                .finish(),
                        )
                        .with_padding_left(22.)
                        .with_padding_right(10.)
                        .with_padding_bottom(2.)
                        .finish(),
                    );
                }
            }
            col = col.with_child(
                Container::new(
                    Text::new(cp.cwd.clone(), appearance.ui_font_family(), 9.)
                        .with_color(sub_color)
                        .finish(),
                )
                .with_padding_left(22.)
                .with_padding_right(10.)
                .with_padding_bottom(4.)
                .finish(),
            );
        }

        col.finish()
    }

    fn render_project_header(
        project: &str,
        count: usize,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();
        let label = if count > 1 {
            format!("{project} ({count})")
        } else {
            project.to_string()
        };
        Container::new(
            Text::new(label, appearance.ui_font_family(), 10.)
                .with_color(sub_color)
                .finish(),
        )
        .with_padding_left(10.)
        .with_padding_top(6.)
        .with_padding_bottom(2.)
        .finish()
    }

    /// Group checkpoints by project, sorted alphabetically. Within each
    /// group, checkpoints are already sorted by `created_at` DESC from the
    /// query.
    fn grouped(
        checkpoints: &[HandupCheckpoint],
    ) -> (Vec<String>, HashMap<String, Vec<&HandupCheckpoint>>) {
        let mut by_project: HashMap<String, Vec<&HandupCheckpoint>> = HashMap::new();
        for cp in checkpoints {
            by_project.entry(cp.project.clone()).or_default().push(cp);
        }
        let mut projects: Vec<String> = by_project.keys().cloned().collect();
        projects.sort();
        (projects, by_project)
    }
}

impl Entity for HandupPanel {
    type Event = HandupPanelEvent;
}

impl warpui::TypedActionView for HandupPanel {
    type Action = HandupPanelAction;

    fn handle_action(&mut self, action: &HandupPanelAction, ctx: &mut ViewContext<Self>) {
        match action {
            HandupPanelAction::Refresh => {
                self.model.update(ctx, |m, ctx| {
                    m.load(ctx);
                });
            }
            HandupPanelAction::ToggleProject(proj) => {
                if self.expanded_projects.contains(proj) {
                    self.expanded_projects.remove(proj);
                } else {
                    self.expanded_projects.insert(proj.clone());
                }
                ctx.notify();
            }
            HandupPanelAction::ToggleRow(id) => {
                if self.expanded_rows.contains(id) {
                    self.expanded_rows.remove(id);
                } else {
                    self.expanded_rows.insert(*id);
                }
                ctx.notify();
            }
        }
    }
}

impl View for HandupPanel {
    fn ui_name() -> &'static str {
        "HandupPanel"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let model = self.model.as_ref(app);

        let item_count = model.checkpoints.len();
        let count_text = format!(
            "{item_count} item{}",
            if item_count == 1 { "" } else { "s" }
        );

        let refresh_btn = icon_button(
            appearance,
            Icon::Refresh,
            false,
            self.refresh_mouse_state.clone(),
        )
        .build()
        .on_click(|ctx, _, _| {
            ctx.dispatch_typed_action(HandupPanelAction::Refresh);
        })
        .finish();

        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();

        let header = Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Shrinkable::new(
                        1.0,
                        Flex::column()
                            .with_child(
                                Text::new(
                                    "Handup",
                                    appearance.ui_font_family(),
                                    appearance.ui_font_size(),
                                )
                                .with_color(appearance.theme().foreground().into_solid())
                                .finish(),
                            )
                            .with_child(
                                Text::new(count_text, appearance.ui_font_family(), 10.)
                                    .with_color(sub_color)
                                    .finish(),
                            )
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
            LoadState::NotLoaded | LoadState::Loading => Container::new(
                Text::new("Loading\u{2026}", appearance.ui_font_family(), 11.)
                    .with_color(sub_color)
                    .finish(),
            )
            .with_padding_left(10.)
            .with_padding_top(8.)
            .finish(),

            LoadState::Error(e) => Container::new(
                Text::new(format!("Error: {e}"), appearance.ui_font_family(), 11.)
                    .with_color(Fill::error().into_solid())
                    .finish(),
            )
            .with_padding_left(10.)
            .with_padding_top(8.)
            .finish(),

            LoadState::Loaded => {
                if model.checkpoints.is_empty() {
                    Container::new(
                        Text::new(
                            "No handup checkpoints found.",
                            appearance.ui_font_family(),
                            11.,
                        )
                        .with_color(sub_color)
                        .finish(),
                    )
                    .with_padding_left(10.)
                    .with_padding_top(8.)
                    .finish()
                } else {
                    let theme = appearance.theme();
                    let (projects, by_project) = Self::grouped(&model.checkpoints);

                    let mut col = Flex::column();
                    for proj in &projects {
                        let items = &by_project[proj];
                        let proj_clone = proj.clone();
                        let header_mouse = MouseStateHandle::default();
                        let project_header = Hoverable::new(header_mouse, {
                            let proj = proj.clone();
                            let items_len = items.len();
                            move |_| Self::render_project_header(&proj, items_len, appearance)
                        })
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(HandupPanelAction::ToggleProject(
                                proj_clone.clone(),
                            ));
                        })
                        .finish();

                        col = col.with_child(project_header);

                        let expanded = self.expanded_projects.contains(proj);
                        if expanded {
                            // Show all checkpoints
                            for cp in items {
                                col = col.with_child(self.render_checkpoint(cp, appearance));
                            }
                        } else {
                            // Show only latest (first, since sorted DESC)
                            if let Some(cp) = items.first() {
                                col = col.with_child(self.render_checkpoint(cp, appearance));
                            }
                        }
                    }

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
            }
        };

        Flex::column()
            .with_child(header)
            .with_child(Shrinkable::new(1.0, body).finish())
            .with_main_axis_size(MainAxisSize::Max)
            .finish()
    }
}
