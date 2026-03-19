use std::collections::VecDeque;

use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Running,
    Paused,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

pub struct GameState {
    width: u16,
    height: u16,
    tick_count: u64,
    score: u32,
    state: RunState,
    direction: Direction,
    pending_direction: Direction,
    snake: VecDeque<Position>,
    food: Position,
}

impl GameState {
    pub fn new() -> Self {
        Self::with_board_size(16, 12)
    }

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

    pub fn toggle_pause(&mut self) {
        self.state = match self.state {
            RunState::Running => RunState::Paused,
            RunState::Paused => RunState::Running,
            RunState::GameOver => RunState::GameOver,
        };
    }

    pub fn restart(&mut self) {
        *self = Self::with_board_size(self.width, self.height);
    }

    pub fn restart_with_board_size(&mut self, width: u16, height: u16) {
        *self = Self::with_board_size(width, height);
    }

    pub fn set_direction(&mut self, direction: Direction) {
        if Self::is_opposite(self.direction, direction) {
            return;
        }

        self.pending_direction = direction;
    }

    pub fn board_size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn run_state(&self) -> RunState {
        self.state
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn snake(&self) -> &VecDeque<Position> {
        &self.snake
    }

    pub fn food(&self) -> Position {
        self.food
    }

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

    fn hit_wall(&self, position: Position) -> bool {
        position.x >= self.width || position.y >= self.height
    }

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
    fn snake_moves_forward_on_tick() {
        let mut game = GameState::with_board_size(10, 8);
        let old_head = game.snake().back().copied().unwrap();

        game.tick();

        let new_head = game.snake().back().copied().unwrap();
        assert_eq!(new_head.x, old_head.x + 1);
        assert_eq!(new_head.y, old_head.y);
    }

    #[test]
    fn opposite_direction_is_ignored() {
        let mut game = GameState::with_board_size(10, 8);
        game.set_direction(Direction::Left);

        game.tick();

        assert_eq!(game.direction(), Direction::Right);
    }

    #[test]
    fn wall_collision_ends_game() {
        let mut game = GameState::with_board_size(4, 4);

        for _ in 0..3 {
            game.tick();
        }

        assert_eq!(game.run_state(), RunState::GameOver);
    }
}
