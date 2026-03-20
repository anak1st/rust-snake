mod app;
mod config;
mod game;
mod render;

use std::env;

use anyhow::Result;

use crate::app::App;

/// 启动应用并进入主游戏循环。
fn main() -> Result<()> {
    let no_color = env::args().any(|arg| arg == "-nocolor");
    App::new(no_color).run()
}
