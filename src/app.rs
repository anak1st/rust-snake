use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use crate::game::{Direction, GameState};
use crate::render::{self, board_size_for_terminal};

/// 控制游戏逻辑推进频率，决定蛇移动速度。
const TICK_RATE: Duration = Duration::from_millis(160);

/// 应用层状态，负责协调终端、输入和游戏循环。
pub struct App {
    game: GameState,
    should_quit: bool,
}

impl App {
    /// 创建应用实例，并初始化默认游戏状态。
    pub fn new() -> Self {
        Self {
            game: GameState::new(),
            should_quit: false,
        }
    }

    /// 初始化终端环境并运行主循环，退出时负责恢复终端状态。
    pub fn run(&mut self) -> Result<()> {
        let mut terminal = setup_terminal()?;
        self.resize_game_to_terminal(terminal.size()?.into());
        let result = self.run_loop(&mut terminal);
        restore_terminal()?;
        result
    }

    /// 驱动渲染、输入处理和固定 tick 更新。
    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let mut last_tick = Instant::now();

        while !self.should_quit {
            terminal.draw(|frame| render::draw(frame, &self.game))?;

            let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                self.handle_event(event::read()?)?;
            }

            if last_tick.elapsed() >= TICK_RATE {
                self.game.tick();
                last_tick = Instant::now();
            }
        }

        Ok(())
    }

    /// 按当前终端可用区域重新计算棋盘尺寸，并重开一局。
    fn resize_game_to_terminal(&mut self, area: Rect) {
        let (width, height) = board_size_for_terminal(area.width, area.height);
        self.game.restart_with_board_size(width, height);
    }

    /// 统一处理键盘和窗口尺寸变化事件。
    fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }

                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char(' ') => self.game.toggle_pause(),
                    KeyCode::Char('r') => self.game.restart(),
                    KeyCode::Up | KeyCode::Char('w') => self.game.set_direction(Direction::Up),
                    KeyCode::Down | KeyCode::Char('s') => self.game.set_direction(Direction::Down),
                    KeyCode::Left | KeyCode::Char('a') => self.game.set_direction(Direction::Left),
                    KeyCode::Right | KeyCode::Char('d') => self.game.set_direction(Direction::Right),
                    _ => {}
                }
            }
            Event::Resize(width, height) => {
                let (board_width, board_height) = board_size_for_terminal(width, height);
                self.game.restart_with_board_size(board_width, board_height);
            }
            _ => {}
        }

        Ok(())
    }
}

/// 开启 raw mode 和备用屏，创建 ratatui 终端实例。
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// 恢复终端模式，避免程序退出后终端状态异常。
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
