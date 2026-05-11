use std::collections::{HashMap, HashSet};

use warp_core::ui::theme::Fill;
use warp_core::ui::Icon;
use warpui::{
    elements::{
        ChildView, ClippedScrollStateHandle, ClippedScrollable, Container, CrossAxisAlignment,
        Element, Flex, Hoverable, MainAxisSize, MouseStateHandle, ParentElement, ScrollbarWidth,
        Shrinkable, Text,
    },
    ui_components::components::UiComponent,
    AppContext, Entity, SingletonEntity, View, ViewContext, ViewHandle,
};

use crate::appearance::Appearance;
use crate::ui_components::buttons::icon_button;
use crate::view_components::{SubmittableTextInput, SubmittableTextInputEvent};

use super::model::{AddDoobItem, DoobItem, DoobModel, DoobModelEvent, LoadState, MutationState};

pub struct DoobPanel {
    model: warpui::ModelHandle<DoobModel>,
    add_input: ViewHandle<SubmittableTextInput>,
    expanded: HashSet<String>,
    row_mouse_states: HashMap<String, MouseStateHandle>,
    complete_mouse_states: HashMap<String, MouseStateHandle>,
    refresh_mouse_state: MouseStateHandle,
    scroll_state: ClippedScrollStateHandle,
}

#[derive(Clone, Debug)]
pub enum DoobPanelAction {
    Refresh,
    ToggleExpand(String),
    Complete(String),
}

#[derive(Clone, Debug)]
pub enum DoobPanelEvent {
    #[allow(dead_code)]
    Refreshed,
}

impl DoobPanel {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let model = ctx.add_model(DoobModel::new);
        let add_input = ctx.add_typed_action_view(|ctx| {
            let mut input =
                SubmittableTextInput::new(ctx).validate_on_submit(|text| !text.trim().is_empty());
            input.set_placeholder_text("Add Doob task", ctx);
            input.set_outer_margins(4., 6., ctx);
            input
        });

        ctx.subscribe_to_model(&model, |me, _, event, ctx| match event {
            DoobModelEvent::Loaded => {
                me.sync_item_mouse_states(ctx);
                ctx.notify();
            }
            DoobModelEvent::Mutated | DoobModelEvent::Error(_) => ctx.notify(),
        });

        ctx.subscribe_to_view(&add_input, |me, _, event, ctx| match event {
            SubmittableTextInputEvent::Submit(title) => {
                let item = AddDoobItem {
                    title: title.clone(),
                    priority: None,
                    due: None,
                    repo: None,
                };
                me.model.update(ctx, |model, ctx| {
                    model.add(item, ctx);
                });
            }
            SubmittableTextInputEvent::Escape => {}
        });

        model.update(ctx, |m, ctx| {
            m.load(ctx);
        });

