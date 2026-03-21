use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;

use crate::config::app::{RENDER_FRAME_RATE_MS, TICK_RATE_MS};
use crate::game::{Direction, GameState, RunState};
use crate::render::{self, board_size_for_terminal, is_terminal_too_small};

/// 应用层状态，负责协调终端、输入和游戏循环。
pub struct App {
    game: GameState,
    render_state: render::RenderState,
    should_quit: bool,
    window_too_small: bool,
    no_color: bool,
}

impl App {
    /// 创建应用实例，并初始化默认游戏状态。
    pub fn new(no_color: bool, test_ai: bool) -> Self {
        let mut game = GameState::new();
        game.set_player_ai_control(test_ai);

        Self {
            game,
            render_state: render::RenderState::new(),
            should_quit: false,
            window_too_small: false,
            no_color,
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

    /// 驱动输入处理、固定逻辑 tick 和独立渲染帧。
    ///
    /// 主循环流程：
    /// - 计算下次 tick 和渲染的时间点
    /// - 等待事件或超时
    /// - 处理输入事件
    /// - 如果到达 tick 时间，推进游戏逻辑
    /// - 如果到达渲染时间，绘制界面
    /// - 循环直到收到退出信号
    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(TICK_RATE_MS);
        let render_rate = Duration::from_millis(RENDER_FRAME_RATE_MS);
        let mut last_render_frame = Instant::now()
            .checked_sub(render_rate)
            .unwrap_or_else(Instant::now);

        while !self.should_quit {
            let now = Instant::now();
            let next_tick_at = last_tick + tick_rate;
            let next_render_at = last_render_frame + render_rate;
            let next_update_at = next_tick_at.min(next_render_at);
            let timeout = next_update_at.saturating_duration_since(now);

            if event::poll(timeout)? {
                self.handle_event(event::read()?)?;
            }

            let now = Instant::now();
            if self.window_too_small {
                last_tick = now;
            } else {
                while now.duration_since(last_tick) >= tick_rate {
                    self.game.tick();
                    last_tick += tick_rate;
                }
            }

            self.render_state.sync(&self.game, now);

            let mut should_render = false;
            while now.duration_since(last_render_frame) >= render_rate {
                last_render_frame += render_rate;
                should_render = true;
            }

            if should_render {
                self.draw_frame(terminal, now)?;
            }
        }

        Ok(())
    }

    /// 按当前状态绘制一帧界面。
    fn draw_frame(
        &self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        now: Instant,
    ) -> Result<()> {
        terminal.draw(|frame| {
            render::draw(
                frame,
                &self.game,
                &self.render_state,
                now,
                self.window_too_small,
                self.no_color,
            )
        })?;
        Ok(())
    }

    /// 按当前终端可用区域重新计算棋盘尺寸，并重开一局。
    fn resize_game_to_terminal(&mut self, area: Rect) {
        self.window_too_small = is_terminal_too_small(area.width, area.height);
        if self.window_too_small {
            return;
        }

        let (width, height) = board_size_for_terminal(area.width, area.height);
        self.game.restart_with_board_size(width, height);
    }

    /// 统一处理键盘和窗口尺寸变化事件。
    ///
    /// **事件类型处理**：
    ///
    /// - **键盘事件 (Key)**：
    ///   - 仅处理 `KeyEventKind::Press` 类型，忽略释放和重复事件
    ///   - 终端窗口过小时，只响应 'q' 退出键
    ///
    /// - **方向控制**（WASD 或方向键）：
    ///   - 在 Ready 状态下：输入方向键会同时启动游戏
    ///   - 在 Running 状态下：仅更新方向
    ///   - 在 Paused/GameOver 状态下：输入方向键会启动游戏（从 Ready 开始）
    ///   - 禁止直接掉头（180度转向），由 `set_direction` 内部处理
    ///
    /// - **开始/暂停**（Space 或 Enter）：
    ///   - Ready 状态 -> 开始游戏
    ///   - Running 状态 -> 切换到暂停
    ///   - Paused 状态 -> 继续游戏
    ///   - GameOver 状态 -> 无操作
    ///
    /// - **重新开始**（r）：
    ///   - 立即重置游戏到 Ready 状态，使用当前棋盘尺寸
    ///
    /// - **退出**（q）：
    ///   - 设置 `should_quit = true`，下次循环检测到后会退出
    ///
    /// - **窗口调整**（Resize）：
    ///   - 终端窗口大小改变时，重新计算棋盘尺寸
    ///   - 如果新尺寸过小，显示提示而非游戏界面
    ///   - 窗口调整会自动重开一局新游戏
    fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }

                if self.window_too_small {
                    if key.code == KeyCode::Char('q') {
                        self.should_quit = true;
                    }
                    return Ok(());
                }

                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Enter => self.game.start(),
                    KeyCode::Char(' ') => {
                        if self.game.run_state() == RunState::Ready {
                            self.game.start();
                        } else {
                            self.game.toggle_pause();
                        }
                    }
                    KeyCode::Char('r') => self.game.restart(),
                    KeyCode::Up | KeyCode::Char('w') => {
                        if self.game.run_state() == RunState::Ready {
                            self.game.start();
                        }
                        self.game.set_direction(Direction::Up);
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        if self.game.run_state() == RunState::Ready {
                            self.game.start();
                        }
                        self.game.set_direction(Direction::Down);
                    }
                    KeyCode::Left | KeyCode::Char('a') => {
                        if self.game.run_state() == RunState::Ready {
                            self.game.start();
                        }
                        self.game.set_direction(Direction::Left);
                    }
                    KeyCode::Right | KeyCode::Char('d') => {
                        if self.game.run_state() == RunState::Ready {
                            self.game.start();
                        }
                        self.game.set_direction(Direction::Right);
                    }
                    _ => {}
                }
            }
            Event::Resize(width, height) => {
                self.resize_game_to_terminal(Rect::new(0, 0, width, height));
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
