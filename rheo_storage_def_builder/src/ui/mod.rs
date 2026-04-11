mod app;
mod input;
mod model;
mod render;

#[cfg(test)]
mod tests;

use std::io;
use std::path::PathBuf;

use crate::BuilderPaths;

pub(crate) fn run_shell(paths: BuilderPaths, log_path: PathBuf) -> io::Result<()> {
    let mut app = app::ShellApp::new(paths, log_path);
    ratatui::run(|terminal| app.run(terminal))
}
