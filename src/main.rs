mod app;
mod config;
mod game;
mod render;

use std::env;

use anyhow::Result;

use crate::app::App;

/// 启动应用并进入主游戏循环。
fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let no_color = args.iter().any(|arg| arg == "-nocolor");
    let test_ai = args.iter().any(|arg| arg == "-testai");

    App::new(no_color, test_ai).run()
}