        Self {
            model,
            add_input,
            expanded: HashSet::new(),
            row_mouse_states: HashMap::new(),
            complete_mouse_states: HashMap::new(),
            refresh_mouse_state: MouseStateHandle::default(),
            scroll_state: ClippedScrollStateHandle::default(),
        }
    }

    fn sync_item_mouse_states(&mut self, ctx: &mut ViewContext<Self>) {
        let item_ids: HashSet<String> = self
            .model
            .as_ref(ctx)
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect();

        self.row_mouse_states.retain(|id, _| item_ids.contains(id));
        self.complete_mouse_states
            .retain(|id, _| item_ids.contains(id));

        for id in item_ids {
            self.row_mouse_states.entry(id.clone()).or_default();
            self.complete_mouse_states.entry(id).or_default();
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

    fn render_priority_badge(priority: Option<&str>, appearance: &Appearance) -> Box<dyn Element> {
        let label = if matches!(priority, Some("P0" | "P1" | "P2" | "P3")) {
            priority.unwrap_or("  ")
        } else {
            "  "
        };
        let color = match priority {
            Some("P0") => Fill::error().into_solid(),
            Some("P1") => Fill::warn().into_solid(),
            _ => appearance
                .theme()
                .sub_text_color(appearance.theme().background())
                .into_solid(),
        };
        Container::new(
            Text::new(label.to_string(), appearance.ui_font_family(), 9.)
                .with_color(color)
                .finish(),
        )
        .with_padding_right(6.)
        .finish()
    }

    fn render_complete_button(
        &self,
        item: &DoobItem,
        is_mutating: bool,
        appearance: &Appearance,
    ) -> Option<Box<dyn Element>> {
        if !matches!(
            item.status.as_deref(),
            Some("pending" | "in_progress" | "in-progress")
        ) {
            return None;
        }

        let mouse_state = self.complete_mouse_states.get(&item.id).cloned()?;

        let item_id = item.id.clone();
        let mut button = icon_button(appearance, Icon::Check, false, mouse_state).build();
        if is_mutating {
            button = button.disable();
        }

        Some(
            button
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(DoobPanelAction::Complete(item_id.clone()));
                })
                .finish(),
        )
    }

    fn render_item(
        &self,
        item: &DoobItem,
        is_mutating: bool,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let is_expanded = self.expanded.contains(&item.id);
        let item_id = item.id.clone();
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();

        let title_text = item.title.clone().unwrap_or_else(|| item.id.clone());

        let row_content = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Self::render_priority_badge(
                item.priority.as_deref(),
                appearance,
            ))
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
        let row_mouse = self
            .row_mouse_states
            .get(&item.id)
            .cloned()
            .unwrap_or_default();
        let row_id = item_id.clone();
        let clickable_row = Hoverable::new(row_mouse, move |_| row_content)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(DoobPanelAction::ToggleExpand(row_id.clone()));
            })
            .finish();

        let mut row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Shrinkable::new(1., clickable_row).finish())
            .with_main_axis_size(MainAxisSize::Max);

        if let Some(complete_button) = self.render_complete_button(item, is_mutating, appearance) {
            row = row.with_child(complete_button);
        }

        let mut col = Flex::column().with_child(
            Container::new(row.finish())
                .with_padding_left(16.)
                .with_padding_top(2.)
                .with_padding_bottom(2.)
                .with_padding_right(10.)
                .finish(),
        );

        if is_expanded {
            if let Some(desc) = &item.description {
                let trimmed = desc.trim();
                if !trimmed.is_empty() {
                    col = col.with_child(
                        Container::new(
                            Text::new(trimmed.to_string(), appearance.ui_font_family(), 10.)
                                .with_color(sub_color)
                                .finish(),
                        )
                        .with_padding_left(22.)
                        .with_padding_right(10.)
                        .with_padding_bottom(4.)
                        .finish(),
                    );
                }
            }
        }

        col.finish()
    }

    fn render_project_header(project: &str, appearance: &Appearance) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();
        Container::new(
            Text::new(project.to_string(), appearance.ui_font_family(), 10.)
                .with_color(sub_color)
                .finish(),
        )
        .with_padding_left(10.)
        .with_padding_top(6.)
        .with_padding_bottom(2.)
        .finish()
    }

    fn render_group(
        &self,
        title: &str,
        items: &[&DoobItem],
        is_mutating: bool,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let mut col = Flex::column().with_child(Self::render_section_header(title, appearance));

        let mut project_order: Vec<String> = Vec::new();
        let mut by_project: HashMap<String, Vec<&DoobItem>> = HashMap::new();
        for item in items {
            let proj = item
                .project
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            if !by_project.contains_key(&proj) {
                project_order.push(proj.clone());
            }
            by_project.entry(proj).or_default().push(item);
        }

        for proj in &project_order {
            col = col.with_child(Self::render_project_header(proj, appearance));
            for item in &by_project[proj] {
                col = col.with_child(self.render_item(item, is_mutating, appearance));
            }
        }

        col.finish()
    }

    fn render_mutation_message(
        model: &DoobModel,
        appearance: &Appearance,
    ) -> Option<Box<dyn Element>> {
        let (message, color) = match &model.mutation_state {
            MutationState::Idle => return None,
            MutationState::Mutating => (
                "Updating Doob\u{2026}".to_string(),
                appearance
                    .theme()
                    .sub_text_color(appearance.theme().background())
                    .into_solid(),
            ),
            MutationState::Success(message) => (
                message.clone(),
                appearance
                    .theme()
                    .sub_text_color(appearance.theme().background())
                    .into_solid(),
            ),
            MutationState::Error(error) => (format!("Error: {error}"), Fill::error().into_solid()),
        };

        Some(
            Container::new(
                Text::new(message, appearance.ui_font_family(), 10.)
                    .with_color(color)
                    .finish(),
            )
            .with_padding_left(10.)
            .with_padding_right(10.)
            .with_padding_bottom(4.)
            .finish(),
        )
    }
}

