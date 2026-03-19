use std::collections::VecDeque;

use rand::Rng;

/// 表示游戏当前所处的运行阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
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

/// 封装一局贪吃蛇的完整状态。
pub struct GameState {
    /// 棋盘宽度，单位为网格数。
    width: u16,
    /// 棋盘高度，单位为网格数。
    height: u16,
    /// 已推进的逻辑帧数。
    tick_count: u64,
    /// 当前累计得分。
    score: u32,
    /// 游戏当前运行状态。
    state: RunState,
    /// 当前已经生效的移动方向。
    direction: Direction,
    /// 玩家最新输入、将在下一帧生效的方向。
    pending_direction: Direction,
    /// 蛇身坐标队列，尾部在前，头部在后。
    snake: VecDeque<Position>,
    /// 当前食物所在位置。
    food: Position,
}

impl GameState {
    /// 创建一个默认尺寸的游戏状态。
    pub fn new() -> Self {
        Self::with_board_size(16, 12)
    }

    /// 按指定棋盘尺寸初始化一局新游戏。
    pub fn with_board_size(width: u16, height: u16) -> Self {
        let mut snake = VecDeque::new();
        snake.push_back(Position {
            x: width / 2 - 1,
            y: height / 2,
        });
        snake.push_back(Position {
            x: width / 2,
            y: height / 2,
        });
        snake.push_back(Position {
            x: width / 2 + 1,
            y: height / 2,
        });

        let mut game = Self {
            width,
            height,
            tick_count: 0,
            score: 0,
            state: RunState::Running,
            direction: Direction::Right,
            pending_direction: Direction::Right,
            snake,
            food: Position { x: 0, y: 0 },
        };
        game.food = game.random_empty_position();
        game
    }

    /// 推进一帧游戏逻辑，处理移动、吃食物和碰撞。
    pub fn tick(&mut self) {
        if self.state != RunState::Running {
            return;
        }

        self.direction = self.pending_direction;

        let next_head = self.next_head_position();
        if self.hit_wall(next_head) || self.snake.contains(&next_head) {
            self.state = RunState::GameOver;
            return;
        }

        self.snake.push_back(next_head);
        if next_head == self.food {
            self.score += 1;
            self.food = self.random_empty_position();
        } else {
            self.snake.pop_front();
        }

        self.tick_count += 1;
    }

    /// 在运行和暂停之间切换；游戏结束后保持结束状态。
    pub fn toggle_pause(&mut self) {
        self.state = match self.state {
            RunState::Running => RunState::Paused,
            RunState::Paused => RunState::Running,
            RunState::GameOver => RunState::GameOver,
        };
    }

    /// 使用当前棋盘尺寸重新开始一局。
    pub fn restart(&mut self) {
        *self = Self::with_board_size(self.width, self.height);
    }

    /// 使用新的棋盘尺寸重新开始一局。
    pub fn restart_with_board_size(&mut self, width: u16, height: u16) {
        *self = Self::with_board_size(width, height);
    }

    /// 更新下一次移动方向，并忽略直接反向输入。
    pub fn set_direction(&mut self, direction: Direction) {
        if Self::is_opposite(self.direction, direction) {
            return;
        }

        self.pending_direction = direction;
    }

    /// 返回当前棋盘尺寸。
    pub fn board_size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// 返回已推进的 tick 数。
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// 返回当前分数。
    pub fn score(&self) -> u32 {
        self.score
    }

    /// 返回当前运行状态。
    pub fn run_state(&self) -> RunState {
        self.state
    }

    /// 返回当前生效的移动方向。
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// 返回蛇身坐标队列，尾部在前，头部在后。
    pub fn snake(&self) -> &VecDeque<Position> {
        &self.snake
    }

    /// 返回当前食物位置。
    pub fn food(&self) -> Position {
        self.food
    }

    /// 根据当前方向计算蛇头下一步的位置。
    fn next_head_position(&self) -> Position {
        let head = self.snake.back().copied().unwrap_or(Position { x: 0, y: 0 });

        match self.direction {
            Direction::Up => Position {
                x: head.x,
                y: head.y.saturating_sub(1),
            },
            Direction::Down => Position {
                x: head.x,
                y: head.y + 1,
            },
            Direction::Left => Position {
                x: head.x.saturating_sub(1),
                y: head.y,
            },
            Direction::Right => Position {
                x: head.x + 1,
                y: head.y,
            },
        }
    }

    /// 判断一个位置是否越出棋盘边界。
    fn hit_wall(&self, position: Position) -> bool {
        position.x >= self.width || position.y >= self.height
    }

    /// 随机生成一个不与蛇身重叠的食物位置。
    fn random_empty_position(&self) -> Position {
        let mut rng = rand::rng();

        loop {
            let candidate = Position {
                x: rng.random_range(0..self.width),
                y: rng.random_range(0..self.height),
            };

            if !self.snake.contains(&candidate) {
                return candidate;
            }
        }
    }

    /// 判断两个方向是否互为反方向。
    fn is_opposite(current: Direction, next: Direction) -> bool {
        matches!(
            (current, next),
            (Direction::Up, Direction::Down)
                | (Direction::Down, Direction::Up)
                | (Direction::Left, Direction::Right)
                | (Direction::Right, Direction::Left)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, GameState, RunState};

    #[test]
    /// 验证每次 tick 都会让蛇头向前推进一格。
    fn snake_moves_forward_on_tick() {
        let mut game = GameState::with_board_size(10, 8);
        let old_head = game.snake().back().copied().unwrap();

        game.tick();

        let new_head = game.snake().back().copied().unwrap();
        assert_eq!(new_head.x, old_head.x + 1);
        assert_eq!(new_head.y, old_head.y);
    }

    #[test]
    /// 验证直接反向输入会被忽略，避免蛇原地掉头。
    fn opposite_direction_is_ignored() {
        let mut game = GameState::with_board_size(10, 8);
        game.set_direction(Direction::Left);

        game.tick();

        assert_eq!(game.direction(), Direction::Right);
    }

    #[test]
    /// 验证蛇撞到边界后会进入游戏结束状态。
    fn wall_collision_ends_game() {
        let mut game = GameState::with_board_size(4, 4);

        for _ in 0..3 {
            game.tick();
        }

        assert_eq!(game.run_state(), RunState::GameOver);
    }
}
