//! 封装 AI 蛇的路径选择与安全性评估。

use std::collections::VecDeque;

use rand::Rng;
use rand::seq::SliceRandom;

use crate::config::game::{
    AI_NON_WALL_AVOIDANCE_CHANCE_PERCENT, AI_RANDOM_WALK_CHANCE_PERCENT, AI_RANDOM_WALK_MAX_STEPS,
    AI_RANDOM_WALK_MIN_STEPS,
};

use super::{Direction, GameState, NavigationDecision, Position, Snake, SnakePlan};

impl Snake {
    /// 为当前 AI 计算下一步移动意图。
    ///
    /// AI 的状态与决策都归属于蛇自身，`GameState` 只提供棋盘规则、
    /// 占用判断与物品分布等环境信息。
    pub(super) fn plan_ai_move(&self, game: &GameState) -> SnakePlan {
        let navigation = self.choose_direction(game);
        let next_head = game.next_position(self.head(), navigation.direction);
        let effect = game.tile_effect(next_head);

        SnakePlan {
            next_head,
            consumable: effect.consumable,
            growth_amount: effect.growth_amount,
            score_gain: effect.score_gain,
            hits_bomb: effect.hits_bomb,
            navigation,
            crashes: false,
        }
    }

    /// 应用本次规划得到的导航状态。
    pub(super) fn apply_navigation(&mut self, navigation: NavigationDecision) {
        self.direction = navigation.direction;
        let ai_state = self.ai_state_mut();
        ai_state.random_walk_steps = navigation.random_walk_steps;
        ai_state.random_walk_direction = navigation.random_walk_direction;
    }

    /// 为 AI 敌蛇选择下一步的移动方向。
    ///
    /// AI 决策采用分层优先级策略，按以下顺序尝试：
    ///
    /// 1. **继续随机漫步**：如果当前正在随机漫步且下一步安全，继续沿当前方向走
    /// 2. **触发随机漫步**：按配置概率进入随机漫步模式，持续配置指定的步数范围
    /// 3. **追逐食物**：计算最近的食物位置，选择能接近食物的安全方向
    /// 4. **保持方向**：如果当前方向安全，继续前进
    /// 5. **紧急逃生**：从剩余安全方向中任选一个
    /// 6. **无路可走**：保持当前方向（将导致死亡）
    ///
    /// # 返回值
    /// 返回包含方向、随机漫步步数和方向的导航决策
    fn choose_direction(&self, game: &GameState) -> NavigationDecision {
        // 如果正在随机漫步，尝试继续沿当前方向走
        let ai_state = self.ai_state();
        if ai_state.random_walk_steps > 0 {
            if let Some(walk_dir) = ai_state.random_walk_direction {
                let next = game.next_position(self.head(), walk_dir);
                if game.snake_step_is_safe(self, next)
                    && game.snake_step_has_adequate_space(self, next)
                {
                    return NavigationDecision {
                        direction: walk_dir,
                        random_walk_steps: ai_state.random_walk_steps.saturating_sub(1),
                        random_walk_direction: Some(walk_dir),
                    };
                }
            }

            // 当前方向不安全，重新选择一个安全的随机方向
            let walk_dir = self.random_walk_direction(game);
            return NavigationDecision {
                direction: walk_dir,
                random_walk_steps: ai_state.random_walk_steps.saturating_sub(1),
                random_walk_direction: Some(walk_dir),
            };
        }

        // 按配置概率触发随机漫步模式
        let mut rng = rand::rng();
        if rng.random_range(0..100) < AI_RANDOM_WALK_CHANCE_PERCENT {
            let walk_dir = self.random_walk_direction(game);
            let steps = rng.random_range(AI_RANDOM_WALK_MIN_STEPS..=AI_RANDOM_WALK_MAX_STEPS);
            return NavigationDecision {
                direction: walk_dir,
                random_walk_steps: steps,
                random_walk_direction: Some(walk_dir),
            };
        }

        // 追逐最近的食物
        let target = game.closest_consumable_to(self.head());
        let preferred = game.preferred_directions(self.head(), target);

        if let Some(direction) = self.pick_direction_with_space_preference(game, preferred) {
            return Self::steady_navigation(direction);
        }

        // 如果当前方向既安全又留有足够活动空间，优先保持方向
        let next = game.next_position(self.head(), self.direction());
        if game.snake_step_is_safe(self, next) && game.snake_step_has_adequate_space(self, next) {
            return Self::steady_navigation(self.direction());
        }

        // 紧急逃生，从剩余安全方向中任选一个
        if let Some(direction) = self.pick_direction_with_space_preference(
            game,
            [
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ],
        ) {
            return Self::steady_navigation(direction);
        }

        // 尝试任何即时安全的方向
        if game.snake_step_is_safe(self, next) {
            return Self::steady_navigation(self.direction());
        }

        // 最后尝试非掉头的安全方向
        let safe_dirs = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
        .into_iter()
        .filter(|&direction| {
            !self.direction().is_opposite(direction)
                && game.snake_step_is_safe(self, game.next_position(self.head(), direction))
        });

        if let Some(direction) = safe_dirs.into_iter().next() {
            return Self::steady_navigation(direction);
        }

        // 无路可走，保持当前方向（将导致死亡）
        Self::steady_navigation(self.direction())
    }

