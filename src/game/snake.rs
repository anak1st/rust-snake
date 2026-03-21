use std::collections::VecDeque;

use ratatui::style::Color;

use super::{Direction, Position};

/// 蛇的固定外观信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeAppearance {
    /// 蛇头显示符号。
    pub(super) head_glyph: &'static str,
    /// 蛇身显示符号。
    pub(super) body_glyph: &'static str,
    /// 蛇头颜色。
    pub(super) head_color: Color,
    /// 蛇身颜色。
    pub(super) body_color: Color,
}

/// AI 控制模式下需要维护的内部状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiState {
    pub(super) random_walk_steps: u8,
    pub(super) random_walk_direction: Option<Direction>,
}

/// 描述一条蛇当前由何种方式驱动。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnakeControl {
    /// 使用手动输入控制，方向会在下一帧同步生效。
    Manual { pending_direction: Direction },
    /// 使用 AI 控制，由 AI 状态驱动下一步决策。
    Ai(AiState),
}

/// 统一的蛇实体，玩家蛇与敌蛇都使用同一种结构。
#[derive(Debug, Clone)]
pub struct Snake {
    /// 当前已生效的移动方向。
    pub(super) direction: Direction,
    /// 蛇身，尾部在前、头部在后。
    pub(super) body: VecDeque<Position>,
    /// 当前累计得分。
    pub(super) score: u32,
    /// 未来还应继续增长的节数。
    pub(super) pending_growth: u16,
    /// 该蛇的固定外观配置。
    pub(super) appearance: SnakeAppearance,
    /// 当前使用的控制方式。
    pub(super) control: SnakeControl,
}

impl Snake {
    /// 创建一条新的蛇。
    ///
    /// 初始化蛇身、方向、外观和控制方式，分数和待增长值默认为零。
    fn new(
        body: VecDeque<Position>,
        direction: Direction,
        appearance: SnakeAppearance,
        control: SnakeControl,
    ) -> Self {
        Self {
            direction,
            body,
            score: 0,
            pending_growth: 0,
            appearance,
            control,
        }
    }

    /// 创建一条手动控制的蛇。
    pub(super) fn new_manual(
        body: VecDeque<Position>,
        direction: Direction,
        appearance: SnakeAppearance,
    ) -> Self {
        Self::new(
            body,
            direction,
            appearance,
            SnakeControl::Manual {
                pending_direction: direction,
            },
        )
    }

    /// 创建一条 AI 控制的蛇。
    pub(super) fn new_ai(
        body: VecDeque<Position>,
        direction: Direction,
        appearance: SnakeAppearance,
    ) -> Self {
        Self::new(
            body,
            direction,
            appearance,
            SnakeControl::Ai(AiState::new()),
        )
    }

    /// 返回当前移动方向。
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// 返回当前是否仍在棋盘上存活。
    pub fn is_alive(&self) -> bool {
        !self.body.is_empty()
    }

    /// 返回蛇身坐标。
    pub fn body(&self) -> &VecDeque<Position> {
        &self.body
    }

    /// 返回累计得分。
    pub fn score(&self) -> u32 {
        self.score
    }

