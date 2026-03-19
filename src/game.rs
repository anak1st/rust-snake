use std::collections::VecDeque;

use rand::Rng;

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

/// 默认同时生成的食物数量。
const FOOD_COUNT: usize = 4;

/// 封装一局贪吃蛇的完整状态。
pub struct GameState {
    /// 棋盘宽度，单位为网格数。
    width: u16,
    /// 棋盘高度，单位为网格数。
    height: u16,
    /// 已推进的逻辑帧数。
    tick_count: u64,
    /// 玩家累计得分。
    score: u32,
    /// 敌方累计得分。
    enemy_score: u32,
    /// 游戏当前运行状态。
    state: RunState,
    /// 玩家当前已经生效的移动方向。
    direction: Direction,
    /// 玩家最新输入、将在下一帧生效的方向。
    pending_direction: Direction,
    /// 敌蛇当前已经生效的移动方向。
    enemy_direction: Direction,
    /// 敌蛇随机漫步剩余步数，为0时表示追逐食物模式。
    enemy_random_walk_steps: u8,
    /// 敌蛇随机漫步的方向。
    enemy_random_walk_direction: Option<Direction>,
    /// 玩家蛇身坐标队列，尾部在前，头部在后。
    snake: VecDeque<Position>,
    /// 敌方蛇身坐标队列，尾部在前，头部在后。
    enemy_snake: VecDeque<Position>,
    /// 当前棋盘上的所有食物位置。
    foods: Vec<Position>,
}

impl GameState {
    /// 创建一个默认尺寸的游戏状态。
    pub fn new() -> Self {
        Self::with_board_size(16, 12)
    }

    /// 按指定棋盘尺寸初始化一局新游戏。
    pub fn with_board_size(width: u16, height: u16) -> Self {
        let snake = Self::spawn_player_snake(width, height);
        let enemy_snake = Self::spawn_enemy_snake(width, height);

        let mut game = Self {
            width,
            height,
            tick_count: 0,
            score: 0,
            enemy_score: 0,
            state: RunState::Ready,
            direction: Direction::Right,
            pending_direction: Direction::Right,
            enemy_direction: Direction::Left,
            enemy_random_walk_steps: 0,
            enemy_random_walk_direction: None,
            snake,
            enemy_snake,
            foods: Vec::new(),
        };
        game.refill_foods();
        game
    }

    /// 进入运行状态，开始或继续推进游戏。
    pub fn start(&mut self) {
        if matches!(self.state, RunState::Ready | RunState::Paused) {
            self.state = RunState::Running;
        }
    }

    /// 推进一帧游戏逻辑，处理玩家、敌蛇、食物和碰撞。
    pub fn tick(&mut self) {
        if self.state != RunState::Running {
            return;
        }

        self.direction = self.pending_direction;

        if self.enemy_random_walk_steps > 0 {
            self.enemy_random_walk_steps -= 1;
        }
        self.enemy_direction = self.choose_enemy_direction();

        let player_next = self.next_position(self.player_head(), self.direction);
        let enemy_next = self.next_position(self.enemy_head(), self.enemy_direction);
        let player_eats = self.foods.contains(&player_next);
        let enemy_eats = self.foods.contains(&enemy_next);

        if self.player_collides(player_next, player_eats, enemy_next, enemy_eats) {
            self.state = RunState::GameOver;
            return;
        }

        let enemy_crashes = self.enemy_collides(enemy_next, enemy_eats, player_next, player_eats);

        self.advance_player(player_next, player_eats);
        if enemy_crashes {
            self.respawn_enemy();
        } else {
            self.advance_enemy(enemy_next, enemy_eats);
        }

        self.refill_foods();
        self.tick_count += 1;
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
        *self = Self::with_board_size(self.width, self.height);
    }

    /// 使用新的棋盘尺寸重新开始一局。
    pub fn restart_with_board_size(&mut self, width: u16, height: u16) {
        *self = Self::with_board_size(width, height);
    }