    /// 为随机漫步选择一个安全的方向。
    ///
    /// 随机打乱所有方向后逐个尝试，从中选择一个安全的方向。
    /// 如果没有安全方向，则尝试保持当前方向；
    /// 如果当前方向也不安全，则默认返回向上。
    fn random_walk_direction(&self, game: &GameState) -> Direction {
        let mut directions = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ];
        let mut rng = rand::rng();
        directions.shuffle(&mut rng);

        if let Some(direction) = self.pick_direction_with_space_preference(game, directions) {
            return direction;
        }

        let next = game.next_position(self.head(), self.direction());
        if game.snake_step_is_safe(self, next) {
            return self.direction();
        }

        Direction::Up
    }

    /// 返回一个不携带随机漫步状态的普通导航结果。
    fn steady_navigation(direction: Direction) -> NavigationDecision {
        NavigationDecision {
            direction,
            random_walk_steps: 0,
            random_walk_direction: None,
        }
    }

    /// AI 是否会主动规避一次非撞墙风险。
    ///
    /// 根据配置的概率决定 AI 是否会主动避开炸弹、玩家或其他 AI。
    /// 这个机制让 AI 偶尔会"失误"，增加游戏的趣味性。
    fn avoids_non_wall_hazard() -> bool {
        let mut rng = rand::rng();
        rng.random_range(0..100) < AI_NON_WALL_AVOIDANCE_CHANCE_PERCENT
    }

    /// 按给定顺序挑选方向，并优先选择“安全且活动空间足够”的候选。
    ///
    /// 如果所有安全方向都会把蛇压进较小空间，则回退到第一个即时安全方向，
    /// 避免在绝境中因为过度保守而直接放弃可走的路。
    fn pick_direction_with_space_preference(
        &self,
        game: &GameState,
        directions: impl IntoIterator<Item = Direction>,
    ) -> Option<Direction> {
        let mut fallback = None;

        for direction in directions {
            if self.direction().is_opposite(direction) {
                continue;
            }

            let next = game.next_position(self.head(), direction);
            if !game.snake_step_is_safe(self, next) {
                continue;
            }

            if game.snake_step_has_adequate_space(self, next) {
                return Some(direction);
            }

            if fallback.is_none() {
                fallback = Some(direction);
            }
        }

        fallback
    }
}

impl GameState {
    /// 判断一条蛇按当前环境前进一步是否安全（不会立即撞死）。
    ///
    /// 对于非墙类危险，AI 有一定概率不会主动规避，
    /// 这增加了游戏的不确定性和可玩性。
    ///
    /// 处理步骤：
    /// - 检查是否撞墙
    /// - 检查蛇是否存活
    /// - 检查是否撞到自身或尸块（考虑尾巴移动规则）
    /// - 检查是否撞到炸弹或其他蛇（有概率不规避）
    ///
    /// # 参数
    /// - `snake`: 当前执行决策的蛇，自身碰撞和"跳过自己"都基于这个引用判断
    /// - `next`: 待检查的下一步位置
    ///
    /// # 返回值
    /// 如果位置安全则返回 `true`
    pub(super) fn snake_step_is_safe(&self, snake: &Snake, next: Position) -> bool {
        // 检查是否撞墙
        if self.hit_wall(next) {
            return false;
        }

        // 检查蛇是否存活
        if !snake.is_alive() {
            return false;
        }

        // 检查是否撞到自身或尸块
        let effect = self.tile_effect(next);
        let my_projected_length = snake.projected_length(effect.growth_amount);

        if self.occupies_with_tail_rules(snake.body(), next, snake.grows(effect.growth_amount))
            || self.corpse_piece_occupies_position(next)
        {
            return false;
        }

        // 检查是否撞到炸弹或其他蛇（有概率不规避）
        let hits_non_wall_hazard = self.bombs.contains(&next)
            || self.other_snakes_occupy_position(snake, next)
            || self.other_snake_can_win_head_on(snake, next, my_projected_length);

        !hits_non_wall_hazard || !Snake::avoids_non_wall_hazard()
    }

