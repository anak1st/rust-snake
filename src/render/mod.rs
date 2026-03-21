//! 渲染模块，负责将游戏状态组织为终端界面，并叠加短时动画效果。

use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::config::render::{
    FOOTER_HEIGHT, HEADER_HEIGHT, INFO_HEIGHT, MIN_BOARD_HEIGHT, MIN_BOARD_WIDTH,
};
use crate::game::{GameEvent, GameState, Position, RunState};

mod board;
mod panels;
mod style;

const FOOD_FLASH_DURATION_MS: u64 = 220;
const FOOD_FLASH_INTERVAL_MS: u64 = 55;
const SUPER_FOOD_PULSE_INTERVAL_MS: u64 = 180;
const FOOD_PULSE_HIGHLIGHT_EVERY_STEPS: u128 = 4;

/// 渲染层持有的短时动画状态。
pub struct RenderState {
    pulse_anchor: Instant,
    last_processed_event_tick: Option<u64>,
    cell_flashes: Vec<CellFlash>,
}

struct CellFlash {
    position: Position,
    kind: CellFlashKind,
    started_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellFlashKind {
    Food,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActiveCellFlash {
    pub position: Position,
    pub kind: CellFlashKind,
    pub is_visible: bool,
}

pub(crate) struct AnimationFrame {
    pub food_pulse_on: bool,
    pub super_food_pulse_on: bool,
    pub active_flashes: Vec<ActiveCellFlash>,
}

impl RenderState {
    /// 创建渲染状态容器。
    pub fn new() -> Self {
        Self {
            pulse_anchor: Instant::now(),
            last_processed_event_tick: None,
            cell_flashes: Vec::new(),
        }
    }

    /// 按当前游戏状态推进渲染层动画。
    pub fn sync(&mut self, game: &GameState, now: Instant) {
        let run_state = game.run_state();

        if self.last_processed_event_tick != Some(game.tick_count()) {
            self.record_game_events(game.recent_events(), now);
            self.last_processed_event_tick = Some(game.tick_count());
        }

        self.cell_flashes.retain(|flash| !flash.is_finished(now));

        if matches!(run_state, RunState::Ready) {
            self.cell_flashes.clear();
        }
    }

    fn animation_frame(&self, now: Instant) -> AnimationFrame {
        AnimationFrame {
            food_pulse_on: sparse_pulse_phase(
                self.pulse_anchor,
                now,
                SUPER_FOOD_PULSE_INTERVAL_MS,
                FOOD_PULSE_HIGHLIGHT_EVERY_STEPS,
            ),
            super_food_pulse_on: pulse_phase(self.pulse_anchor, now, SUPER_FOOD_PULSE_INTERVAL_MS),
            active_flashes: self
                .cell_flashes
                .iter()
                .map(|flash| ActiveCellFlash {
                    position: flash.position,
                    kind: flash.kind,
                    is_visible: flash.is_visible(now),
                })
                .collect(),
        }
    }

    fn record_game_events(&mut self, events: &[GameEvent], now: Instant) {
        for event in events {
            let GameEvent::CorpseFoodCreated(position) = *event;
            self.cell_flashes.push(CellFlash {
                position,
                kind: CellFlashKind::Food,
                started_at: now,
            });
        }
    }
}

impl CellFlash {
    fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started_at)
    }

    fn is_finished(&self, now: Instant) -> bool {
        self.elapsed(now) >= Duration::from_millis(FOOD_FLASH_DURATION_MS)
    }

    fn is_visible(&self, now: Instant) -> bool {
        let interval = Duration::from_millis(FOOD_FLASH_INTERVAL_MS).as_millis();
        let step = self.elapsed(now).as_millis() / interval.max(1);
        step % 2 == 0
    }
}

fn pulse_phase(anchor: Instant, now: Instant, interval_ms: u64) -> bool {
    let interval = Duration::from_millis(interval_ms).as_millis();
    let step = now.saturating_duration_since(anchor).as_millis() / interval.max(1);
    step % 2 == 0
}

fn sparse_pulse_phase(anchor: Instant, now: Instant, interval_ms: u64, every_steps: u128) -> bool {
    let interval = Duration::from_millis(interval_ms).as_millis();
    let step = now.saturating_duration_since(anchor).as_millis() / interval.max(1);
    step % every_steps == every_steps.saturating_sub(1)
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
    use crate::game::{GameEvent, GameState, Position};

    #[test]
    /// 验证尸块转成食物时会触发短时闪光，并在重开后清除。
    fn corpse_food_event_creates_flash_and_clears_on_restart() {
        let mut render_state = RenderState::new();
        let now = std::time::Instant::now();
        let flash_position = Position { x: 4, y: 4 };

        let mut game = GameState::with_board_size(4, 4);
        render_state.sync(&game, now);
        assert!(render_state.animation_frame(now).active_flashes.is_empty());

        render_state.record_game_events(&[GameEvent::CorpseFoodCreated(flash_position)], now);
        assert!(
            render_state
                .animation_frame(now)
                .active_flashes
                .iter()
                .any(|flash| flash.position == flash_position
                    && flash.kind == super::CellFlashKind::Food)
        );

        render_state.sync(&game, now);

        game.restart();
        render_state.sync(&game, now + Duration::from_millis(10));
        assert!(
            render_state
                .animation_frame(now + Duration::from_millis(10))
                .active_flashes
                .is_empty()
        );
    }

    #[test]
    /// 验证场上的超级食物脉冲会按固定节奏切换相位。
    fn super_food_pulse_phase_toggles_over_time() {
        let render_state = RenderState::new();
        let now = std::time::Instant::now();

        let first = render_state.animation_frame(now).super_food_pulse_on;
        let second = render_state
            .animation_frame(now + Duration::from_millis(super::SUPER_FOOD_PULSE_INTERVAL_MS))
            .super_food_pulse_on;

        assert_ne!(first, second);
    }

    #[test]
    /// 验证普通食物采用稀疏闪烁，而不是像超级食物那样每拍交替。
    fn normal_food_pulse_is_sparse() {
        let render_state = RenderState::new();
        let now = std::time::Instant::now();

        let step0 = render_state.animation_frame(now).food_pulse_on;
        let step1 = render_state
            .animation_frame(now + Duration::from_millis(super::SUPER_FOOD_PULSE_INTERVAL_MS))
            .food_pulse_on;
        let step2 = render_state
            .animation_frame(now + Duration::from_millis(super::SUPER_FOOD_PULSE_INTERVAL_MS * 2))
            .food_pulse_on;
        let step3 = render_state
            .animation_frame(now + Duration::from_millis(super::SUPER_FOOD_PULSE_INTERVAL_MS * 3))
            .food_pulse_on;

        assert!(!step0);
        assert!(!step1);
        assert!(!step2);
        assert!(step3);
    }
}
