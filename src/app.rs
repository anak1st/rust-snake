use std::collections::VecDeque;
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

use crate::config::app::{
    DEATH_REPLAY_FRAME_COUNT, RENDER_FRAME_RATE_MS, TEST_AI_TICK_RATE_MS, TICK_RATE_MS,
};
use crate::game::{Direction, GameState, RunState};
use crate::render::{self, ReplayStatus, board_size_for_terminal, is_terminal_too_small};

/// 应用层状态，负责协调终端、输入和游戏循环。
pub struct App {
    game: GameState,
    death_replay_frames: VecDeque<GameState>,
    death_replay_cursor: Option<usize>,
    is_replay_mode: bool,
    render_state: render::RenderState,
    should_quit: bool,
    window_too_small: bool,
    no_color: bool,
    tick_rate: Duration,
}

impl App {
    /// 创建应用实例，并初始化默认游戏状态。
    pub fn new(no_color: bool, test_ai: bool) -> Self {
        let mut game = GameState::new();
        game.set_player_ai_control(test_ai);

        Self {
            game,
            death_replay_frames: VecDeque::new(),
            death_replay_cursor: None,
            is_replay_mode: false,
            render_state: render::RenderState::new(),
            should_quit: false,
            window_too_small: false,
            no_color,
            tick_rate: Duration::from_millis(if test_ai {
                TEST_AI_TICK_RATE_MS
            } else {
                TICK_RATE_MS
            }),
        }
        .with_initialized_replay()
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
        let tick_rate = self.tick_rate;
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
            if self.window_too_small || self.game.run_state() != RunState::Running {
                last_tick = now;
            } else {
                while now.duration_since(last_tick) >= tick_rate {
                    let was_game_over = self.game.run_state() == RunState::GameOver;
                    self.game.tick();
                    self.record_replay_frame_after_tick(was_game_over);
                    last_tick += tick_rate;
                }
            }

            let active_game = self.active_game().clone();
            self.render_state.sync(&active_game, now);

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
                self.active_game(),
                &self.render_state,
                self.replay_status(),
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
        self.reset_replay_history();
    }

    /// 统一处理键盘和窗口尺寸变化事件。
    ///
    /// **事件类型处理**：
    ///
    /// - **键盘事件 (Key)**：
    ///   - 忽略 `KeyEventKind::Release`，保留按下与连发事件，避免方向键回放失效
    ///   - 终端窗口过小时，只响应 'q' 退出键
    ///
    /// - **方向控制**（WASD 或方向键）：
    ///   - 在 Ready 状态下：输入方向键会同时启动游戏
    ///   - 在 Running 状态下：仅更新方向
    ///   - 在 Paused/GameOver 状态下：输入方向键不会推进游戏
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
                if matches!(key.kind, KeyEventKind::Release) {
                    return Ok(());
                }

                if self.window_too_small {
                    if key.code == KeyCode::Char('q') {
                        self.should_quit = true;
                    }
                    return Ok(());
                }

                if self.is_replay_mode {
                    match key.code {
                        KeyCode::Left | KeyCode::Char('a') => {
                            self.step_replay_backward();
                            return Ok(());
                        }
                        KeyCode::Right | KeyCode::Char('d') => {
                            self.step_replay_forward();
                            return Ok(());
                        }
                        KeyCode::Esc | KeyCode::Char(' ') => {
                            self.exit_replay_mode();
                            return Ok(());
                        }
                        _ => {}
                    }
                }

