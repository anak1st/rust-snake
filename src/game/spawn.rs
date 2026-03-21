//! 负责玩家与敌蛇的出生、重生与出生合法性校验。

use std::collections::VecDeque;

use rand::Rng;

use super::{Direction, GameState, Position, Snake, SnakeAppearance};

impl GameState {
    /// 生成玩家初始蛇身，默认放在棋盘中部偏左。
    pub(super) fn spawn_player_snake(width: u16, height: u16) -> VecDeque<Position> {
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

    /// 随机生成一条长度为 3 的敌蛇及其初始方向。
    pub(super) fn spawn_enemy_snake(width: u16, height: u16) -> (VecDeque<Position>, Direction) {
        let mut rng = rand::rng();
        let horizontal = width >= 3 && (height < 3 || rng.random_bool(0.6));

        if horizontal {
            let head_x = rng.random_range(0..=width.saturating_sub(3));
            let y = rng.random_range(0..height);
            (Self::horizontal_enemy_body(head_x, y), Direction::Left)
        } else {
            let x = rng.random_range(0..width);
            let head_y = rng.random_range(0..=height.saturating_sub(3));
            let mut snake = VecDeque::new();
            snake.push_back(Position { x, y: head_y + 2 });
            snake.push_back(Position { x, y: head_y + 1 });
            snake.push_back(Position { x, y: head_y });
            (snake, Direction::Up)
        }
    }

    /// 让 AI 重生到预设角落位置，避免出生点过于随机。
    ///
    /// 重生时会重置位置、状态和分数。
    /// 如果无法在角落找到有效位置，会尝试随机位置。
    pub(super) fn respawn_enemy(&mut self, enemy_index: usize) -> bool {
        if let Some(replacement) = self.try_spawn_enemy_for_slot(enemy_index) {
            self.enemies[enemy_index] = replacement;
            return true;
        }

        false
    }

    /// 尝试在指定 slot 对应的角落生成一条 AI 蛇。
    ///
    /// 优先在棋盘四角生成 AI，如果角落位置不可用，
    /// 则回退到随机位置生成。
    pub(super) fn try_spawn_enemy_for_slot(&self, slot: usize) -> Option<Snake> {
        if self.width < 3 && self.height < 3 {
            return None;
        }

        for (body, direction) in self.corner_spawn_candidates(slot) {
            let enemy = Snake::new_ai(body, direction, SnakeAppearance::for_slot(slot));
            if self.enemy_spawn_is_valid(enemy.body()) {
                return Some(enemy);
            }
        }

        self.try_spawn_enemy(slot)
    }

    /// 返回指定 slot 在角落处的候选出生形态。
    fn corner_spawn_candidates(&self, slot: usize) -> Vec<(VecDeque<Position>, Direction)> {
        let corner_index = slot % 4;
        let mut candidates = Vec::with_capacity(2);

        if self.height >= 3 {
            candidates.push(match corner_index {
                0 => (Self::vertical_enemy_body(0, 0), Direction::Down),
                1 => (
                    Self::vertical_enemy_body(self.width.saturating_sub(1), 0),
                    Direction::Down,
                ),
                2 => (
                    Self::vertical_enemy_body_from_bottom(0, self.height.saturating_sub(1)),
                    Direction::Up,
                ),
                _ => (
                    Self::vertical_enemy_body_from_bottom(
                        self.width.saturating_sub(1),
                        self.height.saturating_sub(1),
                    ),
                    Direction::Up,
                ),
            });
        }

        if self.width >= 3 {
            candidates.push(match corner_index {
                0 => (Self::horizontal_enemy_body(0, 0), Direction::Right),
                1 => (
                    Self::horizontal_enemy_body_from_right(self.width.saturating_sub(1), 0),
                    Direction::Left,
                ),
                2 => (
                    Self::horizontal_enemy_body(0, self.height.saturating_sub(1)),
                    Direction::Right,
                ),
                _ => (
                    Self::horizontal_enemy_body_from_right(
                        self.width.saturating_sub(1),
                        self.height.saturating_sub(1),
                    ),
                    Direction::Left,
                ),
            });
        }

        candidates
    }

    /// 随机尝试生成一条 AI 蛇，最多尝试 256 次。
    fn try_spawn_enemy(&self, slot: usize) -> Option<Snake> {
        if self.width < 3 && self.height < 3 {
            return None;
        }

        for _ in 0..256 {
            let (body, direction) = Self::spawn_enemy_snake(self.width, self.height);
            if self.enemy_spawn_is_valid(&body) {
                return Some(Snake::new_ai(
                    body,
                    direction,
                    SnakeAppearance::for_slot(slot),
                ));
            }
        }

        None
    }

    /// 检查生成的 AI 蛇身位置是否有效。
    fn enemy_spawn_is_valid(&self, body: &VecDeque<Position>) -> bool {
        if self
            .player
            .body()
            .iter()
            .any(|segment| body.contains(segment))
        {
            return false;
        }

        if self.foods.iter().any(|food| body.contains(food)) {
            return false;
        }

        if self.legacy_foods.iter().any(|food| body.contains(food)) {
            return false;
        }

        if self.super_foods.iter().any(|fruit| body.contains(fruit))
            || self.bombs.iter().any(|bomb| body.contains(bomb))
        {
            return false;
        }

        if self
            .corpse_pieces
            .iter()
            .any(|piece| body.contains(&piece.position()))
        {
            return false;
        }

        !self
            .enemies
            .iter()
            .filter(|enemy| enemy.is_alive())
            .any(|enemy| enemy.body().iter().any(|segment| body.contains(segment)))
    }

    /// 生成一条水平放置的敌蛇身体。
    pub(super) fn horizontal_enemy_body(head_x: u16, y: u16) -> VecDeque<Position> {
        let mut snake = VecDeque::new();
        snake.push_back(Position { x: head_x + 2, y });
        snake.push_back(Position { x: head_x + 1, y });
        snake.push_back(Position { x: head_x, y });
        snake
    }

    /// 生成一条从右向左延伸的水平蛇身。
    pub(super) fn horizontal_enemy_body_from_right(head_x: u16, y: u16) -> VecDeque<Position> {
        let mut snake = VecDeque::new();
        snake.push_back(Position {
            x: head_x.saturating_sub(2),
            y,
        });
        snake.push_back(Position {
            x: head_x.saturating_sub(1),
            y,
        });
        snake.push_back(Position { x: head_x, y });
        snake
    }

    /// 生成一条从上向下延伸的垂直蛇身。
    pub(super) fn vertical_enemy_body(x: u16, head_y: u16) -> VecDeque<Position> {
        let mut snake = VecDeque::new();
        snake.push_back(Position { x, y: head_y + 2 });
        snake.push_back(Position { x, y: head_y + 1 });
        snake.push_back(Position { x, y: head_y });
        snake
    }

    /// 生成一条从下向上延伸的垂直蛇身。
    pub(super) fn vertical_enemy_body_from_bottom(x: u16, head_y: u16) -> VecDeque<Position> {
        let mut snake = VecDeque::new();
        snake.push_back(Position {
            x,
            y: head_y.saturating_sub(2),
        });
        snake.push_back(Position {
            x,
            y: head_y.saturating_sub(1),
        });
        snake.push_back(Position { x, y: head_y });
        snake
    }
}