    /// 更新玩家下一次移动方向，并忽略直接反向输入。
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

    /// 返回玩家当前分数。
    pub fn score(&self) -> u32 {
        self.score
    }

    /// 返回敌方当前分数。
    pub fn enemy_score(&self) -> u32 {
        self.enemy_score
    }

    /// 返回当前运行状态。
    pub fn run_state(&self) -> RunState {
        self.state
    }

    /// 返回玩家当前生效的移动方向。
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// 返回敌蛇当前生效的移动方向。
    pub fn enemy_direction(&self) -> Direction {
        self.enemy_direction
    }

    /// 返回玩家蛇身坐标队列，尾部在前，头部在后。
    pub fn snake(&self) -> &VecDeque<Position> {
        &self.snake
    }

    /// 返回敌方蛇身坐标队列，尾部在前，头部在后。
    pub fn enemy_snake(&self) -> &VecDeque<Position> {
        &self.enemy_snake
    }

    /// 返回当前所有食物位置。
    pub fn foods(&self) -> &[Position] {
        &self.foods
    }

    /// 返回玩家蛇头位置。
    fn player_head(&self) -> Position {
        self.snake
            .back()
            .copied()
            .unwrap_or(Position { x: 0, y: 0 })
    }

    /// 返回敌蛇蛇头位置。
    fn enemy_head(&self) -> Position {
        self.enemy_snake
            .back()
            .copied()
            .unwrap_or(Position { x: 0, y: 0 })
    }

    /// 让玩家蛇前进一步，并处理吃食物后的增长。
    fn advance_player(&mut self, next_head: Position, eats_food: bool) {
        self.snake.push_back(next_head);
        if eats_food {
            self.score += 1;
            self.remove_food(next_head);
        } else {
            self.snake.pop_front();
        }
    }

    /// 让敌蛇前进一步，并处理吃食物后的增长。
    fn advance_enemy(&mut self, next_head: Position, eats_food: bool) {
        self.enemy_snake.push_back(next_head);
        if eats_food {
            self.enemy_score += 1;
            self.remove_food(next_head);
        } else {
            self.enemy_snake.pop_front();
        }
    }

    /// 判断玩家下一步是否会导致游戏结束。
    fn player_collides(
        &self,
        next_head: Position,
        player_eats: bool,
        enemy_next: Position,
        enemy_eats: bool,
    ) -> bool {
        self.hit_wall(next_head)
            || self.occupies_with_tail_rules(&self.snake, next_head, player_eats)
            || self.occupies_with_tail_rules(&self.enemy_snake, next_head, enemy_eats)
            || next_head == enemy_next
    }

    /// 判断敌蛇下一步是否会撞死；敌蛇撞死时会在下一帧被重生。
    fn enemy_collides(
        &self,
        next_head: Position,
        enemy_eats: bool,
        player_next: Position,
        player_eats: bool,
    ) -> bool {
        self.hit_wall(next_head)
            || self.occupies_with_tail_rules(&self.enemy_snake, next_head, enemy_eats)
            || self.occupies_with_tail_rules(&self.snake, next_head, player_eats)
            || next_head == player_next
    }

    /// 让敌蛇重生到棋盘另一侧，避免 AI 卡死后整局无法继续。
    fn respawn_enemy(&mut self) {
        self.enemy_snake = Self::spawn_enemy_snake(self.width, self.height);
        self.enemy_direction = Direction::Left;
        self.enemy_random_walk_steps = 0;
        self.enemy_random_walk_direction = None;

        while self.snake_overlaps(&self.enemy_snake)
            || self
                .foods
                .iter()
                .any(|food| self.enemy_snake.contains(food))
        {
            self.enemy_snake = Self::spawn_enemy_snake(self.width, self.height);
        }
    }

