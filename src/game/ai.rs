use std::collections::VecDeque;

use rand::Rng;

use crate::config::game::AI_NON_WALL_AVOIDANCE_CHANCE_PERCENT;

use super::{
    Direction, EnemyPlan, EnemySnake, GameState, NavigationDecision, Position, SnakeAppearance,
};

impl GameState {
    /// 为一条 AI 计算下一步移动意图。
    pub(super) fn plan_enemy_move(&self, enemy_index: usize) -> EnemyPlan {
        let navigation = self.choose_enemy_direction(enemy_index);
        let enemy = &self.enemies[enemy_index];
        let next_head = self.next_position(enemy.head(), navigation.direction);
        let effect = self.tile_effect(next_head);

        EnemyPlan {
            next_head,
            consumable: effect.consumable,
            growth_amount: effect.growth_amount,
            score_gain: effect.score_gain,
            hits_bomb: effect.hits_bomb,
            navigation,
            crashes: false,
        }
    }

    /// 为 AI 敌蛇选择下一步的移动方向。
    fn choose_enemy_direction(&self, enemy_index: usize) -> NavigationDecision {
        let enemy = &self.enemies[enemy_index];

        if enemy.random_walk_steps > 0 {
            if let Some(walk_dir) = enemy.random_walk_direction {
                let next = self.next_position(enemy.head(), walk_dir);
                if self.enemy_step_is_safe(enemy_index, next) {
                    return NavigationDecision {
                        direction: walk_dir,
                        random_walk_steps: enemy.random_walk_steps.saturating_sub(1),
                        random_walk_direction: Some(walk_dir),
                    };
                }
            }

            let walk_dir = self.random_walk_direction(enemy_index, enemy.direction());
            return NavigationDecision {
                direction: walk_dir,
                random_walk_steps: enemy.random_walk_steps.saturating_sub(1),
                random_walk_direction: Some(walk_dir),
            };
        }

        let mut rng = rand::rng();
        if rng.random_range(0..100) < 15 {
            let walk_dir = self.random_walk_direction(enemy_index, enemy.direction());
            let steps = rng.random_range(5..15);
            return NavigationDecision {
                direction: walk_dir,
                random_walk_steps: steps,
                random_walk_direction: Some(walk_dir),
            };
        }

        let target = self.closest_consumable_to(enemy.head());
        let preferred = self.preferred_directions(enemy.head(), target);

        for direction in preferred {
            if Self::is_opposite(enemy.direction(), direction) {
                continue;
            }

            let next = self.next_position(enemy.head(), direction);
            if self.enemy_step_is_safe(enemy_index, next) {
                return NavigationDecision {
                    direction,
                    random_walk_steps: 0,
                    random_walk_direction: None,
                };
            }
        }

        let next = self.next_position(enemy.head(), enemy.direction());
        if self.enemy_step_is_safe(enemy_index, next) {
            return NavigationDecision {
                direction: enemy.direction(),
                random_walk_steps: 0,
                random_walk_direction: None,
            };
        }

        let safe_dirs = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
        .into_iter()
        .filter(|&direction| {
            !Self::is_opposite(enemy.direction(), direction)
                && self.enemy_step_is_safe(enemy_index, self.next_position(enemy.head(), direction))
        });

        if let Some(direction) = safe_dirs.into_iter().next() {
            return NavigationDecision {
                direction,
                random_walk_steps: 0,
                random_walk_direction: None,
            };
        }

        NavigationDecision {
            direction: enemy.direction(),
            random_walk_steps: 0,
            random_walk_direction: None,
        }
    }

    /// 为随机漫步选择一个安全的方向。
    fn random_walk_direction(&self, enemy_index: usize, current_direction: Direction) -> Direction {
        let all = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ];
        let mut rng = rand::rng();
        let mut safe_directions = Vec::new();

        for _ in 0..all.len() {
            let direction = all[rng.random_range(0..all.len())];
            if Self::is_opposite(current_direction, direction) {
                continue;
            }

            let next = self.next_position(self.enemies[enemy_index].head(), direction);
            if self.enemy_step_is_safe(enemy_index, next) {
                safe_directions.push(direction);
            }
        }

        if let Some(&direction) = safe_directions.first() {
            return direction;
        }

        let next = self.next_position(self.enemies[enemy_index].head(), current_direction);
        if self.enemy_step_is_safe(enemy_index, next) {
            return current_direction;
        }

        Direction::Up
    }

    /// 判断 AI 的下一步位置是否安全（不会立即撞死）。
    fn enemy_step_is_safe(&self, enemy_index: usize, next: Position) -> bool {
        if self.hit_wall(next) {
            return false;
        }

        if self.occupies_with_tail_rules(self.enemies[enemy_index].body(), next, false) {
            return false;
        }

        let hits_non_wall_hazard = self.bombs.contains(&next)
            || self.player_occupies_position(next, 0)
            || self.enemies.iter().enumerate().any(|(other_index, _)| {
                other_index != enemy_index && self.enemy_occupies_position(other_index, next, &[])
            });

        !hits_non_wall_hazard || !self.enemy_avoids_non_wall_hazard()
    }

    /// AI 是否会主动规避一次非撞墙风险。
    fn enemy_avoids_non_wall_hazard(&self) -> bool {
        let mut rng = rand::rng();
        rng.random_range(0..100) < AI_NON_WALL_AVOIDANCE_CHANCE_PERCENT
    }

    /// 让 AI 重生到预设角落位置，避免出生点过于随机。
    pub(super) fn respawn_enemy(&mut self, enemy_index: usize) {
        let score = self.enemies[enemy_index].snake.score;

        if let Some(replacement) = self.try_spawn_enemy_for_slot(enemy_index) {
            self.enemies[enemy_index] = replacement;
            self.enemies[enemy_index].snake.score = score;
        } else {
            self.enemies[enemy_index].snake.score = score;
        }
    }

    /// 尝试在指定 slot 对应的角落生成一条 AI 蛇。
    pub(super) fn try_spawn_enemy_for_slot(&self, slot: usize) -> Option<EnemySnake> {
        if self.width < 3 && self.height < 3 {
            return None;
        }

        for (body, direction) in self.corner_spawn_candidates(slot) {
            let enemy = EnemySnake::new(body, direction, SnakeAppearance::for_slot(slot));
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
    fn try_spawn_enemy(&self, slot: usize) -> Option<EnemySnake> {
        if self.width < 3 && self.height < 3 {
            return None;
        }

        for _ in 0..256 {
            let (body, direction) = Self::spawn_enemy_snake(self.width, self.height);
            if self.enemy_spawn_is_valid(&body) {
                return Some(EnemySnake::new(
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

        !self
            .enemies
            .iter()
            .any(|enemy| enemy.body().iter().any(|segment| body.contains(segment)))
    }

    /// 返回离指定坐标最近的一颗可食用物品。
    fn closest_consumable_to(&self, origin: Position) -> Position {
        self.foods
            .iter()
            .chain(self.legacy_foods.iter())
            .chain(self.super_foods.iter())
            .copied()
            .min_by_key(|food| Self::manhattan_distance(origin, *food))
            .unwrap_or(origin)
    }

    /// 按“更接近目标优先，其余方向补齐”的顺序返回方向列表。
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

        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            if !directions.contains(&direction) {
                directions.push(direction);
            }
        }

        directions
    }
}
