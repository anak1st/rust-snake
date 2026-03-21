//! 汇总游戏核心状态、共享类型与对外接口。

use crate::config::game::{AI_SNAKE_COUNT, DEFAULT_BOARD_HEIGHT, DEFAULT_BOARD_WIDTH};

mod ai;
mod corpse;
mod logic;
mod snake;
mod spawn;
#[cfg(test)]
mod tests;

pub use corpse::{CorpseCell, CorpsePiece};
#[cfg(test)]
pub(crate) use snake::SnakeControl;
pub use snake::{Snake, SnakeAppearance};

/// 表示游戏当前所处的运行阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    /// 游戏尚未开始，显示开始界面。
    Ready,
    /// 游戏正常进行中。
    Running,
    /// 游戏已暂停，tick 不再推进。
    Paused,
    /// 游戏已结束，等待重开。
    GameOver,
}

/// 表示蛇当前或下一步的移动方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// 表示棋盘上的一个网格坐标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Direction {
    /// 判断两个方向是否互为反方向。
    fn is_opposite(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Up, Self::Down)
                | (Self::Down, Self::Up)
                | (Self::Left, Self::Right)
                | (Self::Right, Self::Left)
        )
    }
}

impl Position {
    /// 计算与另一个坐标之间的曼哈顿距离。
    fn manhattan_distance(self, other: Self) -> u16 {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }
}

#[derive(Debug, Clone, Copy)]
struct NavigationDecision {
    direction: Direction,
    random_walk_steps: u8,
    random_walk_direction: Option<Direction>,
}

