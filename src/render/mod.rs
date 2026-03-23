//! 汇总渲染入口、整体布局与终端尺寸辅助。

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::config::render::{
    FOOTER_HEIGHT, HEADER_HEIGHT, INFO_HEIGHT, MIN_BOARD_HEIGHT, MIN_BOARD_WIDTH,
};
use crate::game::GameState;

mod animation;
mod board;
mod panels;
mod style;

pub use animation::RenderState;
pub(crate) use animation::{ActiveCellFlash, AnimationFrame, CellFlashKind};

/// 描述死亡回放当前正在查看的帧位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayStatus {
    pub current_frame: usize,
    pub total_frames: usize,
}

/// 根据当前游戏状态绘制整个界面，并叠加动画帧。
pub fn draw(
    frame: &mut Frame,
    game: &GameState,
    render_state: &RenderState,
    replay_status: Option<ReplayStatus>,
    now: Instant,
    window_too_small: bool,
    no_color: bool,
) {
    if window_too_small {
        panels::draw_too_small(frame, no_color);
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

    panels::draw_header(
        frame,
        header_area,
        game.run_state(),
        replay_status,
        no_color,
    );
    panels::draw_status(frame, info_area, game, replay_status, no_color);
    board::draw_board(frame, board_area, game, &animation, no_color);
    panels::draw_state_overlay_with_replay(
        frame,
        board_area,
        game.run_state(),
        replay_status,
        no_color,
    );
    panels::draw_footer(
        frame,
        footer_area,
        game.run_state(),
        replay_status,
        no_color,
    );
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

/// 返回终端最小宽度要求。
fn min_terminal_width() -> u16 {
    MIN_BOARD_WIDTH + 2
}

/// 返回终端最小高度要求。
fn min_terminal_height() -> u16 {
    HEADER_HEIGHT + FOOTER_HEIGHT + INFO_HEIGHT + MIN_BOARD_HEIGHT + 2
}