    fn choose_enemy_direction(&mut self) -> Direction {
        if self.enemy_random_walk_steps > 0 {
            if let Some(walk_dir) = self.enemy_random_walk_direction {
                let next = self.next_position(self.enemy_head(), walk_dir);
                if !self.hit_wall(next)
                    && !self.occupies_with_tail_rules(&self.enemy_snake, next, false)
                    && !self.occupies_with_tail_rules(&self.snake, next, false)
                {
                    return walk_dir;
                }
            }
            let walk_dir = self.random_walk_direction();
            self.enemy_random_walk_direction = Some(walk_dir);
            return walk_dir;
        }

        let mut rng = rand::rng();
        if rng.random_range(0..100) < 15 {
            let walk_dir = self.random_walk_direction();
            self.enemy_random_walk_steps = rng.random_range(5..15);
            self.enemy_random_walk_direction = Some(walk_dir);
            return walk_dir;
        }

        let target = self.closest_food_to(self.enemy_head());
        let preferred = self.preferred_directions(self.enemy_head(), target);

        for direction in preferred {
            if Self::is_opposite(self.enemy_direction, direction) {
                continue;
            }

            let next = self.next_position(self.enemy_head(), direction);
            if self.hit_wall(next) {
                continue;
            }

            if self.occupies_with_tail_rules(&self.enemy_snake, next, false)
                || self.occupies_with_tail_rules(&self.snake, next, false)
            {
                continue;
            }

            self.enemy_random_walk_direction = None;
            return direction;
        }

        self.enemy_direction
    }

    fn random_walk_direction(&self) -> Direction {
        let all = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ];
        let mut rng = rand::rng();

        for _ in 0..3 {
            let idx = rng.random_range(0..all.len());
            let direction = all[idx];
            if Self::is_opposite(self.enemy_direction, direction) {
                continue;
            }
            let next = self.next_position(self.enemy_head(), direction);
            if !self.hit_wall(next)
                && !self.occupies_with_tail_rules(&self.enemy_snake, next, false)
                && !self.occupies_with_tail_rules(&self.snake, next, false)
            {
                return direction;
            }
        }