    /// 判断这一步走完后，蛇头所在连通区域是否仍足以容纳自身长度。
    ///
    /// 该检查用于识别“虽然不会立刻撞死，但会把自己扎进狭小封闭空间”的走法。
    /// 实现上会先按本步结果投影出新的蛇身位置，再从新蛇头出发做 flood fill，
    /// 统计仍可接触到的活动格数量；如果这片区域小于投影体长，就视为高风险。
    ///
    /// 这是一个轻量近似，不会尝试精确模拟未来数步，
    /// 因此当空间本身足够大时，AI 仍会允许进入。
    pub(super) fn snake_step_has_adequate_space(&self, snake: &Snake, next: Position) -> bool {
        let effect = self.tile_effect(next);
        let projected_length = snake.projected_length(effect.growth_amount);
        self.reachable_space_after_step(snake, next, effect.growth_amount) >= projected_length
    }

    /// 判断除当前蛇自身外，是否还有其他蛇占据指定位置。
    fn other_snakes_occupy_position(&self, snake: &Snake, position: Position) -> bool {
        (self.player.is_alive()
            && !std::ptr::eq(&self.player, snake)
            && self.occupies_with_tail_rules(
                self.player.body(),
                position,
                self.snake_might_grow_next_tick(&self.player),
            ))
            || self.enemies.iter().any(|other_enemy| {
                other_enemy.is_alive()
                    && !std::ptr::eq(other_enemy, snake)
                    && self.occupies_with_tail_rules(
                        other_enemy.body(),
                        position,
                        self.snake_might_grow_next_tick(other_enemy),
                    )
            })
    }

    /// 判断是否有其他蛇能够在下一步争抢同一个头部位置，并在头撞头中不输。
    fn other_snake_can_win_head_on(
        &self,
        snake: &Snake,
        position: Position,
        my_projected_length: usize,
    ) -> bool {
        let growth_amount = self.tile_effect(position).growth_amount;

        (self.player.is_alive()
            && !std::ptr::eq(&self.player, snake)
            && self.snake_can_reach_position_next_tick(&self.player, position)
            && self.player.projected_length(growth_amount) >= my_projected_length)
            || self
                .enemies
                .iter()
                .filter(|other_enemy| other_enemy.is_alive() && !std::ptr::eq(*other_enemy, snake))
                .any(|other_enemy| {
                    self.snake_can_reach_position_next_tick(other_enemy, position)
                        && other_enemy.projected_length(growth_amount) >= my_projected_length
                })
    }

