#![cfg_attr(feature = "windows", windows_subsystem = "windows")]

pub(crate) fn main() -> anyhow::Result<()> {
    oxidize::cli::main()
}
