//! 渲染模块，负责将游戏状态组织为终端界面，并叠加短时动画效果。

use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Color;

use crate::config::render::{
    FOOTER_HEIGHT, HEADER_HEIGHT, INFO_HEIGHT, MIN_BOARD_HEIGHT, MIN_BOARD_WIDTH,
};
use crate::game::{GameEvent, GameState, Position, RunState, SnakeDeathEvent};

mod board;
mod panels;
mod style;

const DEATH_SEGMENT_STAGGER_MS: u64 = 70;
const DEATH_SEGMENT_FLASH_DURATION_MS: u64 = 180;
const DEATH_SEGMENT_FLASH_INTERVAL_MS: u64 = 45;
const FOOD_FLASH_DURATION_MS: u64 = 220;
const FOOD_FLASH_INTERVAL_MS: u64 = 55;
const SUPER_FOOD_PULSE_INTERVAL_MS: u64 = 180;

/// 渲染层持有的短时动画状态。
pub struct RenderState {
    pulse_anchor: Instant,
    last_processed_event_tick: Option<u64>,
    previous_run_state: RunState,
    previous_foods: Vec<Position>,
    previous_super_foods: Vec<Position>,
    death_animations: Vec<SnakeDeathAnimation>,
    cell_flashes: Vec<CellFlash>,
}

struct SnakeDeathAnimation {
    started_at: Instant,
    segments: Vec<DeathSegment>,
}

struct DeathSegment {
    position: Position,
    glyph: &'static str,
    color: Color,
    bold: bool,
    start_after: Duration,
}

struct CellFlash {
    position: Position,
    kind: CellFlashKind,
    started_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellFlashKind {
    Food,
    SuperFood,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActiveCellFlash {
    pub position: Position,
    pub kind: CellFlashKind,
    pub is_visible: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ActiveDeathCell {
    pub position: Position,
    pub glyph: &'static str,
    pub color: Color,
    pub bold: bool,
}

pub(crate) struct AnimationFrame {
    pub active_death_cells: Vec<ActiveDeathCell>,
    pub super_food_pulse_on: bool,
    pub active_flashes: Vec<ActiveCellFlash>,
}

impl RenderState {
    /// 创建渲染状态容器。
    pub fn new() -> Self {
        Self {
            pulse_anchor: Instant::now(),
            last_processed_event_tick: None,
            previous_run_state: RunState::Ready,
            previous_foods: Vec::new(),
            previous_super_foods: Vec::new(),
            death_animations: Vec::new(),
            cell_flashes: Vec::new(),
        }
    }

    /// 按当前游戏状态推进渲染层动画。
    pub fn sync(&mut self, game: &GameState, now: Instant) {
        let run_state = game.run_state();
        let current_foods = game.foods();
        let current_super_foods = game.super_foods();

        if self.previous_run_state == RunState::Running {
            self.record_disappeared_foods(current_foods, current_super_foods, now);
        }

        if self.last_processed_event_tick != Some(game.tick_count()) {
            self.record_game_events(game.recent_events(), now);
            self.last_processed_event_tick = Some(game.tick_count());
        }

        self.death_animations
            .retain(|animation| !animation.is_finished(now));
        self.cell_flashes.retain(|flash| !flash.is_finished(now));

        if matches!(run_state, RunState::Ready) {
            self.death_animations.clear();
            self.cell_flashes.clear();
        }

        self.previous_run_state = run_state;
        self.previous_foods = current_foods.to_vec();
        self.previous_super_foods = current_super_foods.to_vec();
    }

    fn animation_frame(&self, now: Instant) -> AnimationFrame {
        AnimationFrame {
            active_death_cells: self
                .death_animations
                .iter()
                .flat_map(|animation| animation.active_cells(now))
                .collect(),
            super_food_pulse_on: pulse_phase(self.pulse_anchor, now),
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
            match event {
                GameEvent::SnakeDied(event) => {
                    self.death_animations
                        .push(SnakeDeathAnimation::from_event(event, now));
                }
            }
        }
    }

    fn record_disappeared_foods(
        &mut self,
        current_foods: &[Position],
        current_super_foods: &[Position],
        now: Instant,
    ) {
        for &position in &self.previous_foods {
            if !current_foods.contains(&position) {
                self.cell_flashes.push(CellFlash {
                    position,
                    kind: CellFlashKind::Food,
                    started_at: now,
                });
            }
        }

        for &position in &self.previous_super_foods {
            if !current_super_foods.contains(&position) {
                self.cell_flashes.push(CellFlash {
                    position,
                    kind: CellFlashKind::SuperFood,
                    started_at: now,
                });
            }
        }
    }
}

impl SnakeDeathAnimation {
    fn from_event(event: &SnakeDeathEvent, started_at: Instant) -> Self {
        let segments = event
            .segments_head_first()
            .iter()
            .enumerate()
            .map(|(index, &position)| DeathSegment {
                position,
                glyph: if index == 0 {
                    event.head_glyph()
                } else {
                    event.body_glyph()
                },
                color: if index == 0 {
                    event.head_color()
                } else {
                    event.body_color()
                },
                bold: index == 0,
                start_after: Duration::from_millis(index as u64 * DEATH_SEGMENT_STAGGER_MS),
            })
            .collect();

        Self {
            started_at,
            segments,
        }
    }

    fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started_at)
    }

    fn is_finished(&self, now: Instant) -> bool {
        let Some(last_segment) = self.segments.last() else {
            return true;
        };

        self.elapsed(now)
            >= last_segment.start_after + Duration::from_millis(DEATH_SEGMENT_FLASH_DURATION_MS)
    }

    fn active_cells(&self, now: Instant) -> Vec<ActiveDeathCell> {
        let elapsed = self.elapsed(now);
        let mut cells = Vec::with_capacity(self.segments.len());

        for segment in &self.segments {
            if elapsed < segment.start_after {
                cells.push(segment.original_cell());
                continue;
            }

            let phase_elapsed = elapsed.saturating_sub(segment.start_after);
            let flash_duration = Duration::from_millis(DEATH_SEGMENT_FLASH_DURATION_MS);
            if phase_elapsed >= flash_duration {
                continue;
            }

            let interval = Duration::from_millis(DEATH_SEGMENT_FLASH_INTERVAL_MS).as_millis();
            let step = phase_elapsed.as_millis() / interval.max(1);
            if step.is_multiple_of(2) {
                cells.push(segment.original_cell());
            } else {
                cells.push(segment.food_cell());
            }
        }

        cells
    }
}

impl DeathSegment {
    fn original_cell(&self) -> ActiveDeathCell {
        ActiveDeathCell {
            position: self.position,
            glyph: self.glyph,
            color: self.color,
            bold: self.bold,
        }
    }