    /// 判断一条蛇在不掉头的前提下，下一步是否有机会到达指定位置。
    fn snake_can_reach_position_next_tick(&self, snake: &Snake, position: Position) -> bool {
        [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
        .into_iter()
        .any(|direction| {
            !snake.direction().is_opposite(direction)
                && self.next_position(snake.head(), direction) == position
        })
    }

    /// 判断一条蛇在下一步是否存在“会吃到东西从而保留尾巴”的可能。
    fn snake_might_grow_next_tick(&self, snake: &Snake) -> bool {
        snake.pending_growth > 0
            || [
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ]
            .into_iter()
            .filter(|&direction| !snake.direction().is_opposite(direction))
            .any(|direction| {
                let next = self.next_position(snake.head(), direction);
                !self.hit_wall(next) && self.tile_effect(next).growth_amount > 0
            })
    }

    /// 统计一条蛇完成指定落点后，蛇头仍能抵达的活动格数量。
    ///
    /// 处理步骤：
    /// - 初始化阻挡数组，标记所有不可通行的格子
    /// - 标记尸块和炸弹为阻挡
    /// - 标记其他蛇的蛇身为阻挡
    /// - 计算投影蛇身并标记为阻挡
    /// - 使用 flood fill 统计可达空间
    fn reachable_space_after_step(
        &self,
        snake: &Snake,
        next: Position,
        growth_amount: u16,
    ) -> usize {
        // 初始化阻挡数组
        let mut blocked = vec![false; usize::from(self.width) * usize::from(self.height)];

        // 标记尸块为阻挡
        for corpse in &self.corpse_pieces {
            blocked[self.board_index(corpse.position())] = true;
        }

        // 标记炸弹为阻挡
        for bomb in &self.bombs {
            blocked[self.board_index(*bomb)] = true;
        }

        // 标记玩家蛇身为阻挡
        if self.player.is_alive() && !std::ptr::eq(&self.player, snake) {
            self.mark_body_as_blocked(
                &mut blocked,
                self.player.body(),
                self.snake_might_grow_next_tick(&self.player),
            );
        }

        // 标记敌蛇蛇身为阻挡
        for enemy in &self.enemies {
            if enemy.is_alive() && !std::ptr::eq(enemy, snake) {
                self.mark_body_as_blocked(
                    &mut blocked,
                    enemy.body(),
                    self.snake_might_grow_next_tick(enemy),
                );
            }
        }

        // 计算投影蛇身并标记为阻挡（排除新蛇头）
        let projected_body = self.projected_body_after_step(snake, next, growth_amount);
        for segment in projected_body
            .iter()
            .take(projected_body.len().saturating_sub(1))
        {
            blocked[self.board_index(*segment)] = true;
        }

        // 使用 flood fill 统计可达空间
        let start = self.board_index(next);
        blocked[start] = false;

        let mut visited = vec![false; blocked.len()];
        let mut frontier = VecDeque::from([next]);
        visited[start] = true;
        let mut reachable = 0;

        while let Some(position) = frontier.pop_front() {
            reachable += 1;

            for direction in [
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                let neighbor = self.next_position(position, direction);
                if self.hit_wall(neighbor) {
                    continue;
                }

                let index = self.board_index(neighbor);
                if blocked[index] || visited[index] {
                    continue;
                }

                visited[index] = true;
                frontier.push_back(neighbor);
            }
        }

        reachable
    }

    /// 将一条蛇当前会占据的格子标记为阻挡。
    ///
    /// 当本回合尾巴会移动时，尾格会被视为可穿过，以保持与即时安全判断一致。
    fn mark_body_as_blocked(&self, blocked: &mut [bool], body: &VecDeque<Position>, grows: bool) {
        for (index, segment) in body.iter().enumerate() {
            let is_tail = index == 0;
            if is_tail && !grows {
                continue;
            }

            blocked[self.board_index(*segment)] = true;
        }
    }

    /// 计算一条蛇在本步结算后的投影蛇身。
    fn projected_body_after_step(
        &self,
        snake: &Snake,
        next: Position,
        growth_amount: u16,
    ) -> VecDeque<Position> {
        let mut body = snake.body().clone();
        body.push_back(next);
        if !snake.grows(growth_amount) {
            body.pop_front();
        }
        body
    }

    /// 将棋盘坐标转换为连续数组下标。
    fn board_index(&self, position: Position) -> usize {
        usize::from(position.y) * usize::from(self.width) + usize::from(position.x)
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
            .min_by_key(|food| origin.manhattan_distance(*food))
            .unwrap_or(origin)
    }

    /// 按"更接近目标优先，其余方向补齐"的顺序返回方向列表。
    ///
    /// 处理步骤：
    /// - 根据目标位置添加能减少距离的方向
    /// - 补充剩余方向以覆盖所有可能性
    ///
    /// # 参数
    /// - `origin`: 起始位置
    /// - `target`: 目标位置
    ///
    /// # 返回值
    /// 返回按优先级排序的方向列表
    fn preferred_directions(&self, origin: Position, target: Position) -> Vec<Direction> {
        let mut directions = Vec::with_capacity(4);

        // 添加能减少与目标距离的方向
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

        // 补充剩余方向
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