                if self.game.run_state() == RunState::GameOver {
                    match key.code {
                        KeyCode::Left | KeyCode::Char('a') => {
                            self.enter_replay_mode();
                            self.step_replay_backward();
                            return Ok(());
                        }
                        KeyCode::Right | KeyCode::Char('d') => {
                            self.enter_replay_mode();
                            self.step_replay_forward();
                            return Ok(());
                        }
                        _ => {}
                    }
                }

                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Enter => {
                        if self.game.run_state() == RunState::GameOver {
                            self.enter_replay_mode();
                        } else {
                            self.game.start();
                        }
                    }
                    KeyCode::Char(' ') => match self.game.run_state() {
                        RunState::Ready => self.game.start(),
                        RunState::Running | RunState::Paused => self.game.toggle_pause(),
                        RunState::GameOver => self.enter_replay_mode(),
                    },
                    KeyCode::Char('r') => {
                        self.game.restart();
                        self.reset_replay_history();
                    }
                    KeyCode::Up | KeyCode::Char('w') => {
                        if self.game.run_state() == RunState::Ready {
                            self.game.start();
                        } else if self.game.run_state() != RunState::Running {
                            return Ok(());
                        }
                        self.game.set_direction(Direction::Up);
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        if self.game.run_state() == RunState::Ready {
                            self.game.start();
                        } else if self.game.run_state() != RunState::Running {
                            return Ok(());
                        }
                        self.game.set_direction(Direction::Down);
                    }
                    KeyCode::Left | KeyCode::Char('a') => {
                        if self.game.run_state() == RunState::Ready {
                            self.game.start();
                        } else if self.game.run_state() != RunState::Running {
                            return Ok(());
                        }
                        self.game.set_direction(Direction::Left);
                    }
                    KeyCode::Right | KeyCode::Char('d') => {
                        if self.game.run_state() == RunState::Ready {
                            self.game.start();
                        } else if self.game.run_state() != RunState::Running {
                            return Ok(());
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

    /// 返回当前应被渲染的游戏状态；回放时显示历史帧，否则显示实时对局。
    fn active_game(&self) -> &GameState {
        self.is_replay_mode
            .then_some(())
            .and(self.death_replay_cursor)
            .and_then(|cursor| self.death_replay_frames.get(cursor))
            .unwrap_or(&self.game)
    }

    /// 返回死亡回放当前帧位置，用于界面提示。
    fn replay_status(&self) -> Option<ReplayStatus> {
        self.is_replay_mode
            .then_some(())
            .and(self.death_replay_cursor)
            .map(|cursor| ReplayStatus {
                current_frame: cursor + 1,
                total_frames: self.death_replay_frames.len(),
            })
    }

    /// 创建实例后初始化一份可用的回放历史。
    fn with_initialized_replay(mut self) -> Self {
        self.reset_replay_history();
        self
    }

    /// 用当前实时游戏状态重建回放缓存。
    fn reset_replay_history(&mut self) {
        self.death_replay_frames.clear();
        self.death_replay_frames.push_back(self.game.clone());
        self.death_replay_cursor = None;
        self.is_replay_mode = false;
    }

    /// 在每次逻辑 tick 之后记录一份快照，并在死亡时开启回放浏览。
    fn record_replay_frame_after_tick(&mut self, was_game_over: bool) {
        self.death_replay_frames.push_back(self.game.clone());
        while self.death_replay_frames.len() > DEATH_REPLAY_FRAME_COUNT {
            self.death_replay_frames.pop_front();
        }

        if !was_game_over && self.game.run_state() == RunState::GameOver {
            self.death_replay_cursor = Some(self.death_replay_frames.len().saturating_sub(1));
        } else if self.game.run_state() != RunState::GameOver {
            self.exit_replay_mode();
            self.death_replay_cursor = None;
        }
    }

    /// 进入死亡回放模式，并默认停在最后一帧。
    fn enter_replay_mode(&mut self) {
        if self.game.run_state() != RunState::GameOver || self.death_replay_frames.is_empty() {
            return;
        }

        self.is_replay_mode = true;
        self.death_replay_cursor = Some(self.death_replay_frames.len().saturating_sub(1));
    }

    /// 退出死亡回放模式，回到结算后的游戏结束画面。
    fn exit_replay_mode(&mut self) {
        self.is_replay_mode = false;
    }

    /// 将死亡回放向前移动一帧。
    fn step_replay_backward(&mut self) {
        let Some(cursor) = self.death_replay_cursor else {
            return;
        };

        if cursor > 0 {
            self.death_replay_cursor = Some(cursor - 1);
        }
    }

    /// 将死亡回放向后移动一帧。
    fn step_replay_forward(&mut self) {
        let Some(cursor) = self.death_replay_cursor else {
            return;
        };

        if cursor + 1 < self.death_replay_frames.len() {
            self.death_replay_cursor = Some(cursor + 1);
        }
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

#[cfg(test)]
mod tests {
    use super::App;
    use crate::game::{GameState, RunState};
    use std::time::Instant;

    #[test]
    /// 验证玩家死亡后需要显式进入回放模式，并允许左右浏览历史。
    fn death_replay_can_be_entered_and_browsed_after_game_over() {
        let mut app = App::new(true, false);
        app.game = GameState::with_board_size(4, 4);
        app.reset_replay_history();
        app.game.start();

        while app.game.run_state() != RunState::GameOver {
            let was_game_over = app.game.run_state() == RunState::GameOver;
            app.game.tick();
            app.record_replay_frame_after_tick(was_game_over);
        }

        let last_cursor = app.death_replay_frames.len() - 1;
        assert!(!app.is_replay_mode);
        assert_eq!(app.death_replay_cursor, Some(last_cursor));
        assert!(app.replay_status().is_none());

        app.enter_replay_mode();
        assert!(app.is_replay_mode);
        assert_eq!(
            app.replay_status().map(|status| status.current_frame),
            Some(last_cursor + 1)
        );

        app.step_replay_backward();
        assert_eq!(app.death_replay_cursor, Some(last_cursor - 1));

        app.step_replay_forward();
        assert_eq!(app.death_replay_cursor, Some(last_cursor));

        app.exit_replay_mode();
        assert!(!app.is_replay_mode);
        assert!(app.replay_status().is_none());
    }

    #[test]
    /// 验证游戏结束后空转不会继续覆盖掉死亡前的回放历史。
    fn game_over_idle_does_not_overwrite_replay_history() {
        let mut app = App::new(true, false);
        app.game = GameState::with_board_size(4, 4);
        app.reset_replay_history();
        app.game.start();

        while app.game.run_state() != RunState::GameOver {
            let was_game_over = app.game.run_state() == RunState::GameOver;
            app.game.tick();
            app.record_replay_frame_after_tick(was_game_over);
        }

        let recorded_frames = app
            .death_replay_frames
            .iter()
            .map(|frame| (frame.tick_count(), frame.player().head(), frame.run_state()))
            .collect::<Vec<_>>();
        let mut last_tick = Instant::now() - app.tick_rate * 4;
        let now = Instant::now();

        if app.window_too_small || app.game.run_state() != RunState::Running {
            last_tick = now;
        } else {
            while now.duration_since(last_tick) >= app.tick_rate {
                let was_game_over = app.game.run_state() == RunState::GameOver;
                app.game.tick();
                app.record_replay_frame_after_tick(was_game_over);
                last_tick += app.tick_rate;
            }
        }

        let replay_after_idle = app
            .death_replay_frames
            .iter()
            .map(|frame| (frame.tick_count(), frame.player().head(), frame.run_state()))
            .collect::<Vec<_>>();

        assert_eq!(replay_after_idle, recorded_frames);
        assert!(last_tick <= now);
    }
}
