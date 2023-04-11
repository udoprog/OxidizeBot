use std::fmt::Write;

use rune::runtime::{Stack, VmError};
use rune::{ContextError, Module, Value};

/// Construct the necessary io shims suitable for use within the bot.
pub(crate) fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_item(&["std", "io"]);
    m.raw_fn(&["dbg"], dbg_impl)?;
    m.function(&["print"], print_impl)?;
    m.function(&["println"], println_impl)?;
    Ok(m)
}

fn dbg_impl(stack: &mut Stack, args: usize) -> Result<(), VmError> {
    let mut string = String::new();

    let mut it = stack.drain(args)?;
    let last = it.next_back();

    for value in it {
        write!(string, "{:?}", value).map_err(VmError::panic)?;
        string.push('\n');
    }

    if let Some(value) = last {
        write!(string, "{:?}", value).map_err(VmError::panic)?;
    }

    tracing::info!("[dbg]: {}", string);
    stack.push(Value::Unit);
    Ok(())
}

fn print_impl(m: &str) {
    tracing::info!("[out]: {}", m);
}

fn println_impl(m: &str) {
    tracing::info!("[out]: {}", m);
}
