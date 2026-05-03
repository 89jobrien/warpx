use std::collections::{HashMap, HashSet};
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
        let label = match priority {
            Some(p @ ("P0" | "P1" | "P2" | "P3")) => p,
            _ => "  ",
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

    fn render_item(&self, item: &HandoffItem, appearance: &Appearance) -> Box<dyn Element> {
        let is_expanded = self.expanded.contains(&item.id);
        let item_id = item.id.clone();
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();

        let title_text = item.title.clone().unwrap_or_else(|| item.id.clone());

        let row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Self::render_priority_badge(
                item.priority.as_deref(),
                appearance,
            ))
            .with_child(
                Shrinkable::new(
                    1.0,
                    Text::new(title_text, appearance.ui_font_family(), 11.)
                        .with_color(sub_color.clone())
                        .finish(),
                )
                .finish(),
            )
            .with_main_axis_size(MainAxisSize::Max)
            .finish();

        let row_mouse = MouseStateHandle::default();
        let row_id = item_id.clone();
        let clickable_row = Hoverable::new(row_mouse, move |_| row)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(HandoffPanelAction::ToggleExpand(row_id.clone()));
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

    /// Extract project name from source filename: `HANDOFF.<id>.<project>.yaml` → `<project>`.
    fn project_from_source(source: &str) -> &str {
        // Strip prefix and suffix, split by '.', second-to-last segment is project.
        let stripped = source
            .strip_prefix("HANDOFF.")
            .unwrap_or(source)
            .strip_suffix(".yaml")
            .unwrap_or(source);
        // e.g. "warpx.warpx" → last segment
        stripped.rsplit('.').next().unwrap_or(stripped)
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
        items: &[&HandoffItem],
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let mut col = Flex::column().with_child(Self::render_section_header(title, appearance));

        // Group by project, preserving first-seen order.
        let mut project_order: Vec<String> = Vec::new();
        let mut by_project: HashMap<String, Vec<&HandoffItem>> = HashMap::new();
        for item in items {
            let proj = Self::project_from_source(&item.source_file).to_string();
            if !by_project.contains_key(&proj) {
                project_order.push(proj.clone());
            }
            by_project.entry(proj).or_default().push(item);
        }

        for proj in &project_order {
            col = col.with_child(Self::render_project_header(proj, appearance));
            for item in &by_project[proj] {
                col = col.with_child(self.render_item(item, appearance));
            }
        }

        col.finish()
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
                                    "Handoff",
                                    appearance.ui_font_family(),
                                    appearance.ui_font_size(),
                                )
                                .with_color(appearance.theme().foreground().into_solid())
                                .finish(),
                            )
                            .with_child(
                                Text::new(count_text, appearance.ui_font_family(), 10.)
                                    .with_color(sub_color.clone())
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
                        Text::new("No handoff items found.", appearance.ui_font_family(), 11.)
                            .with_color(sub_color)
                            .finish(),
                    )
                    .with_padding_left(10.)
                    .with_padding_top(8.)
                    .finish()
                } else {
                    let open: Vec<&HandoffItem> = model
                        .items
                        .iter()
                        .filter(|i| i.status.as_deref() == Some("open"))
                        .collect();
                    let blocked: Vec<&HandoffItem> = model
                        .items
                        .iter()
                        .filter(|i| i.status.as_deref() == Some("blocked"))
                        .collect();
                    let done: Vec<&HandoffItem> = model
                        .items
                        .iter()
                        .filter(|i| i.status.as_deref() == Some("done"))
                        .collect();

                    let mut col = Flex::column();
                    if !open.is_empty() {
                        col = col.with_child(self.render_group("Open", &open, appearance));
                    }
                    if !blocked.is_empty() {
                        col = col.with_child(self.render_group("Blocked", &blocked, appearance));
                    }
                    if !done.is_empty() {
                        col = col.with_child(self.render_group("Done", &done, appearance));
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