#[derive(Debug, Clone, Copy)]
struct SnakePlan {
    next_head: Position,
    consumable: Option<ConsumableKind>,
    growth_amount: u16,
    score_gain: u32,
    hits_bomb: bool,
    navigation: NavigationDecision,
    crashes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConsumableKind {
    Food,
    SuperFood,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TileEffect {
    consumable: Option<ConsumableKind>,
    growth_amount: u16,
    score_gain: u32,
    hits_bomb: bool,
}

impl SnakePlan {
    /// 将规划中记录的格子效果还原为通用结算信息。
    fn tile_effect(self) -> TileEffect {
        TileEffect {
            consumable: self.consumable,
            growth_amount: self.growth_amount,
            score_gain: self.score_gain,
            hits_bomb: self.hits_bomb,
        }
    }
}

/// 游戏在最近一个逻辑 tick 中产生的事件。
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// 一个尸块腐化完成，并在原地转成了尸体食物。
    CorpseFoodCreated(Position),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingEnemyRespawn {
    group_id: u64,
    enemy_index: usize,
}

/// 封装一局贪吃蛇的完整状态。
pub struct GameState {
    /// 棋盘宽度，单位为网格数。
    width: u16,
    /// 棋盘高度，单位为网格数。
    height: u16,
    /// 已推进的逻辑帧数。
    tick_count: u64,
    /// 游戏当前运行状态。
    state: RunState,
    /// 玩家蛇。
    player: Snake,
    /// 所有 AI 敌蛇。
    enemies: Vec<Snake>,
    /// 所有尚未腐化完成的尸块。
    corpse_pieces: Vec<CorpsePiece>,
    /// 下一批尸块使用的分组编号。
    next_corpse_group_id: u64,
    /// 已死亡、等待对应尸块全部腐化后重生的敌蛇。
    pending_enemy_respawns: Vec<PendingEnemyRespawn>,
    /// 当前棋盘上的所有食物位置。
    foods: Vec<Position>,
    /// 蛇死亡后留下的尸体食物位置。
    legacy_foods: Vec<Position>,
    /// 当前棋盘上的所有超级食物位置。
    super_foods: Vec<Position>,
    /// 当前棋盘上的所有炸弹位置。
    bombs: Vec<Position>,
    /// 最近一个逻辑 tick 产生的瞬时事件。
    recent_events: Vec<GameEvent>,
}

impl GameState {
    /// 创建一个默认尺寸（16x12）的游戏状态。
    ///
    /// 初始化后游戏处于 Ready 状态，需要玩家按键才开始。
    /// 默认生成 3 条 AI 蛇和 4 颗食物。
    pub fn new() -> Self {
        Self::with_board_size(DEFAULT_BOARD_WIDTH, DEFAULT_BOARD_HEIGHT)
    }

    /// 按指定棋盘尺寸初始化一局新游戏。
    pub fn with_board_size(width: u16, height: u16) -> Self {
        let player = Snake::new_manual(
            Self::spawn_player_snake(width, height),
            Direction::Right,
            SnakeAppearance::player(),
        );

        let mut game = Self {
            width,
            height,
            tick_count: 0,
            state: RunState::Ready,
            player,
            enemies: Vec::with_capacity(AI_SNAKE_COUNT),
            corpse_pieces: Vec::new(),
            next_corpse_group_id: 0,
            pending_enemy_respawns: Vec::new(),
            foods: Vec::new(),
            legacy_foods: Vec::new(),
            super_foods: Vec::new(),
            bombs: Vec::new(),
            recent_events: Vec::new(),
        };

        for slot in 0..AI_SNAKE_COUNT {
            let Some(enemy) = game.try_spawn_enemy_for_slot(slot) else {
                break;
            };
            game.enemies.push(enemy);
        }

        game.refill_items();
        game
    }

    /// 进入运行状态，开始或继续推进游戏。
    pub fn start(&mut self) {
        if matches!(self.state, RunState::Ready | RunState::Paused) {
            self.state = RunState::Running;
        }
    }

    /// 在运行和暂停之间切换；游戏结束后保持结束状态。
    pub fn toggle_pause(&mut self) {
        self.state = match self.state {
            RunState::Running => RunState::Paused,
            RunState::Paused => RunState::Running,
            RunState::Ready => RunState::Ready,
            RunState::GameOver => RunState::GameOver,
        };
    }

    /// 使用当前棋盘尺寸重新开始一局。
    pub fn restart(&mut self) {
        let player_uses_ai = self.player.is_ai_controlled();
        *self = Self::with_board_size(self.width, self.height);
        self.set_player_ai_control(player_uses_ai);
    }

    /// 使用新的棋盘尺寸重新开始一局。
    pub fn restart_with_board_size(&mut self, width: u16, height: u16) {
        let player_uses_ai = self.player.is_ai_controlled();
        *self = Self::with_board_size(width, height);
        self.set_player_ai_control(player_uses_ai);
    }

    /// 设置玩家蛇是否由 AI 接管控制。
    pub fn set_player_ai_control(&mut self, enabled: bool) {
        self.player.set_ai_controlled(enabled);
    }

    /// 更新玩家下一次移动方向，并忽略直接反向输入。
    ///
    /// 当玩家蛇当前不处于手动控制模式时，此调用不会产生效果。
    pub fn set_direction(&mut self, direction: Direction) {
        if self.player.direction().is_opposite(direction) {
            return;
        }

        self.player.set_manual_direction(direction);
    }

    /// 返回当前棋盘尺寸。
    pub fn board_size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// 返回已推进的 tick 数。
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// 返回玩家当前分数。
    pub fn score(&self) -> u32 {
        self.player.score()
    }

    /// 返回 AI 数量。
    pub fn enemy_count(&self) -> usize {
        self.enemies.len()
    }

    /// 返回当前运行状态。
    pub fn run_state(&self) -> RunState {
        self.state
    }

    /// 返回玩家当前生效的移动方向。
    pub fn direction(&self) -> Direction {
        self.player.direction()
    }

    /// 返回所有 AI 敌蛇。
    pub fn enemies(&self) -> &[Snake] {
        &self.enemies
    }

    /// 返回玩家蛇。
    pub fn player(&self) -> &Snake {
        &self.player
    }

    /// 返回当前仍未腐化完成的所有尸块。
    #[cfg(test)]
    pub(crate) fn corpse_pieces(&self) -> &[CorpsePiece] {
        &self.corpse_pieces
    }

    /// 返回当前所有食物位置。
    pub fn foods(&self) -> &[Position] {
        &self.foods
    }

    /// 返回所有由死亡蛇身转化而来的尸体食物位置。
    pub fn legacy_foods(&self) -> &[Position] {
        &self.legacy_foods
    }

    /// 返回当前所有超级果实位置。
    pub fn super_foods(&self) -> &[Position] {
        &self.super_foods
    }

    /// 返回当前所有炸弹位置。
    pub fn bombs(&self) -> &[Position] {
        &self.bombs
    }

    /// 返回最近一个逻辑 tick 产生的事件列表。
    pub fn recent_events(&self) -> &[GameEvent] {
        &self.recent_events
    }

    /// 返回指定位置上的尸块渲染信息。
    pub fn corpse_cell(&self, position: Position) -> Option<CorpseCell> {
        self.corpse_pieces
            .iter()
            .find(|piece| piece.position() == position)
            .map(|piece| piece.cell())
    }

    /// 返回玩家蛇头位置。
    fn player_head(&self) -> Position {
        self.player.head()
    }
}
