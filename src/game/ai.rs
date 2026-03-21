use std::collections::VecDeque;

use rand::Rng;

use crate::config::game::AI_NON_WALL_AVOIDANCE_CHANCE_PERCENT;

use super::{
    Direction, EnemyPlan, EnemySnake, GameState, NavigationDecision, Position, SnakeAppearance,
};

impl GameState {
    /// 为一条 AI 计算下一步移动意图。
    ///
    /// 根据当前游戏状态，为指定 AI 选择最佳移动方向，
    /// 并计算该移动带来的效果（吃到食物、撞到炸弹等）。
    ///
    /// # 参数
    /// - `enemy_index`: AI 蛇在 `enemies` 数组中的索引
    ///
    /// # 返回值
    /// 返回包含下一步位置、移动效果和导航决策的完整计划
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
    ///
    /// AI 决策采用分层优先级策略，按以下顺序尝试：
    ///
    /// 1. **继续随机漫步**：如果当前正在随机漫步且下一步安全，继续沿当前方向走
    /// 2. **触发随机漫步**：15% 概率进入随机漫步模式，持续 5-14 步
    /// 3. **追逐食物**：计算最近的食物位置，选择能接近食物的安全方向
    /// 4. **保持方向**：如果当前方向安全，继续前进
    /// 5. **紧急逃生**：从剩余安全方向中任选一个
    /// 6. **无路可走**：保持当前方向（将导致死亡）
    ///
    /// # 参数
    /// - `enemy_index`: AI 蛇在 `enemies` 数组中的索引
    ///
    /// # 返回值
    /// 返回包含方向、随机漫步步数和方向的导航决策
    fn choose_enemy_direction(&self, enemy_index: usize) -> NavigationDecision {
        let enemy = &self.enemies[enemy_index];

        // 如果正在随机漫步，尝试继续沿当前方向走
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
            // 当前方向不安全，重新选择一个安全的随机方向
            let walk_dir = self.random_walk_direction(enemy_index, enemy.direction());
            return NavigationDecision {
                direction: walk_dir,
                random_walk_steps: enemy.random_walk_steps.saturating_sub(1),
                random_walk_direction: Some(walk_dir),
            };
        }

        // 15% 概率触发随机漫步模式
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

        // 追逐最近的食物
        let target = self.closest_consumable_to(enemy.head());
        let preferred = self.preferred_directions(enemy.head(), target);

        for direction in preferred {
            // 跳过反向（不能 180 度掉头）
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

        // 保持当前方向（如果安全）
        let next = self.next_position(enemy.head(), enemy.direction());
        if self.enemy_step_is_safe(enemy_index, next) {
            return NavigationDecision {
                direction: enemy.direction(),
                random_walk_steps: 0,
                random_walk_direction: None,
            };
        }

        // 紧急逃生，从剩余安全方向中任选一个
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

        // 无路可走，保持当前方向（将导致死亡）
        NavigationDecision {
            direction: enemy.direction(),
            random_walk_steps: 0,
            random_walk_direction: None,
        }
    }

    /// 为随机漫步选择一个安全的方向。
    ///
    /// 随机尝试所有方向，从中选择一个安全的方向。
    /// 如果没有安全方向，则尝试保持当前方向；
    /// 如果当前方向也不安全，则默认返回向上。
    ///
    /// # 参数
    /// - `enemy_index`: AI 蛇的索引
    /// - `current_direction`: 当前移动方向
    ///
    /// # 返回值
    /// 返回一个安全的移动方向
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
    ///
    /// 安全性检查包括：
    /// - 是否撞墙
    /// - 是否撞到自身（考虑尾巴移动规则）
    /// - 是否撞到炸弹、玩家或其他 AI
    ///
    /// 对于非墙类危险，AI 有一定概率不会主动规避，
    /// 这增加了游戏的不确定性和可玩性。
    ///
    /// # 参数
    /// - `enemy_index`: AI 蛇的索引
    /// - `next`: 待检查的下一步位置
    ///
    /// # 返回值
    /// 如果位置安全则返回 `true`
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
    ///
    /// 根据配置的概率决定 AI 是否会主动避开炸弹、玩家或其他 AI。
    /// 这个机制让 AI 偶尔会"失误"，增加游戏的趣味性。
    fn enemy_avoids_non_wall_hazard(&self) -> bool {
        let mut rng = rand::rng();
        rng.random_range(0..100) < AI_NON_WALL_AVOIDANCE_CHANCE_PERCENT
    }

