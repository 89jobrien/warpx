use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use tempfile::NamedTempFile;
use warp_cli::agent::Harness;
use warp_managed_secrets::ManagedSecretValue;
use warpui::{ModelHandle, ModelSpawner};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::ambient_agents::AmbientAgentTaskId;
use crate::server::server_api::harness_support::HarnessSupportClient;
use crate::server::server_api::ServerApi;
use crate::terminal::model::block::BlockId;
use crate::terminal::CLIAgent;

use super::super::terminal::{CommandHandle, TerminalDriver};
use super::super::{AgentDriver, AgentDriverError};
use super::{write_temp_file, HarnessRunner, ResumePayload, SavePoint, ThirdPartyHarness};

/// Harness that delegates to `braid-agent`, a local CLI binary wrapping
/// the braid `Engine` for personal-fork agent orchestration.
pub(crate) struct JoeHarness;

/// Format slug sent to the server when creating a Joe conversation.
const JOE_CLI_FORMAT: &str = "braid_agent";
/// Command sent to gracefully shut down the braid agent.
const JOE_EXIT_COMMAND: &str = "/exit";

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl ThirdPartyHarness for JoeHarness {
    fn harness(&self) -> Harness {
        Harness::Joe
    }

    fn cli_agent(&self) -> CLIAgent {
        // Reuse the Unknown variant for now — braid-agent is not a
        // recognized upstream CLI agent and adding a CLIAgent variant
        // would touch even more exhaustive matches.
        CLIAgent::Unknown
    }

    fn build_runner(
        &self,
        prompt: &str,
        _system_prompt: Option<&str>,
        _resumption_prompt: Option<&str>,
        _working_dir: &Path,
        _task_id: Option<AmbientAgentTaskId>,
        server_api: Arc<ServerApi>,
        terminal_driver: ModelHandle<TerminalDriver>,
        _resume: Option<ResumePayload>,
    ) -> Result<Box<dyn HarnessRunner>, AgentDriverError> {
        let client: Arc<dyn HarnessSupportClient> = server_api;
        Ok(Box::new(JoeHarnessRunner::new(
            "braid-agent",
            prompt,
            client,
            terminal_driver,
        )?))
    }
}

fn joe_command(cli_name: &str, prompt_path: &str) -> String {
    format!("{cli_name} --prompt \"$(cat '{prompt_path}')\"")
}

enum JoeRunnerState {
    Preexec,
    Running {
        conversation_id: AIConversationId,
        block_id: BlockId,
    },
}

struct JoeHarnessRunner {
    command: String,
    _temp_prompt_file: NamedTempFile,
    client: Arc<dyn HarnessSupportClient>,
    terminal_driver: ModelHandle<TerminalDriver>,
    state: Mutex<JoeRunnerState>,
}

impl JoeHarnessRunner {
    fn new(
        cli_command: &str,
        prompt: &str,
        client: Arc<dyn HarnessSupportClient>,
        terminal_driver: ModelHandle<TerminalDriver>,
    ) -> Result<Self, AgentDriverError> {
        let temp_file = write_temp_file("oz_prompt_joe_", prompt)?;
        let prompt_path = temp_file.path().display().to_string();

        Ok(Self {
            command: joe_command(cli_command, &prompt_path),
            _temp_prompt_file: temp_file,
            client,
            terminal_driver,
            state: Mutex::new(JoeRunnerState::Preexec),
        })
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl HarnessRunner for JoeHarnessRunner {
    async fn start(
        &self,
        foreground: &ModelSpawner<AgentDriver>,
    ) -> Result<CommandHandle, AgentDriverError> {
        let conversation_id = self
            .client
            .create_external_conversation(JOE_CLI_FORMAT)
            .await
            .map_err(|e| {
                log::error!("Failed to create external conversation: {e}");
                AgentDriverError::ConfigBuildFailed(e)
            })?;
        log::info!("Created external conversation {conversation_id}");

        let command = self.command.clone();
        let terminal_driver = self.terminal_driver.clone();
        let command_handle = foreground
            .spawn(move |_, ctx| {
                terminal_driver.update(ctx, |driver, ctx| driver.execute_command(&command, ctx))
            })
            .await??
            .await?;

        *self.state.lock() = JoeRunnerState::Running {
            conversation_id,
            block_id: command_handle.block_id().clone(),
        };

        Ok(command_handle)
    }

    async fn exit(&self, foreground: &ModelSpawner<AgentDriver>) -> Result<()> {
        log::info!("Sending /exit to braid-agent");
        let terminal_driver = self.terminal_driver.clone();
        foreground
            .spawn(move |_, ctx| {
                terminal_driver.update(ctx, |driver, ctx| {
                    driver.send_text_to_cli(JOE_EXIT_COMMAND.to_string(), ctx);
                });
            })
            .await
            .map_err(|_| anyhow::anyhow!("Agent driver dropped while sending /exit"))
    }

    async fn save_conversation(
        &self,
        save_point: SavePoint,
        foreground: &ModelSpawner<AgentDriver>,
    ) -> Result<()> {
        if matches!(save_point, SavePoint::Periodic)
            && !super::has_running_cli_agent(&self.terminal_driver, foreground).await
        {
            log::debug!("Will not save conversation, braid-agent not in progress");
            return Ok(());
        }

        let (conversation_id, block_id) = match &*self.state.lock() {
            JoeRunnerState::Preexec => {
                log::warn!("save_conversation called before start");
                return Ok(());
            }
            JoeRunnerState::Running {
                conversation_id,
                block_id,
            } => (*conversation_id, block_id.clone()),
        };

        super::upload_current_block_snapshot(
            foreground,
            &self.terminal_driver,
            self.client.as_ref(),
            conversation_id,
            block_id,
        )
        .await
    }
}
