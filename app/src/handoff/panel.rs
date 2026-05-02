use std::collections::HashSet;
use std::path::PathBuf;

use warp_core::ui::theme::Fill;
use warp_core::ui::Icon;
use warpui::{
    elements::{
        Container, CrossAxisAlignment, Element, Flex, Hoverable, MainAxisSize, MouseStateHandle,
        ParentElement, Shrinkable, Text,
    },
    ui_components::components::UiComponent,
    AppContext, Entity, SingletonEntity, View, ViewContext,
};

use crate::appearance::Appearance;
use crate::ui_components::buttons::icon_button;

use super::model::{HandoffItem, HandoffModel, HandoffModelEvent, LoadState};

pub struct HandoffPanel {
    model: warpui::ModelHandle<HandoffModel>,
    /// Set of item IDs currently expanded.
    expanded: HashSet<String>,
    refresh_mouse_state: MouseStateHandle,
}

#[derive(Clone, Debug)]
pub enum HandoffPanelAction {
    Refresh,
    ToggleExpand(String),
}

#[derive(Clone, Debug)]
pub enum HandoffPanelEvent {
    #[allow(dead_code)]
    Refreshed,
}

fn resolve_cwd() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")))
}

impl HandoffPanel {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let model = ctx.add_model(HandoffModel::new);

        ctx.subscribe_to_model(&model, |_me, _, event, ctx| match event {
            HandoffModelEvent::Loaded | HandoffModelEvent::Error(_) => ctx.notify(),
        });

        let cwd = resolve_cwd();
        log::info!("[handoff] HandoffPanel::new cwd={cwd:?}");
        model.update(ctx, |m, ctx| {
            m.load(cwd, ctx);
        });

        Self {
            model,
            expanded: HashSet::new(),
            refresh_mouse_state: MouseStateHandle::default(),
        }
    }

    fn render_priority_badge(priority: Option<&str>, appearance: &Appearance) -> Box<dyn Element> {
        let label = match priority {
            Some("P0") => "P0",
            Some("P1") => "P1",
            Some("P2") => "P2",
            Some("P3") => "P3",
            _ => "??",
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
            Text::new(label, appearance.ui_font_family(), 10.)
                .with_color(color)
                .finish(),
        )
        .with_padding_left(4.)
        .with_padding_right(4.)
        .finish()
    }

    fn render_status_badge(status: Option<&str>, appearance: &Appearance) -> Box<dyn Element> {
        let (label, color) = match status {
            Some("open") => ("open", appearance.theme().foreground().into_solid()),
            Some("blocked") => ("blocked", Fill::warn().into_solid()),
            Some("done") => ("done", Fill::success().into_solid()),
            _ => (
                "?",
                appearance
                    .theme()
                    .sub_text_color(appearance.theme().background())
                    .into_solid(),
            ),
        };
        Container::new(
            Text::new(label, appearance.ui_font_family(), 10.)
                .with_color(color)
                .finish(),
        )
        .with_padding_left(4.)
        .with_padding_right(4.)
        .finish()
    }

    fn render_item(
        &self,
        item: &HandoffItem,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let is_expanded = self.expanded.contains(&item.id);
        let item_id = item.id.clone();

        let title_text = item.title.clone().unwrap_or_else(|| item.id.clone());

        let priority_badge = Self::render_priority_badge(item.priority.as_deref(), appearance);
        let status_badge = Self::render_status_badge(item.status.as_deref(), appearance);

        let title_element = Text::new(
            title_text,
            appearance.ui_font_family(),
            appearance.ui_font_size(),
        )
        .with_color(appearance.theme().foreground().into_solid())
        .finish();

        let header: Box<dyn Element> = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(priority_badge)
            .with_child(Shrinkable::new(1.0, title_element).finish())
            .with_child(status_badge)
            .with_main_axis_size(MainAxisSize::Max)
            .finish();

        let row_mouse = MouseStateHandle::default();
        let row_id = item_id.clone();
        let clickable_header = Hoverable::new(row_mouse, move |_mouse_state| header)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(HandoffPanelAction::ToggleExpand(row_id.clone()));
            })
            .finish();

        if !is_expanded {
            return Container::new(clickable_header)
                .with_padding_top(4.)
                .with_padding_bottom(4.)
                .with_padding_left(8.)
                .with_padding_right(8.)
                .finish();
        }

        // Expanded body: show description if present.
        let mut body = Flex::column();

        if let Some(desc) = &item.description {
            if !desc.trim().is_empty() {
                let sub_color = appearance
                    .theme()
                    .sub_text_color(appearance.theme().background())
                    .into_solid();
                let desc_elem = Container::new(
                    Text::new(desc.trim().to_string(), appearance.ui_font_family(), 11.)
                        .with_color(sub_color)
                        .finish(),
                )
                .with_padding_left(8.)
                .with_padding_right(8.)
                .with_padding_top(2.)
                .with_padding_bottom(4.)
                .finish();
                body = body.with_child(desc_elem);
            }
        }

        Flex::column()
            .with_child(
                Container::new(clickable_header)
                    .with_padding_top(4.)
                    .with_padding_bottom(2.)
                    .with_padding_left(8.)
                    .with_padding_right(8.)
                    .finish(),
            )
            .with_child(body.finish())
            .finish()
    }
}

impl Entity for HandoffPanel {
    type Event = HandoffPanelEvent;
}

impl warpui::TypedActionView for HandoffPanel {
    type Action = HandoffPanelAction;

    fn handle_action(&mut self, action: &HandoffPanelAction, ctx: &mut ViewContext<Self>) {
        match action {
            HandoffPanelAction::Refresh => {
                let cwd = resolve_cwd();
                self.model.update(ctx, |m, ctx| {
                    m.load(cwd, ctx);
                });
            }
            HandoffPanelAction::ToggleExpand(id) => {
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

impl View for HandoffPanel {
    fn ui_name() -> &'static str {
        "HandoffPanel"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let model = self.model.as_ref(app);

        let cwd_label = model
            .cwd
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Handoff");
        let project_label = format!("Handoff — {cwd_label}");

        let item_count = model.items.len();
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
            ctx.dispatch_typed_action(HandoffPanelAction::Refresh);
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
                                    project_label,
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
        .with_padding_bottom(8.)
        .finish();

        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();

        let body: Box<dyn Element> = match &model.load_state {
            LoadState::NotLoaded | LoadState::Loading => Container::new(
                Text::new("Loading\u{2026}", appearance.ui_font_family(), 12.)
                    .with_color(sub_color)
                    .finish(),
            )
            .with_padding_left(10.)
            .with_padding_top(8.)
            .finish(),

            LoadState::Error(e) => {
                let err_text = format!("Error: {e}");
                Container::new(
                    Text::new(err_text, appearance.ui_font_family(), 11.)
                        .with_color(Fill::error().into_solid())
                        .finish(),
                )
                .with_padding_left(10.)
                .with_padding_top(8.)
                .finish()
            }

            LoadState::Loaded => {
                if model.items.is_empty() {
                    Container::new(
                        Text::new("No handoff items found.", appearance.ui_font_family(), 11.)
                            .with_color(sub_color)
                            .finish(),
                    )
                    .with_padding_left(10.)
                    .with_padding_top(8.)
                    .finish()
                } else {
                    let mut col = Flex::column();
                    for item in &model.items {
                        col = col.with_child(self.render_item(item, appearance, app));
                    }
                    Shrinkable::new(1.0, col.finish()).finish()
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
