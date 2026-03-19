mod app;
mod game;
mod render;

use anyhow::Result;

use crate::app::App;

fn main() -> Result<()> {
    App::new().run()
}
