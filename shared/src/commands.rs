use serde::{Deserialize, Serialize};
use std::error::Error;

const COMMANDS: &str = include_str!("../commands.toml");

#[derive(Serialize, Deserialize)]
pub struct Command {
    /// The name of the command.
    pub name: String,
    /// Description of the command.
    pub description: String,
    /// Examples in markdown.
    #[serde(default)]
    pub examples: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CommandGroup {
    /// The group a command belongs to.
    pub name: String,
    /// A list of commands in this group.
    #[serde(default)]
    pub commands: Vec<Command>,
}

#[derive(Serialize, Deserialize)]
pub struct Commands {
    /// All commands, mapped by group id.
    #[serde(default)]
    pub groups: Vec<CommandGroup>,
}

/// Load the static collection of commands.
pub fn load_commands() -> Result<Commands, Box<dyn Error>> {
    Ok(toml::from_str(COMMANDS)?)
}

#[cfg(test)]
mod tests {
    use super::load_commands;
    use std::error::Error;

    #[test]
    fn test_deserialize() -> Result<(), Box<dyn Error>> {
        let _ = load_commands()?;
        Ok(())
    }
}
