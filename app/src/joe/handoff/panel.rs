use command::blocking::Command;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use warp_core::ui::theme::Fill;
use warp_core::ui::Icon;
use warpui::{
    elements::{
        ChildView, ClippedScrollStateHandle, ClippedScrollable, ConstrainedBox, Container,
        CrossAxisAlignment, Element, Flex, Hoverable, MainAxisSize, MouseStateHandle,
        ParentElement, ScrollbarWidth, Shrinkable, Text,
    },
    ui_components::components::UiComponent,
    AppContext, Entity, SingletonEntity, View, ViewContext, ViewHandle,
};

use crate::appearance::Appearance;
use crate::editor::{EditorOptions, EditorView, Event as EditorEvent, TextOptions};
use crate::ui_components::buttons::icon_button;

use super::model::{HandoffItem, HandoffModel, HandoffModelEvent, LoadState};

const SCRATCHPAD_MAX_HEIGHT: f32 = 24.;

/// Blue color for issue badges.
fn issue_badge_color() -> warpui::color::ColorU {
    warpui::color::ColorU::new(66, 133, 244, 255)
}

pub struct HandoffPanel {
    model: warpui::ModelHandle<HandoffModel>,
    expanded: HashSet<String>,
    refresh_mouse_state: MouseStateHandle,
    sync_mouse_state: MouseStateHandle,
    scroll_state: ClippedScrollStateHandle,
    /// Single-line editor for hj commands.
    scratchpad_editor: ViewHandle<EditorView>,
}

#[derive(Clone, Debug)]
pub enum HandoffPanelAction {
    Refresh,
    Sync,
    ToggleExpand(String),
    OpenIssue(String, u64),
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

        ctx.subscribe_to_model(&model, |me, _, event, ctx| match event {
            HandoffModelEvent::Loaded | HandoffModelEvent::Error(_) => ctx.notify(),
            HandoffModelEvent::CommandDone => {
                // After a command completes, refresh the item list
                let cwd = resolve_cwd();
                me.model.update(ctx, |m, ctx| {
                    m.load(cwd, ctx);
                });
                ctx.notify();
            }
        });

        let cwd = resolve_cwd();
        log::info!("[handoff] HandoffPanel::new cwd={cwd:?}");
        model.update(ctx, |m, ctx| {
            m.load(cwd, ctx);
        });

        // Create single-line editor for the scratchpad
        let scratchpad_editor = ctx.add_typed_action_view(|ctx| {
            let appearance = Appearance::as_ref(ctx);
            let options = EditorOptions {
                text: TextOptions::ui_text(Some(10.), appearance),
                single_line: true,
                autogrow: false,
                ..Default::default()
            };
            let mut editor = EditorView::new(options, ctx);
            editor.set_placeholder_text("hj ...", ctx);
            editor
        });

        ctx.subscribe_to_view(&scratchpad_editor, |me, _, event, ctx| {
            me.handle_editor_event(event, ctx);
        });

