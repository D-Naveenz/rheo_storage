pub mod app;
pub mod capabilities;
pub mod command;
pub mod package;
pub mod process;
pub mod shell;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
