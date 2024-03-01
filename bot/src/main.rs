#![cfg_attr(all(windows, not(feature = "cli")), windows_subsystem = "windows")]

pub(crate) fn main() -> anyhow::Result<()> {
    oxidize::cli::main()
}