        Self {
            model,
            expanded: HashSet::new(),
            refresh_mouse_state: MouseStateHandle::default(),
            sync_mouse_state: MouseStateHandle::default(),
            scroll_state: ClippedScrollStateHandle::default(),
            scratchpad_editor,
        }
    }

    fn handle_editor_event(&mut self, event: &EditorEvent, ctx: &mut ViewContext<Self>) {
        if let EditorEvent::Enter = event {
            let buffer_text = self.scratchpad_editor.as_ref(ctx).buffer_text(ctx);
            let trimmed = buffer_text.trim().to_string();
            if !trimmed.is_empty() {
                // Clear the editor
                self.scratchpad_editor.update(ctx, |editor, ctx| {
                    editor.set_buffer_text("", ctx);
                });
                // Run the command
                self.model.update(ctx, |m, ctx| {
                    m.run_command(trimmed, ctx);
                });
            }
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
        #[allow(clippy::manual_unwrap_or)]
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

    fn render_issue_badge(item: &HandoffItem, appearance: &Appearance) -> Option<Box<dyn Element>> {
        let issue_num = item.issue_number?;
        let repo = item.issue_repo.clone()?;
        let label = format!("#{issue_num}");
        let badge_mouse = MouseStateHandle::default();

        let badge = Container::new(
            Text::new(label, appearance.ui_font_family(), 9.)
                .with_color(Fill::Solid(issue_badge_color()).into_solid())
                .finish(),
        )
        .with_padding_left(4.)
        .finish();

        let clickable = Hoverable::new(badge_mouse, move |_| badge)
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(HandoffPanelAction::OpenIssue(repo.clone(), issue_num));
            })
            .finish();

        Some(clickable)
    }

    fn render_item(&self, item: &HandoffItem, appearance: &Appearance) -> Box<dyn Element> {
        let is_expanded = self.expanded.contains(&item.id);
        let item_id = item.id.clone();
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();

        let title_text = item.title.clone().unwrap_or_else(|| item.id.clone());

        let mut row = Flex::row()
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
            );

        // Add issue badge if linked
        if let Some(badge) = Self::render_issue_badge(item, appearance) {
            row = row.with_child(badge);
        }

        let row = row.with_main_axis_size(MainAxisSize::Max).finish();

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
        project_order: &[String],
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let mut col = Flex::column().with_child(Self::render_section_header(title, appearance));

        let mut by_project: HashMap<String, Vec<&HandoffItem>> = HashMap::new();
        let mut seen: Vec<String> = Vec::new();
        for item in items {
            let proj = warpx::handoff::project_from_source(&item.source_file).to_string();
            if !by_project.contains_key(&proj) {
                seen.push(proj.clone());
            }
            by_project.entry(proj).or_default().push(item);
        }

        let mut ordered: Vec<String> = project_order
            .iter()
            .filter(|p| by_project.contains_key(*p))
            .cloned()
            .collect();
        for proj in &seen {
            if !ordered.contains(proj) {
                ordered.push(proj.clone());
            }
        }

        for proj in &ordered {
            col = col.with_child(Self::render_project_header(proj, appearance));
            for item in &by_project[proj] {
                col = col.with_child(self.render_item(item, appearance));
            }
        }

        col.finish()
    }

    fn render_command_output(
        result: &warpx::handoff::HjCommandResult,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();

        let output = if !result.stdout.is_empty() {
            result.stdout.trim().to_string()
        } else if !result.stderr.is_empty() {
            result.stderr.trim().to_string()
        } else {
            "(no output)".to_string()
        };

        // Truncate long output
        let display = if output.len() > 500 {
            format!("{}\n...(truncated)", &output[..500])
        } else {
            output
        };

        let color = if result.success {
            sub_color
        } else {
            Fill::error().into_solid()
        };

        let mut col = Flex::column();

        // Show the command that was run
        col = col.with_child(
            Container::new(
                Text::new(
                    format!("> {}", result.command),
                    appearance.ui_font_family(),
                    9.,
                )
                .with_color(sub_color)
                .finish(),
            )
            .with_padding_left(10.)
            .with_padding_right(10.)
            .with_padding_top(4.)
            .finish(),
        );

        col = col.with_child(
            Container::new(
                Text::new(display, appearance.ui_font_family(), 9.)
                    .with_color(color)
                    .finish(),
            )
            .with_padding_left(10.)
            .with_padding_right(10.)
            .with_padding_top(2.)
            .with_padding_bottom(6.)
            .finish(),
        );

        col.finish()
    }

    fn render_scratchpad(&self, appearance: &Appearance) -> Box<dyn Element> {
        Flex::column()
            .with_child(
                Container::new(
                    Text::new("hj", appearance.ui_font_family(), 10.)
                        .with_color(appearance.theme().foreground().into_solid())
                        .finish(),
                )
                .with_padding_left(10.)
                .with_padding_top(8.)
                .with_padding_bottom(2.)
                .finish(),
            )
            .with_child(
                Container::new(
                    ConstrainedBox::new(ChildView::new(&self.scratchpad_editor).finish())
                        .with_max_height(SCRATCHPAD_MAX_HEIGHT)
                        .finish(),
                )
                .with_padding_left(10.)
                .with_padding_right(10.)
                .with_padding_bottom(8.)
                .with_background(appearance.theme().surface_2())
                .finish(),
            )
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
            HandoffPanelAction::Sync => {
                self.model.update(ctx, |m, ctx| {
                    m.sync(ctx);
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
            HandoffPanelAction::OpenIssue(repo, issue_number) => {
                let url = format!("https://github.com/{repo}/issues/{issue_number}");
                if let Err(e) = Command::new("open").arg(&url).spawn() {
                    log::warn!("[handoff] failed to open {url}: {e}");
                }
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

        let sync_btn = icon_button(
            appearance,
            Icon::RefreshCw04,
            false,
            self.sync_mouse_state.clone(),
        )
        .build()
        .on_click(|ctx, _, _| {
            ctx.dispatch_typed_action(HandoffPanelAction::Sync);
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
                                    .with_color(sub_color)
                                    .finish(),
                            )
                            .finish(),
                    )
                    .finish(),
                )
                .with_child(sync_btn)
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
                    let theme = appearance.theme();
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
                    let project_order = &model.project_order;
                    if !open.is_empty() {
                        col = col.with_child(self.render_group(
                            "Open",
                            &open,
                            project_order,
                            appearance,
                        ));
                    }
                    if !blocked.is_empty() {
                        col = col.with_child(self.render_group(
                            "Blocked",
                            &blocked,
                            project_order,
                            appearance,
                        ));
                    }
                    if !done.is_empty() {
                        col = col.with_child(self.render_group(
                            "Done",
                            &done,
                            project_order,
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

        // Command output area
        let command_output: Box<dyn Element> = if let Some(result) = &model.last_command {
            Self::render_command_output(result, appearance)
        } else {
            // Empty placeholder
            Container::new(
                Text::new(String::new(), appearance.ui_font_family(), 1.)
                    .with_color(sub_color)
                    .finish(),
            )
            .finish()
        };

        // Scratchpad
        let scratchpad = self.render_scratchpad(appearance);

        Flex::column()
            .with_child(header)
            .with_child(Shrinkable::new(1.0, body).finish())
            .with_child(command_output)
            .with_child(scratchpad)
            .with_main_axis_size(MainAxisSize::Max)
            .finish()
    }
}