    /// 返回蛇头符号。
    pub fn head_glyph(&self) -> &'static str {
        self.appearance.head_glyph
    }

    /// 返回蛇身符号。
    pub fn body_glyph(&self) -> &'static str {
        self.appearance.body_glyph
    }

    /// 返回蛇头颜色。
    pub fn head_color(&self) -> Color {
        self.appearance.head_color
    }

    /// 返回蛇身颜色。
    pub fn body_color(&self) -> Color {
        self.appearance.body_color
    }

    /// 返回蛇头位置。如果身体为空，返回 (0, 0)。
    pub fn head(&self) -> Position {
        self.body.back().copied().unwrap_or(Position { x: 0, y: 0 })
    }

    /// 将最新手动输入记录到待生效方向中。
    pub(super) fn set_manual_direction(&mut self, direction: Direction) {
        if let SnakeControl::Manual { pending_direction } = &mut self.control {
            *pending_direction = direction;
        }
    }

    /// 将控制状态中的待生效方向同步到当前方向。
    pub(super) fn sync_control_direction(&mut self) {
        if let SnakeControl::Manual { pending_direction } = &self.control {
            self.direction = *pending_direction;
        }
    }

    /// 返回当前是否由 AI 控制。
    pub(super) fn is_ai_controlled(&self) -> bool {
        matches!(self.control, SnakeControl::Ai(_))
    }

    /// 切换当前蛇的控制模式。
    pub(super) fn set_ai_controlled(&mut self, enabled: bool) {
        self.control = if enabled {
            SnakeControl::Ai(AiState::new())
        } else {
            SnakeControl::Manual {
                pending_direction: self.direction,
            }
        };
    }

    /// 返回当前 AI 状态；如果不是 AI 控制则视为逻辑错误。
    pub(super) fn ai_state(&self) -> &AiState {
        match &self.control {
            SnakeControl::Ai(state) => state,
            SnakeControl::Manual { .. } => {
                panic!("AI logic was called for a manually controlled snake")
            }
        }
    }

    /// 返回当前 AI 状态的可变引用；如果不是 AI 控制则视为逻辑错误。
    pub(super) fn ai_state_mut(&mut self) -> &mut AiState {
        match &mut self.control {
            SnakeControl::Ai(state) => state,
            SnakeControl::Manual { .. } => {
                panic!("AI logic was called for a manually controlled snake")
            }
        }
    }

    /// 计算这条蛇在本次前进中是否会增长。
    pub(super) fn grows(&self, growth_amount: u16) -> bool {
        self.pending_growth > 0 || growth_amount > 0
    }

    /// 计算这条蛇在本次移动结算后会表现出的体型长度。
    pub(super) fn projected_length(&self, growth_amount: u16) -> usize {
        self.body.len() + usize::from(self.grows(growth_amount))
    }

    /// 将蛇从棋盘上移除，但保留控制方式和方向。
    pub(super) fn remove_from_board(&mut self) {
        self.body.clear();
        self.pending_growth = 0;
    }

    /// 将当前得分清零。
    pub(super) fn reset_score(&mut self) {
        self.score = 0;
    }

    /// 推进蛇身，并根据成长值决定是否保留尾巴。
    ///
    /// 蛇身增长的实现采用"延迟增长"机制：
    /// - 吃到食物时，`pending_growth` 记录待增长的节数
    /// - 每次移动时，如果 `pending_growth > 0`，则不移除尾巴，并递减计数
    /// - 这样蛇会逐渐变长，而不是一次性增长
    pub(super) fn advance(&mut self, next_head: Position, growth_amount: u16, score_gain: u32) {
        self.body.push_back(next_head);
        self.score += score_gain;

        let total_growth = self.pending_growth.saturating_add(growth_amount);
        if total_growth > 0 {
            self.pending_growth = total_growth.saturating_sub(1);
        } else {
            self.body.pop_front();
        }
    }
}

impl AiState {
    /// 创建一份初始 AI 状态。
    ///
    /// 默认不处于随机漫步模式，步数为零且无指定方向。
    fn new() -> Self {
        Self {
            random_walk_steps: 0,
            random_walk_direction: None,
        }
    }
}

impl SnakeAppearance {
    /// 返回玩家蛇的外观配置。
    pub(super) fn player() -> Self {
        Self {
            head_glyph: "@",
            body_glyph: "o",
            head_color: Color::White,
            body_color: Color::White,
        }
    }

    /// 按固定槽位返回 AI 的外观配置。
    pub(super) fn for_slot(slot: usize) -> Self {
        match slot % 6 {
            0 => Self {
                head_glyph: "A",
                body_glyph: "a",
                head_color: Color::LightMagenta,
                body_color: Color::Magenta,
            },
            1 => Self {
                head_glyph: "B",
                body_glyph: "b",
                head_color: Color::LightCyan,
                body_color: Color::Cyan,
            },
            2 => Self {
                head_glyph: "C",
                body_glyph: "c",
                head_color: Color::LightYellow,
                body_color: Color::Yellow,
            },
            3 => Self {
                head_glyph: "D",
                body_glyph: "d",
                head_color: Color::LightRed,
                body_color: Color::Red,
            },
            4 => Self {
                head_glyph: "E",
                body_glyph: "e",
                head_color: Color::LightBlue,
                body_color: Color::Blue,
            },
            _ => Self {
                head_glyph: "F",
                body_glyph: "f",
                head_color: Color::White,
                body_color: Color::Gray,
            },
        }
    }
}