        self.enemy_direction
    }

    /// 返回离指定坐标最近的一颗食物。
    fn closest_food_to(&self, origin: Position) -> Position {
        self.foods
            .iter()
            .copied()
            .min_by_key(|food| Self::manhattan_distance(origin, *food))
            .unwrap_or(origin)
    }

    /// 按“先横向后纵向”的优先级给出更接近目标的方向列表。
    fn preferred_directions(&self, origin: Position, target: Position) -> Vec<Direction> {
        let mut directions = Vec::with_capacity(4);

        if target.x > origin.x {
            directions.push(Direction::Right);
        } else if target.x < origin.x {
            directions.push(Direction::Left);
        }

        if target.y > origin.y {
            directions.push(Direction::Down);
        } else if target.y < origin.y {
            directions.push(Direction::Up);
        }

        directions
    }

    /// 根据当前位置和方向计算下一步位置。
    fn next_position(&self, head: Position, direction: Direction) -> Position {
        match direction {
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

    /// 按尾巴是否会移动的规则判断某条蛇是否占用了指定位置。
    fn occupies_with_tail_rules(
        &self,
        snake: &VecDeque<Position>,
        position: Position,
        grows: bool,
    ) -> bool {
        snake.iter().enumerate().any(|(index, segment)| {
            let is_tail = index == 0;
            *segment == position && (grows || !is_tail)
        })
    }

    /// 按配置数量补齐食物。
    fn refill_foods(&mut self) {
        while self.foods.len() < FOOD_COUNT {
            let food = self.random_empty_position();
            self.foods.push(food);
        }
    }

    /// 从棋盘上移除一颗被吃掉的食物。
    fn remove_food(&mut self, position: Position) {
        if let Some(index) = self.foods.iter().position(|food| *food == position) {
            self.foods.swap_remove(index);
        }
    }

    /// 随机生成一个不与任意蛇身或食物重叠的位置。
    fn random_empty_position(&self) -> Position {
        let mut rng = rand::rng();

        loop {
            let candidate = Position {
                x: rng.random_range(0..self.width),
                y: rng.random_range(0..self.height),
            };

            if !self.snake.contains(&candidate)
                && !self.enemy_snake.contains(&candidate)
                && !self.foods.contains(&candidate)
            {
                return candidate;
            }
        }
    }

    /// 生成玩家初始蛇身，默认放在棋盘中部偏左。
    fn spawn_player_snake(width: u16, height: u16) -> VecDeque<Position> {
        let mut snake = VecDeque::new();
        let center_y = height / 2;
        let center_x = width / 3;

        snake.push_back(Position {
            x: center_x.saturating_sub(1),
            y: center_y,
        });
        snake.push_back(Position {
            x: center_x,
            y: center_y,
        });
        snake.push_back(Position {
            x: center_x + 1,
            y: center_y,
        });
        snake
    }

    /// 生成敌蛇初始蛇身，默认放在棋盘中部偏右。
    fn spawn_enemy_snake(width: u16, height: u16) -> VecDeque<Position> {
        let mut snake = VecDeque::new();
        let center_y = height / 2;
        let center_x = (width * 2) / 3;

        snake.push_back(Position {
            x: center_x + 1,
            y: center_y,
        });
        snake.push_back(Position {
            x: center_x,
            y: center_y,
        });
        snake.push_back(Position {
            x: center_x.saturating_sub(1),
            y: center_y,
        });
        snake
    }

    /// 判断两条蛇当前是否有重叠。
    fn snake_overlaps(&self, other: &VecDeque<Position>) -> bool {
        self.snake.iter().any(|segment| other.contains(segment))
    }

    /// 计算两个坐标之间的曼哈顿距离。
    fn manhattan_distance(a: Position, b: Position) -> u16 {
        a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
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
    /// 验证每次 tick 都会让玩家蛇头向前推进一格。
    fn snake_moves_forward_on_tick() {
        let mut game = GameState::with_board_size(18, 8);
        game.start();
        let old_head = game.snake().back().copied().unwrap();

        game.tick();

        let new_head = game.snake().back().copied().unwrap();
        assert_eq!(new_head.x, old_head.x + 1);
        assert_eq!(new_head.y, old_head.y);
    }

    #[test]
    /// 验证直接反向输入会被忽略，避免玩家蛇原地掉头。
    fn opposite_direction_is_ignored() {
        let mut game = GameState::with_board_size(18, 8);
        game.start();
        game.set_direction(Direction::Left);

        game.tick();

        assert_eq!(game.direction(), Direction::Right);
    }

    #[test]
    /// 验证玩家蛇撞到边界后会进入游戏结束状态。
    fn wall_collision_ends_game() {
        let mut game = GameState::with_board_size(4, 4);
        game.start();

        for _ in 0..2 {
            game.tick();
        }

        assert_eq!(game.run_state(), RunState::GameOver);
    }

    #[test]
    /// 验证新游戏默认停留在开始界面，等待玩家启动。
    fn new_game_starts_in_ready_state() {
        let game = GameState::with_board_size(10, 8);

        assert_eq!(game.run_state(), RunState::Ready);
    }

    #[test]
    /// 验证新游戏会一次生成多颗食物。
    fn game_spawns_multiple_foods() {
        let game = GameState::with_board_size(12, 8);

        assert_eq!(game.foods().len(), 4);
    }

    #[test]
    /// 验证敌蛇初始位置不会与玩家蛇重叠。
    fn enemy_snake_starts_separate_from_player() {
        let game = GameState::with_board_size(12, 8);

        assert!(game
            .enemy_snake()
            .iter()
            .all(|segment| !game.snake().contains(segment)));
    }
}