    /// 让 AI 重生到预设角落位置，避免出生点过于随机。
    ///
    /// 重生时会保留 AI 之前的分数，只重置位置和状态。
    /// 如果无法在角落找到有效位置，会尝试随机位置。
    ///
    /// # 参数
    /// - `enemy_index`: 需要重生的 AI 索引
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
    ///
    /// 优先在棋盘四角生成 AI，如果角落位置不可用，
    /// 则回退到随机位置生成。
    ///
    /// # 参数
    /// - `slot`: AI 蛇的槽位编号，决定出生角落
    ///
    /// # 返回值
    /// 成功时返回生成的 AI 蛇，失败时返回 `None`
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
    ///
    /// AI 蛇优先在棋盘四角出生，每条蛇对应一个角落（slot % 4）。
    /// 对于每个角落，会生成两种候选形态：
    /// - 垂直放置（如果高度 >= 3）
    /// - 水平放置（如果宽度 >= 3）
    ///
    /// 角落分配规则：
    /// - slot 0, 4, 8... → 左上角 (0, 0)
    /// - slot 1, 5, 9... → 右上角 (width-1, 0)
    /// - slot 2, 6, 10... → 左下角 (0, height-1)
    /// - slot 3, 7, 11... → 右下角 (width-1, height-1)
    ///
    /// # 参数
    /// - `slot`: AI 蛇的槽位编号
    ///
    /// # 返回值
    /// 返回候选形态列表，每个元素包含（蛇身坐标，初始方向）
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
    ///
    /// 使用拒绝采样方法，随机生成候选位置，
    /// 直到找到一个不与现有物体重叠的位置。
    ///
    /// # 参数
    /// - `slot`: AI 蛇的槽位编号，用于确定外观
    ///
    /// # 返回值
    /// 成功时返回生成的 AI 蛇，失败时返回 `None`
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
    ///
    /// 有效位置需要满足以下条件：
    /// - 不与玩家蛇身重叠
    /// - 不与普通食物重叠
    /// - 不与尸体食物重叠
    /// - 不与超级食物或炸弹重叠
    /// - 不与其他 AI 蛇身重叠
    fn enemy_spawn_is_valid(&self, body: &VecDeque<Position>) -> bool {
        // 不与玩家蛇身重叠
        if self
            .player
            .body()
            .iter()
            .any(|segment| body.contains(segment))
        {
            return false;
        }

        // 不与普通食物重叠
        if self.foods.iter().any(|food| body.contains(food)) {
            return false;
        }

        // 不与尸体食物重叠
        if self.legacy_foods.iter().any(|food| body.contains(food)) {
            return false;
        }

        // 不与超级食物或炸弹重叠
        if self.super_foods.iter().any(|fruit| body.contains(fruit))
            || self.bombs.iter().any(|bomb| body.contains(bomb))
        {
            return false;
        }

        // 不与其他 AI 蛇身重叠
        !self
            .enemies
            .iter()
            .any(|enemy| enemy.body().iter().any(|segment| body.contains(segment)))
    }

    /// 返回离指定坐标最近的一颗可食用物品。
    ///
    /// 在普通食物、尸体食物和超级食物中，
    /// 选择曼哈顿距离最近的一个作为目标。
    ///
    /// # 参数
    /// - `origin`: 起始位置（通常是 AI 蛇头）
    ///
    /// # 返回值
    /// 返回最近食物的位置，如果没有食物则返回起始位置
    fn closest_consumable_to(&self, origin: Position) -> Position {
        self.foods
            .iter()
            .chain(self.legacy_foods.iter())
            .chain(self.super_foods.iter())
            .copied()
            .min_by_key(|food| Self::manhattan_distance(origin, *food))
            .unwrap_or(origin)
    }

    /// 按"更接近目标优先，其余方向补齐"的顺序返回方向列表。
    ///
    /// 首先添加能减少与目标距离的方向，
    /// 然后按固定顺序补充剩余方向。
    /// 这确保 AI 优先选择朝向食物的方向。
    ///
    /// # 参数
    /// - `origin`: 起始位置
    /// - `target`: 目标位置
    ///
    /// # 返回值
    /// 返回按优先级排序的方向列表
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
