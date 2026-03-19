mod app;
mod game;
mod render;

use anyhow::Result;

use crate::app::App;

/// 启动应用并进入主游戏循环。
fn main() -> Result<()> {
    App::new().run()
}