impl Entity for DoobPanel {
    type Event = DoobPanelEvent;
}

impl warpui::TypedActionView for DoobPanel {
    type Action = DoobPanelAction;

    fn handle_action(&mut self, action: &DoobPanelAction, ctx: &mut ViewContext<Self>) {
        match action {
            DoobPanelAction::Refresh => {
                self.model.update(ctx, |m, ctx| {
                    m.load(ctx);
                });
            }
            DoobPanelAction::Complete(id) => {
                let id = id.clone();
                self.model.update(ctx, |m, ctx| {
                    m.complete(id, ctx);
                });
            }
            DoobPanelAction::ToggleExpand(id) => {
                if self.expanded.contains(id) {
                    self.expanded.remove(id);
                } else {
                    self.expanded.insert(id.clone());
                }
                ctx.notify();
            }
        }
    }
}

impl View for DoobPanel {
    fn ui_name() -> &'static str {
        "DoobPanel"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let model = self.model.as_ref(app);

        let item_count = model.items.len();
        let count_text = format!(
            "{item_count} task{}",
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
            ctx.dispatch_typed_action(DoobPanelAction::Refresh);
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
                                    "Doob Tasks",
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
                if model.items.is_empty() {
                    Container::new(
                        Text::new("No tasks found.", appearance.ui_font_family(), 11.)
                            .with_color(sub_color)
                            .finish(),
                    )
                    .with_padding_left(10.)
                    .with_padding_top(8.)
                    .finish()
                } else {
                    let theme = appearance.theme();
                    let is_mutating = model.mutation_state == MutationState::Mutating;
                    let pending: Vec<&DoobItem> = model
                        .items
                        .iter()
                        .filter(|i| i.status.as_deref() == Some("pending"))
                        .collect();
                    let in_progress: Vec<&DoobItem> = model
                        .items
                        .iter()
                        .filter(|i| i.status.as_deref() == Some("in_progress"))
                        .collect();
                    let done: Vec<&DoobItem> = model
                        .items
                        .iter()
                        .filter(|i| i.status.as_deref() == Some("done"))
                        .collect();

                    let mut col = Flex::column();
                    if !pending.is_empty() {
                        col = col.with_child(self.render_group(
                            "Pending",
                            &pending,
                            is_mutating,
                            appearance,
                        ));
                    }
                    if !in_progress.is_empty() {
                        col = col.with_child(self.render_group(
                            "In Progress",
                            &in_progress,
                            is_mutating,
                            appearance,
                        ));
                    }
                    if !done.is_empty() {
                        col = col.with_child(self.render_group(
                            "Done",
                            &done,
                            is_mutating,
                            appearance,
                        ));
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
            .with_child(
                Container::new(ChildView::new(&self.add_input).finish())
                    .with_padding_left(10.)
                    .with_padding_right(10.)
                    .finish(),
            )
            .with_child(
                Self::render_mutation_message(model, appearance).unwrap_or_else(|| {
                    Container::new(
                        Text::new(String::new(), appearance.ui_font_family(), 0.).finish(),
                    )
                    .finish()
                }),
            )
            .with_child(Shrinkable::new(1.0, body).finish())
            .with_main_axis_size(MainAxisSize::Max)
            .finish()
    }
}
