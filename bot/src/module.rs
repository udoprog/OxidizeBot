use crate::command;
use hashbrown::HashMap;

pub type Handlers = HashMap<String, Box<dyn command::Handler + Send + 'static>>;

mod countdown;

pub trait Module {
    /// Set up command handlers for this module.
    fn setup_command(&self, _: &mut Handlers) -> Result<(), failure::Error> {
        Ok(())
    }
}
