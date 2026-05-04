use crate::terminal::model::session::Session;
use smol_str::SmolStr;
use std::sync::Arc;
use warp_completer::parsers::simple::all_parsed_commands;
use warp_util::path::EscapeChar;

/// Contains information about an aliased command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AliasedCommand {
    /// The alias itself.
    pub alias: SmolStr,
    /// The value that the alias is mapped to.
    pub alias_value: String,
}

/// Returns whether the alias can be expanded by Warp given its value.
///
/// We don't expand on any alias that starts with itself, as it leads to
/// cases where the alias is expanded twice: once as the user types in the
/// editor and again by the shell when the command is entered.
pub fn is_expandable_alias(alias: &str, alias_value: &str) -> bool {
    let commands: Vec<_> = all_parsed_commands(alias_value, EscapeChar::Backslash).collect();
    if commands.is_empty() {
        return false;
    }
    !commands
        .into_iter()
        .any(|cmd| cmd.parts.first().map(|p| p.item.as_str()) == Some(alias))
}

/// Searches the source text for any aliases in a command position. Returns
/// information about the alias if found.
fn check_for_alias(source: &str, session: Arc<Session>) -> Option<AliasedCommand> {
    let mut all_commands_iterator =
        all_parsed_commands(source, session.shell_family().escape_char());
    all_commands_iterator.find_map(|command| {
        let first_token = command.parts.first()?;
        let alias_value = session.alias_value(&first_token.item)?;
        is_expandable_alias(&first_token.item, alias_value).then(|| AliasedCommand {
            alias: first_token.item.as_str().into(),
            alias_value: alias_value.into(),
        })
    })
}

pub async fn check_for_alias_async(source: &str, session: Arc<Session>) -> Option<AliasedCommand> {
    check_for_alias(source, session)
}

#[cfg(test)]
#[path = "alias_tests.rs"]
pub mod tests;
