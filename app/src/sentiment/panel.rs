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

use super::model::{LoadState, SentimentEntry, SentimentModel, SentimentModelEvent};

pub struct SentimentPanel {
    model: warpui::ModelHandle<SentimentModel>,
    refresh_mouse_state: MouseStateHandle,
}

#[derive(Clone, Debug)]
pub enum SentimentPanelAction {
    Refresh,
}

#[derive(Clone, Debug)]
pub enum SentimentPanelEvent {
    Refreshed,
}

impl SentimentPanel {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let model = ctx.add_model(SentimentModel::new);

        ctx.subscribe_to_model(&model, |_me, _, event, ctx| match event {
            SentimentModelEvent::Updated | SentimentModelEvent::Error(_) => ctx.notify(),
        });

        model.update(ctx, |m, ctx| {
            m.load(ctx);
        });

        Self {
            model,
            refresh_mouse_state: MouseStateHandle::default(),
        }
    }

    fn render_sentiment_badge(sentiment: &str, appearance: &Appearance) -> Box<dyn Element> {
        let color = match sentiment {
            "VeryPositive" | "Positive" => Fill::success().into_solid(),
            "Neutral" => appearance
                .theme()
                .sub_text_color(appearance.theme().background())
                .into_solid(),
            "Negative" | "VeryNegative" => Fill::error().into_solid(),
            _ => appearance
                .theme()
                .sub_text_color(appearance.theme().background())
                .into_solid(),
        };
        Container::new(
            Text::new(sentiment.to_string(), appearance.ui_font_family(), 11.)
                .with_color(color)
                .finish(),
        )
        .with_padding_left(4.)
        .with_padding_right(4.)
        .finish()
    }

    fn render_mood_badge(mood: &str, appearance: &Appearance) -> Box<dyn Element> {
        let color = appearance.theme().foreground().into_solid();
        Container::new(
            Text::new(mood.to_string(), appearance.ui_font_family(), 10.)
                .with_color(color)
                .finish(),
        )
        .with_padding_left(4.)
        .with_padding_right(4.)
        .finish()
    }

    fn render_confidence_bar(confidence: f64, appearance: &Appearance) -> Box<dyn Element> {
        let pct = (confidence * 100.0) as u32;
        let label = format!("{pct}%");
        let color = if confidence >= 0.8 {
            Fill::success().into_solid()
        } else if confidence >= 0.5 {
            Fill::warn().into_solid()
        } else {
            Fill::error().into_solid()
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

    fn render_current(&self, entry: &SentimentEntry, appearance: &Appearance) -> Box<dyn Element> {
        let sentiment_badge = Self::render_sentiment_badge(&entry.sentiment, appearance);
        let mood_badge = Self::render_mood_badge(&entry.mood, appearance);
        let confidence_bar = Self::render_confidence_bar(entry.confidence, appearance);

        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();

        let severity_label = Text::new(
            format!("Severity: {}", entry.severity),
            appearance.ui_font_family(),
            10.,
        )
        .with_color(sub_color)
        .finish();

        let text_preview = if entry.text.len() > 80 {
            format!("{}...", &entry.text[..80])
        } else {
            entry.text.clone()
        };
        let text_label = Text::new(text_preview, appearance.ui_font_family(), 10.)
            .with_color(sub_color)
            .finish();

        Flex::column()
            .with_child(
                Flex::row()
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .with_child(sentiment_badge)
                    .with_child(mood_badge)
                    .with_child(confidence_bar)
                    .with_main_axis_size(MainAxisSize::Max)
                    .finish(),
            )
            .with_child(
                Container::new(severity_label)
                    .with_padding_left(4.)
                    .with_padding_top(2.)
                    .finish(),
            )
            .with_child(
                Container::new(text_label)
                    .with_padding_left(4.)
                    .with_padding_top(4.)
                    .finish(),
            )
            .finish()
    }

    fn render_history_entry(entry: &SentimentEntry, appearance: &Appearance) -> Box<dyn Element> {
        let sub_color = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into_solid();

        let sentiment_color = match entry.sentiment.as_str() {
            "VeryPositive" | "Positive" => Fill::success().into_solid(),
            "Negative" | "VeryNegative" => Fill::error().into_solid(),
            _ => sub_color,
        };

        let text_preview = if entry.text.len() > 50 {
            format!("{}...", &entry.text[..50])
        } else {
            entry.text.clone()
        };

        Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Container::new(
                        Text::new(entry.sentiment.clone(), appearance.ui_font_family(), 9.)
                            .with_color(sentiment_color)
                            .finish(),
                    )
                    .with_padding_right(6.)
                    .finish(),
                )
                .with_child(
                    Shrinkable::new(
                        1.0,
                        Text::new(text_preview, appearance.ui_font_family(), 9.)
                            .with_color(sub_color)
                            .finish(),
                    )
                    .finish(),
                )
                .with_main_axis_size(MainAxisSize::Max)
                .finish(),
        )
        .with_padding_top(3.)
        .with_padding_bottom(3.)
        .with_padding_left(8.)
        .with_padding_right(8.)
        .finish()
    }
}

impl Entity for SentimentPanel {
    type Event = SentimentPanelEvent;
}

impl warpui::TypedActionView for SentimentPanel {
    type Action = SentimentPanelAction;

    fn handle_action(&mut self, action: &SentimentPanelAction, ctx: &mut ViewContext<Self>) {
        match action {
            SentimentPanelAction::Refresh => {
                self.model.update(ctx, |m, ctx| {
                    m.load(ctx);
                });
            }
        }
    }
}

impl View for SentimentPanel {
    fn ui_name() -> &'static str {
        "SentimentPanel"
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
            ctx.dispatch_typed_action(SentimentPanelAction::Refresh);
        })
        .finish();

        let header = Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Shrinkable::new(
                        1.0,
                        Flex::column()
                            .with_child(
                                Text::new(
                                    "Sentiment",
                                    appearance.ui_font_family(),
                                    appearance.ui_font_size(),
                                )
                                .with_color(appearance.theme().foreground().into_solid())
                                .finish(),
                            )
                            .with_child(
                                Text::new(
                                    model.state.status.clone(),
                                    appearance.ui_font_family(),
                                    10.,
                                )
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
                    format!("No sentiment data: {e}"),
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
                let mut col = Flex::column();

                // Current sentiment
                if let Some(ref current) = model.state.current {
                    col = col.with_child(
                        Container::new(self.render_current(current, appearance))
                            .with_padding_left(8.)
                            .with_padding_right(8.)
                            .with_padding_top(4.)
                            .with_padding_bottom(8.)
                            .finish(),
                    );
                }

                // History section
                if !model.state.history.is_empty() {
                    col = col.with_child(
                        Container::new(
                            Text::new("History", appearance.ui_font_family(), 10.)
                                .with_color(sub_color)
                                .finish(),
                        )
                        .with_padding_left(10.)
                        .with_padding_top(6.)
                        .with_padding_bottom(4.)
                        .finish(),
                    );

                    for entry in model.state.history.iter().rev().take(10) {
                        col = col.with_child(Self::render_history_entry(entry, appearance));
                    }
                }

                if model.state.current.is_none() && model.state.history.is_empty() {
                    col = col.with_child(
                        Container::new(
                            Text::new(
                                "No sentiment data yet. Run a looprs session to populate.",
                                appearance.ui_font_family(),
                                11.,
                            )
                            .with_color(sub_color)
                            .finish(),
                        )
                        .with_padding_left(10.)
                        .with_padding_top(8.)
                        .finish(),
                    );
                }

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
