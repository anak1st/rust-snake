use std::collections::VecDeque;

use rand::Rng;

use super::{Direction, GameState, Position};

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
