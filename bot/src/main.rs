#![recursion_limit = "256"]
#![type_length_limit = "4194304"]
#![cfg_attr(feature = "windows", windows_subsystem = "windows")]
#![cfg_attr(backtrace, feature(backtrace))]
#![allow(clippy::field_reassign_with_default)]

pub(crate) fn main() -> anyhow::Result<()> {
    oxidize::cli::main()
}