    fn food_cell(&self) -> ActiveDeathCell {
        ActiveDeathCell {
            position: self.position,
            glyph: "*",
            color: Color::LightGreen,
            bold: true,
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

fn pulse_phase(anchor: Instant, now: Instant) -> bool {
    let interval = Duration::from_millis(SUPER_FOOD_PULSE_INTERVAL_MS).as_millis();
    let step = now.saturating_duration_since(anchor).as_millis() / interval.max(1);
    step % 2 == 0
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
    use crate::game::{GameState, Position, RunState};

    #[test]
    /// 验证进入 GameOver 后会触发局部死亡动画，并在重新开始后清除。
    fn death_animation_starts_and_clears_with_state_changes() {
        let mut render_state = RenderState::new();
        let now = std::time::Instant::now();

        let mut game = GameState::with_board_size(4, 4);
        render_state.sync(&game, now);
        assert!(
            render_state
                .animation_frame(now)
                .active_death_cells
                .is_empty()
        );

        game.start();
        game.tick();
        game.tick();
        render_state.sync(&game, now);
        assert!(
            !render_state
                .animation_frame(now)
                .active_death_cells
                .is_empty()
        );

        game.restart();
        render_state.sync(&game, now + Duration::from_millis(10));
        assert!(
            render_state
                .animation_frame(now + Duration::from_millis(10))
                .active_death_cells
                .is_empty()
        );
    }

    #[test]
    /// 验证运行中消失的普通食物会生成短时闪光。
    fn consumed_food_creates_flash() {
        let now = std::time::Instant::now();
        let mut render_state = RenderState::new();
        let mut game = GameState::with_board_size(10, 8);
        let flash_position = Position { x: 99, y: 99 };

        game.start();
        render_state.previous_run_state = RunState::Running;
        render_state.previous_foods = vec![flash_position];
        render_state.sync(&game, now);

        let frame = render_state.animation_frame(now);
        assert!(frame.active_flashes.iter().any(|flash| {
            flash.position == flash_position && flash.kind == super::CellFlashKind::Food
        }));
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
}
