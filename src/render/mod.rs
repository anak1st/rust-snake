//! 渲染模块，负责将游戏状态组织为终端界面，并叠加短时动画效果。

use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::config::render::{
    FOOTER_HEIGHT, HEADER_HEIGHT, INFO_HEIGHT, MIN_BOARD_HEIGHT, MIN_BOARD_WIDTH,
};
use crate::game::{GameState, RunState};

mod board;
mod panels;
mod style;

const DEATH_FLASH_DURATION_MS: u64 = 720;
const DEATH_FLASH_INTERVAL_MS: u64 = 120;

/// 渲染层持有的短时动画状态。
pub struct RenderState {
    previous_run_state: RunState,
    death_flash: Option<DeathFlash>,
}

struct DeathFlash {
    started_at: Instant,
}

pub(crate) struct AnimationFrame {
    pub death_flash_visible: bool,
}

impl RenderState {
    /// 创建渲染状态容器。
    pub fn new() -> Self {
        Self {
            previous_run_state: RunState::Ready,
            death_flash: None,
        }
    }

    /// 按当前游戏状态推进渲染层动画。
    pub fn sync(&mut self, game: &GameState, now: Instant) {
        let run_state = game.run_state();
        if self.previous_run_state != RunState::GameOver && run_state == RunState::GameOver {
            self.death_flash = Some(DeathFlash { started_at: now });
        }

        if run_state != RunState::GameOver {
            self.death_flash = None;
        }

        if self
            .death_flash
            .as_ref()
            .is_some_and(|flash| flash.is_finished(now))
        {
            self.death_flash = None;
        }

        self.previous_run_state = run_state;
    }

    fn animation_frame(&self, now: Instant) -> AnimationFrame {
        AnimationFrame {
            death_flash_visible: self
                .death_flash
                .as_ref()
                .is_some_and(|flash| flash.is_visible(now)),
        }
    }
}

impl DeathFlash {
    fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started_at)
    }

    fn is_finished(&self, now: Instant) -> bool {
        self.elapsed(now) >= Duration::from_millis(DEATH_FLASH_DURATION_MS)
    }

    fn is_visible(&self, now: Instant) -> bool {
        let interval = Duration::from_millis(DEATH_FLASH_INTERVAL_MS).as_millis();
        let step = self.elapsed(now).as_millis() / interval.max(1);
        step % 2 == 0
    }
}

/// 根据当前游戏状态绘制整个界面，并叠加动画帧。
pub fn draw(
    frame: &mut Frame,
    game: &GameState,
    render_state: &RenderState,
    now: Instant,
    window_too_small: bool,
    no_color: bool,
) {
    if window_too_small {
        panels::draw_too_small(frame, no_color, centered_area);
        return;
    }

    let [header_area, body_area, footer_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Min(8),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .areas(frame.area());

    let [info_area, board_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(INFO_HEIGHT), Constraint::Min(3)])
        .areas(body_area);

    let animation = render_state.animation_frame(now);

    panels::draw_header(frame, header_area, no_color);
    panels::draw_status(frame, info_area, game, no_color);
    board::draw_board(frame, board_area, game, &animation, no_color);
    panels::draw_state_overlay(frame, board_area, game.run_state(), no_color, centered_area);
    panels::draw_footer(frame, footer_area, game.run_state(), no_color);
}

/// 判断当前终端尺寸是否已经小到无法稳定显示主界面。
pub fn is_terminal_too_small(width: u16, height: u16) -> bool {
    width < min_terminal_width() || height < min_terminal_height()
}

/// 根据终端尺寸估算可用棋盘大小，并保留最小可玩尺寸。
pub fn board_size_for_terminal(width: u16, height: u16) -> (u16, u16) {
    let board_width = width.saturating_sub(2).max(MIN_BOARD_WIDTH);
    let board_height = height
        .saturating_sub(HEADER_HEIGHT + FOOTER_HEIGHT + INFO_HEIGHT + 2)
        .max(MIN_BOARD_HEIGHT);

    (board_width, board_height)
}

fn min_terminal_width() -> u16 {
    MIN_BOARD_WIDTH + 2
}

fn min_terminal_height() -> u16 {
    HEADER_HEIGHT + FOOTER_HEIGHT + INFO_HEIGHT + MIN_BOARD_HEIGHT + 2
}

pub(crate) fn centered_area(area: Rect, width: u16, height: u16) -> Rect {
    let popup_width = width.min(area.width.saturating_sub(2)).max(1);
    let popup_height = height.min(area.height.saturating_sub(2)).max(1);

    let vertical: [Rect; 3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(popup_height),
            Constraint::Fill(1),
        ])
        .areas(area);

    let horizontal: [Rect; 3] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(popup_width),
            Constraint::Fill(1),
        ])
        .areas(vertical[1]);

    horizontal[1]
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RenderState;
    use crate::game::GameState;

    #[test]
    /// 验证进入 GameOver 后会触发死亡闪烁动画，并在重新开始后清除。
    fn death_flash_starts_and_clears_with_state_changes() {
        let mut render_state = RenderState::new();
        let now = std::time::Instant::now();

        let mut game = GameState::with_board_size(4, 4);
        render_state.sync(&game, now);
        assert!(!render_state.animation_frame(now).death_flash_visible);

        game.start();
        game.tick();
        game.tick();
        render_state.sync(&game, now);
        assert!(render_state.animation_frame(now).death_flash_visible);

        game.restart();
        render_state.sync(&game, now + Duration::from_millis(10));
        assert!(
            !render_state
                .animation_frame(now + Duration::from_millis(10))
                .death_flash_visible
        );
    }
}
