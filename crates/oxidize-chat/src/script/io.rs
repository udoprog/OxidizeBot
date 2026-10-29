use rune::runtime::{InstAddress, Memory, Output, VmResult};
use rune::{ContextError, Module};

/// Construct the necessary io shims suitable for use within the bot.
pub(crate) fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_item(["std", "io"])?;
    m.raw_function("dbg", dbg_impl).build()?;
    m.function("print", print_impl).build()?;
    m.function("println", println_impl).build()?;
    Ok(m)
}

fn dbg_impl(stack: &mut dyn Memory, addr: InstAddress, args: usize, out: Output) -> VmResult<()> {
    let mut string = String::new();

    for (n, value) in rune::vm_try!(stack.slice_at(addr, args)).iter().enumerate() {
        if n > 0 {
            string.push('\n');
        }

        string.push_str(&format!("{value:?}"));
    }

    tracing::info!("[dbg]: {}", string);
    rune::vm_try!(out.store(stack, ()));
    VmResult::Ok(())
}

fn print_impl(m: &str) {
    tracing::info!("[out]: {}", m);
}

fn println_impl(m: &str) {
    tracing::info!("[out]: {}", m);
}
