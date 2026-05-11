use serde_json::{Map, Value};
use warpui::{Entity, ModelContext, SingletonEntity};

#[cfg(not(target_family = "wasm"))]
use crate::ai::mcp::{reconnecting_peer::ReconnectingPeer, TemplatableMCPServerManager};
pub use warpx::doob::DoobItem;

const DOOB_LIST_TOOL: &str = "doob_list";
const DOOB_COMPLETE_TOOL: &str = "doob_complete";
const DOOB_ADD_TOOL: &str = "doob_add";

/// Load state for the panel.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum LoadState {
    #[default]
    NotLoaded,
    Loading,
    Loaded,
    Error(String),
}
#[derive(Clone, Debug, Default, PartialEq)]
pub enum MutationState {
    #[default]
    Idle,
    Mutating,
    Success(String),
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AddDoobItem {
    pub title: String,
    pub priority: Option<u8>,
    pub due: Option<String>,
    pub repo: Option<String>,
}

#[cfg(not(target_family = "wasm"))]
fn doob_peer(tool_name: &str, ctx: &ModelContext<DoobModel>) -> Result<ReconnectingPeer, String> {
    TemplatableMCPServerManager::as_ref(ctx)
        .server_with_tool_name(tool_name.to_string())
        .ok_or_else(|| format!("MCP server for {tool_name} not found"))
}

#[cfg(target_family = "wasm")]
fn doob_peer(_: &str, _: &ModelContext<DoobModel>) -> Result<(), String> {
    Err("Doob MCP tools are not available on web".to_string())
}

#[cfg(not(target_family = "wasm"))]
async fn call_doob_tool(
    peer: ReconnectingPeer,
    tool_name: &str,
    arguments: Map<String, Value>,
) -> Result<rmcp::model::CallToolResult, String> {
    let result = peer
        .call_tool(rmcp::model::CallToolRequestParam {
            name: tool_name.to_string().into(),
            arguments: Some(arguments),
        })
        .await
        .map_err(|e| e.to_string())?;

    if matches!(result.is_error, Some(true)) {
        return Err(tool_result_text(&result)
            .unwrap_or_else(|| format!("MCP tool {tool_name} returned an error")));
    }

    Ok(result)
}

#[cfg(target_family = "wasm")]
async fn call_doob_tool(
    _: (),
    tool_name: &str,
    _: Map<String, Value>,
) -> Result<rmcp::model::CallToolResult, String> {
    Err(format!("MCP tool {tool_name} is not available on web"))
}

fn parse_doob_items(result: rmcp::model::CallToolResult) -> Result<Vec<DoobItem>, String> {
    let value = result
        .into_typed::<Value>()
        .map_err(|e| format!("failed to parse doob MCP result: {e}"))?;
    warpx::doob::parse_items_from_value(value)
}

fn tool_result_text(result: &rmcp::model::CallToolResult) -> Option<String> {
    result.content.iter().find_map(|content| {
        content
            .as_text()
            .map(|text| text.text.trim().to_string())
            .filter(|text| !text.is_empty())
    })
}

pub struct DoobModel {
    pub items: Vec<DoobItem>,
    pub load_state: LoadState,
    pub mutation_state: MutationState,
}

impl DoobModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            items: Vec::new(),
            load_state: LoadState::NotLoaded,
            mutation_state: MutationState::Idle,
        }
    }

    pub fn load(&mut self, ctx: &mut ModelContext<Self>) {
        self.load_state = LoadState::Loading;
        ctx.notify();
        let peer = match doob_peer(DOOB_LIST_TOOL, ctx) {
            Ok(peer) => peer,
            Err(e) => {
                self.load_state = LoadState::Error(e.clone());
                ctx.emit(DoobModelEvent::Error(e));
                ctx.notify();
                return;
            }
        };

        ctx.spawn(
            async move {
                let result = call_doob_tool(peer, DOOB_LIST_TOOL, Map::new()).await?;
                parse_doob_items(result)
            },
            |me, result: Result<Vec<DoobItem>, String>, ctx| match result {
                Ok(items) => {
                    me.items = items;
                    me.load_state = LoadState::Loaded;
                    ctx.emit(DoobModelEvent::Loaded);
                    ctx.notify();
                }
                Err(e) => {
                    me.load_state = LoadState::Error(e.clone());
                    ctx.emit(DoobModelEvent::Error(e));
                    ctx.notify();
                }
            },
        );
    }

    pub fn complete(&mut self, id: String, ctx: &mut ModelContext<Self>) {
        self.mutation_state = MutationState::Mutating;
        ctx.notify();

        let peer = match doob_peer(DOOB_COMPLETE_TOOL, ctx) {
            Ok(peer) => peer,
            Err(e) => {
                self.mutation_state = MutationState::Error(e.clone());
                ctx.emit(DoobModelEvent::Error(e));
                ctx.notify();
                return;
            }
        };

        ctx.spawn(
            async move {
                let mut arguments = Map::new();
                arguments.insert(
                    "ids".to_string(),
                    Value::Array(vec![Value::String(id.clone())]),
                );
                call_doob_tool(peer, DOOB_COMPLETE_TOOL, arguments).await?;
                Ok::<String, String>(id)
            },
            |me, result: Result<String, String>, ctx| match result {
                Ok(id) => {
                    me.mutation_state = MutationState::Success(format!("Completed {id}"));
                    ctx.emit(DoobModelEvent::Mutated);
                    me.load(ctx);
                    ctx.notify();
                }
                Err(e) => {
                    me.mutation_state = MutationState::Error(e.clone());
                    ctx.emit(DoobModelEvent::Error(e));
                    ctx.notify();
                }
            },
        );
    }

    pub fn add(&mut self, item: AddDoobItem, ctx: &mut ModelContext<Self>) {
        self.mutation_state = MutationState::Mutating;
        ctx.notify();

        let peer = match doob_peer(DOOB_ADD_TOOL, ctx) {
            Ok(peer) => peer,
            Err(e) => {
                self.mutation_state = MutationState::Error(e.clone());
                ctx.emit(DoobModelEvent::Error(e));
                ctx.notify();
                return;
            }
        };

        ctx.spawn(
            async move {
                let mut arguments = Map::new();
                arguments.insert("title".to_string(), Value::String(item.title.clone()));
                if let Some(priority) = item.priority {
                    arguments.insert("priority".to_string(), Value::Number(priority.into()));
                }
                if let Some(due) = item.due {
                    arguments.insert("due".to_string(), Value::String(due));
                }
                if let Some(repo) = item.repo {
                    arguments.insert("repo".to_string(), Value::String(repo));
                }
                call_doob_tool(peer, DOOB_ADD_TOOL, arguments).await?;
                Ok::<String, String>(item.title)
            },
            |me, result: Result<String, String>, ctx| match result {
                Ok(title) => {
                    me.mutation_state = MutationState::Success(format!("Added {title}"));
                    ctx.emit(DoobModelEvent::Mutated);
                    me.load(ctx);
                    ctx.notify();
                }
                Err(e) => {
                    me.mutation_state = MutationState::Error(e.clone());
                    ctx.emit(DoobModelEvent::Error(e));
                    ctx.notify();
                }
            },
        );
    }
}

impl Entity for DoobModel {
    type Event = DoobModelEvent;
}

#[derive(Clone, Debug)]
pub enum DoobModelEvent {
    Loaded,
    Mutated,
    #[allow(dead_code)]
    Error(String),
}
