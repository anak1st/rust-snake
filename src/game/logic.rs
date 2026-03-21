use std::collections::VecDeque;

use rand::Rng;

use crate::config::game::{
    BOMB_COUNT, FOOD_COUNT, FOOD_GROWTH_AMOUNT, FOOD_SCORE_GAIN, SUPER_FOOD_COUNT,
    SUPER_FOOD_GROWTH_AMOUNT, SUPER_FOOD_SCORE_GAIN,
};

use super::{
    ConsumableKind, Direction, EnemyPlan, GameState, Position, RunState, Snake, TileEffect,
};

impl GameState {
    /// 推进一帧游戏逻辑，处理玩家、AI、食物和碰撞。
    ///
    /// tick 是核心逻辑推进函数，处理流程分为以下几个阶段：
    ///
    /// 1. **方向同步**：将 pending_direction 同步为实际生效的 direction
    /// 2. **玩家移动计算**：计算玩家下一步位置和格子效果
    /// 3. **AI 移动规划**：为每条 AI 预规划下一步移动
    /// 4. **碰撞检测**：判断玩家和 AI 是否会撞死
    /// 5. **头撞头结算**：处理玩家与 AI、AI 与 AI 的头碰头情况
    /// 6. **状态更新**：推进存活的蛇，重生死亡的 AI
    /// 7. **收尾工作**：补充物品、递增 tick 计数
    pub fn tick(&mut self) {
        // 检查游戏是否正在运行
        if self.state != RunState::Running {
            return;
        }

        // 方向同步
        self.player.snake.direction = self.player.pending_direction();

        // 玩家移动计算
        let player_next = self.next_position(self.player_head(), self.player.direction());
        let player_effect = self.tile_effect(player_next);

        // AI 移动规划（所有 AI 的规划在碰撞判断之前完成，确保公平性）
        let mut enemy_plans = Vec::with_capacity(self.enemies.len());
        for enemy_index in 0..self.enemies.len() {
            enemy_plans.push(self.plan_enemy_move(enemy_index));
        }

        // 碰撞检测
        let player_grows = self.snake_grows(&self.player.snake, player_effect.growth_amount);
        let mut player_dies = self.player_hits_hazard_or_self(player_next, player_effect)
            || self.player_hits_enemy_body(player_next, &enemy_plans);
        let mut enemy_dies = (0..enemy_plans.len())
            .map(|enemy_index| {
                self.enemy_hits_hazard_or_self(enemy_index, &enemy_plans)
                    || self.enemy_hits_player_body(
                        enemy_index,
                        player_grows,
                        player_next,
                        &enemy_plans,
                    )
                    || self.enemy_hits_enemy_body(enemy_index, &enemy_plans)
            })
            .collect::<Vec<_>>();

        // 阶段 5：头撞头结算
        self.resolve_player_enemy_head_on(
            player_next,
            player_effect,
            &enemy_plans,
            &mut player_dies,
            &mut enemy_dies,
        );
        self.resolve_enemy_head_on(&enemy_plans, &mut enemy_dies);

        // 将死亡标记写入计划
        for (plan, dies) in enemy_plans.iter_mut().zip(enemy_dies.iter().copied()) {
            plan.crashes = dies;
        }

        // 阶段 6：保存碰撞前的蛇身（用于生成尸体食物）
        let player_body_before_crash = self.player.body().clone();
        let enemy_bodies_before_crash = self
            .enemies
            .iter()
            .map(|enemy| enemy.body().clone())
            .collect::<Vec<_>>();

        // 阶段 6：推进存活的玩家
        if !player_dies {
            self.advance_player(player_next, player_effect);
        }

        // 阶段 6：推进或重生 AI
        for (enemy_index, plan) in enemy_plans.into_iter().enumerate() {
            if plan.crashes {
                self.drop_legacy_from_body(&enemy_bodies_before_crash[enemy_index]);
                self.respawn_enemy(enemy_index);
            } else {
                self.advance_enemy(enemy_index, plan);
            }
        }

        // 阶段 6：处理玩家死亡
        if player_dies {
            self.drop_legacy_from_body(&player_body_before_crash);
            self.state = RunState::GameOver;
        }

        // 阶段 7：收尾工作
        self.refill_items();
        self.tick_count += 1;
    }

    /// 让玩家蛇前进一步，并处理吃到物品后的增长。
    fn advance_player(&mut self, next_head: Position, effect: TileEffect) {
        Self::advance_snake(
            &mut self.player.snake,
            next_head,
            effect.growth_amount,
            effect.score_gain,
        );
        self.consume_tile(next_head, effect);
    }

    /// 让指定 AI 前进一步，并处理吃到物品后的增长。
    fn advance_enemy(&mut self, enemy_index: usize, plan: EnemyPlan) {
        let enemy = &mut self.enemies[enemy_index];
        enemy.snake.direction = plan.navigation.direction;
        enemy.random_walk_steps = plan.navigation.random_walk_steps;
        enemy.random_walk_direction = plan.navigation.random_walk_direction;
        Self::advance_snake(
            &mut enemy.snake,
            plan.next_head,
            plan.growth_amount,
            plan.score_gain,
        );
        self.consume_tile(
            plan.next_head,
            TileEffect {
                consumable: plan.consumable,
                growth_amount: plan.growth_amount,
                score_gain: plan.score_gain,
                hits_bomb: plan.hits_bomb,
            },
        );
    }

    /// 判断玩家下一步是否会导致游戏结束。
    fn player_hits_hazard_or_self(&self, next_head: Position, player_effect: TileEffect) -> bool {
        self.hit_wall(next_head)
            || player_effect.hits_bomb
            || self.occupies_with_tail_rules(
                self.player.body(),
                next_head,
                self.snake_grows(&self.player.snake, player_effect.growth_amount),
            )
    }

    /// 判断玩家下一步是否会撞上敌蛇身体；头撞头单独处理。
    fn player_hits_enemy_body(&self, next_head: Position, enemy_plans: &[EnemyPlan]) -> bool {
        self.enemies.iter().enumerate().any(|(enemy_index, _)| {
            self.enemy_occupies_position(enemy_index, next_head, enemy_plans)
        })
    }

    /// 判断指定 AI 下一步是否会撞上墙、炸弹或自身。
    fn enemy_hits_hazard_or_self(&self, enemy_index: usize, enemy_plans: &[EnemyPlan]) -> bool {
        let enemy = &self.enemies[enemy_index];
        let plan = enemy_plans[enemy_index];

        self.hit_wall(plan.next_head)
            || plan.hits_bomb
            || self.occupies_with_tail_rules(
                enemy.body(),
                plan.next_head,
                self.snake_grows(&enemy.snake, plan.growth_amount),
            )
    }

    /// 判断指定 AI 下一步是否会撞上玩家身体；头撞头单独处理。
    fn enemy_hits_player_body(
        &self,
        enemy_index: usize,
        player_grows: bool,
        player_next: Position,
        enemy_plans: &[EnemyPlan],
    ) -> bool {
        let next_head = enemy_plans[enemy_index].next_head;
        next_head != player_next
            && self.occupies_with_tail_rules(self.player.body(), next_head, player_grows)
    }

    /// 判断指定 AI 下一步是否会撞上其他 AI 身体；头撞头单独处理。
    fn enemy_hits_enemy_body(&self, enemy_index: usize, enemy_plans: &[EnemyPlan]) -> bool {
        let plan = enemy_plans[enemy_index];

        self.enemies.iter().enumerate().any(|(other_index, _)| {
            other_index != enemy_index
                && self.enemy_occupies_position(other_index, plan.next_head, enemy_plans)
        })
    }

    /// 结算玩家与 AI 的头撞头规则：体型较小的一方死亡，同体型同死。
    ///
    /// # 参数
    /// - `player_next`: 玩家下一步位置
    /// - `player_effect`: 玩家下一步的格子效果
    /// - `enemy_plans`: 所有 AI 的移动计划
    /// - `player_dies`: 玩家是否死亡的输出参数
    /// - `enemy_dies`: AI 是否死亡的输出数组
    ///
    /// # 结算规则
    /// - 玩家体型 > AI 体型：AI 死亡
    /// - 玩家体型 < AI 体型：玩家死亡
    /// - 玩家体型 = AI 体型：双方同死
    pub(super) fn resolve_player_enemy_head_on(
        &self,
        player_next: Position,
        player_effect: TileEffect,
        enemy_plans: &[EnemyPlan],
        player_dies: &mut bool,
        enemy_dies: &mut [bool],
    ) {
        // 计算玩家在本次移动后的体型（包含即将增长的部分）
        let player_length = self.projected_length(&self.player.snake, player_effect.growth_amount);

        // 遍历所有 AI，检查是否有头撞头
        for (enemy_index, plan) in enemy_plans.iter().enumerate() {
            // 跳过不与玩家头撞头的 AI
            if plan.next_head != player_next {
                continue;
            }

            // 计算该 AI 在本次移动后的体型
            let enemy_length =
                self.projected_length(&self.enemies[enemy_index].snake, plan.growth_amount);

            // 根据体型比较决定生死
            if player_length > enemy_length {
                // 玩家体型更大，AI 死亡
                enemy_dies[enemy_index] = true;
            } else if player_length < enemy_length {
                // AI 体型更大，玩家死亡
                *player_dies = true;
            } else {
                // 体型相同，双方同死
                *player_dies = true;
                enemy_dies[enemy_index] = true;
            }
        }
    }

    /// 结算所有 AI 之间的头撞头规则：体型较小的一方死亡，同体型同死。
    ///
    /// 使用双重循环检查所有 AI 对，避免重复比较。
    /// 只比较 `enemy_index < other_index` 的对，确保每对只处理一次。
    fn resolve_enemy_head_on(&self, enemy_plans: &[EnemyPlan], enemy_dies: &mut [bool]) {
        // 外层循环：遍历每条 AI
        for enemy_index in 0..enemy_plans.len() {
            // 内层循环：只与索引更大的 AI 比较，避免重复
            for other_index in (enemy_index + 1)..enemy_plans.len() {
                // 跳过不发生头撞头的 AI 对
                if enemy_plans[enemy_index].next_head != enemy_plans[other_index].next_head {
                    continue;
                }

                // 计算两条 AI 的体型
                let enemy_length = self.projected_length(
                    &self.enemies[enemy_index].snake,
                    enemy_plans[enemy_index].growth_amount,
                );
                let other_length = self.projected_length(
                    &self.enemies[other_index].snake,
                    enemy_plans[other_index].growth_amount,
                );

                // 根据体型比较决定生死
                if enemy_length > other_length {
                    // 第一条 AI 体型更大，第二条死亡
                    enemy_dies[other_index] = true;
                } else if enemy_length < other_length {
                    // 第二条 AI 体型更大，第一条死亡
                    enemy_dies[enemy_index] = true;
                } else {
                    // 体型相同，双方同死
                    enemy_dies[enemy_index] = true;
                    enemy_dies[other_index] = true;
                }
            }
        }
    }

    /// 根据当前位置和方向计算下一步位置。
    pub(super) fn next_position(&self, head: Position, direction: Direction) -> Position {
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
    pub(super) fn hit_wall(&self, position: Position) -> bool {
        position.x >= self.width || position.y >= self.height
    }

    /// 按尾巴是否会移动的规则判断某条蛇是否占用了指定位置。
    pub(super) fn occupies_with_tail_rules(
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

    /// 判断玩家蛇在本 tick 结束前是否占据指定位置。
    pub(super) fn player_occupies_position(&self, position: Position, growth_amount: u16) -> bool {
        self.occupies_with_tail_rules(
            self.player.body(),
            position,
            self.snake_grows(&self.player.snake, growth_amount),
        )
    }

    /// 判断指定 AI 蛇在本 tick 结束前是否占据指定位置。
    pub(super) fn enemy_occupies_position(
        &self,
        enemy_index: usize,
        position: Position,
        enemy_plans: &[EnemyPlan],
    ) -> bool {
        let enemy = &self.enemies[enemy_index];
        let growth_amount = enemy_plans
            .get(enemy_index)
            .map(|plan| plan.growth_amount)
            .unwrap_or(0);

        self.occupies_with_tail_rules(
            enemy.body(),
            position,
            self.snake_grows(&enemy.snake, growth_amount),
        )
    }

    /// 按配置数量补齐所有物品。
    pub(super) fn refill_items(&mut self) {
        self.refill_food_positions(FOOD_COUNT);
        self.refill_super_food_positions(SUPER_FOOD_COUNT);
        self.refill_bomb_positions(BOMB_COUNT);
    }

    /// 按目标数量补齐普通食物。
    fn refill_food_positions(&mut self, target: usize) {
        while self.foods.len() < target && self.empty_cell_count() > 0 {
            self.foods.push(self.random_empty_position());
        }
    }

    /// 按目标数量补齐超级食物。
    fn refill_super_food_positions(&mut self, target: usize) {
        while self.super_foods.len() < target && self.empty_cell_count() > 0 {
            self.super_foods.push(self.random_empty_position());
        }
    }

    /// 按目标数量补齐炸弹。
    fn refill_bomb_positions(&mut self, target: usize) {
        while self.bombs.len() < target && self.empty_cell_count() > 0 {
            self.bombs.push(self.random_empty_position());
        }
    }

    /// 返回指定位置对应的格子效果。
    pub(super) fn tile_effect(&self, position: Position) -> TileEffect {
        if self.foods.contains(&position) || self.legacy_foods.contains(&position) {
            return TileEffect {
                consumable: Some(ConsumableKind::Food),
                growth_amount: FOOD_GROWTH_AMOUNT,
                score_gain: FOOD_SCORE_GAIN,
                hits_bomb: false,
            };
        }

        if self.super_foods.contains(&position) {
            return TileEffect {
                consumable: Some(ConsumableKind::SuperFood),
                growth_amount: SUPER_FOOD_GROWTH_AMOUNT,
                score_gain: SUPER_FOOD_SCORE_GAIN,
                hits_bomb: false,
            };
        }

        TileEffect {
            consumable: None,
            growth_amount: 0,
            score_gain: 0,
            hits_bomb: self.bombs.contains(&position),
        }
    }

    /// 从棋盘上消费一个格子效果。
    fn consume_tile(&mut self, position: Position, effect: TileEffect) {
        match effect.consumable {
            Some(ConsumableKind::Food) => self.remove_food(position),
            Some(ConsumableKind::SuperFood) => self.remove_super_fruit(position),
            None => {}
        }

        if effect.hits_bomb {
            self.remove_bomb(position);
        }
    }

    /// 计算某条蛇在本次前进中是否会增长。
    fn snake_grows(&self, snake: &Snake, growth_amount: u16) -> bool {
        snake.pending_growth > 0 || growth_amount > 0
    }

    /// 计算蛇在本次移动结算后会表现出的体型长度。
    fn projected_length(&self, snake: &Snake, growth_amount: u16) -> usize {
        snake.body.len() + usize::from(self.snake_grows(snake, growth_amount))
    }

    /// 推进一条蛇，并根据成长值决定是否保留尾巴。
    ///
    /// 蛇身增长的实现采用"延迟增长"机制：
    /// - 吃到食物时，`pending_growth` 记录待增长的节数
    /// - 每次移动时，如果 `pending_growth > 0`，则不移除尾巴，并递减计数
    /// - 这样蛇会逐渐变长，而不是一次性增长
    ///
    /// # 参数
    /// - `snake`: 要推进的蛇（可变引用）
    /// - `next_head`: 新蛇头的位置
    /// - `growth_amount`: 本次移动带来的额外增长节数（如吃到食物）
    /// - `score_gain`: 本次移动带来的分数增益
    fn advance_snake(snake: &mut Snake, next_head: Position, growth_amount: u16, score_gain: u32) {
        snake.body.push_back(next_head);
        snake.score += score_gain;

        let total_growth = snake.pending_growth.saturating_add(growth_amount);
        if total_growth > 0 {
            snake.pending_growth = total_growth.saturating_sub(1);
        } else {
            snake.body.pop_front();
        }
    }

    /// 将死亡蛇的身体转成普通食物，供其他蛇争夺。
    fn drop_legacy_from_body(&mut self, body: &VecDeque<Position>) {
        for &segment in body {
            if !self.foods.contains(&segment)
                && !self.legacy_foods.contains(&segment)
                && !self.super_foods.contains(&segment)
                && !self.bombs.contains(&segment)
            {
                self.legacy_foods.push(segment);
            }
        }
    }

    /// 从棋盘上移除一颗被吃掉的普通食物。
    fn remove_food(&mut self, position: Position) {
        if let Some(index) = self.foods.iter().position(|food| *food == position) {
            self.foods.swap_remove(index);
            return;
        }

        if let Some(index) = self.legacy_foods.iter().position(|food| *food == position) {
            self.legacy_foods.swap_remove(index);
        }
    }

    /// 从棋盘上移除一颗被吃掉的超级果实。
    fn remove_super_fruit(&mut self, position: Position) {
        if let Some(index) = self.super_foods.iter().position(|fruit| *fruit == position) {
            self.super_foods.swap_remove(index);
        }
    }

    /// 从棋盘上移除一个炸弹。
    fn remove_bomb(&mut self, position: Position) {
        if let Some(index) = self.bombs.iter().position(|bomb| *bomb == position) {
            self.bombs.swap_remove(index);
        }
    }

    /// 返回当前仍可用于生成物品的空格数。
    fn empty_cell_count(&self) -> usize {
        let area = self.width as usize * self.height as usize;
        let occupied_by_snakes = self.player.body().len()
            + self
                .enemies
                .iter()
                .map(|enemy| enemy.body().len())
                .sum::<usize>();
        let occupied_by_items =
            self.foods.len() + self.legacy_foods.len() + self.super_foods.len() + self.bombs.len();

        area.saturating_sub(occupied_by_snakes + occupied_by_items)
    }

    /// 随机生成一个不与任意蛇身或食物重叠的位置。
    ///
    /// 使用拒绝采样算法：随机生成候选位置，直到找到一个空位。
    /// 该方法在棋盘空间充足时效率较高，但当棋盘接近满载时可能需要多次尝试。
    ///
    /// 排除的位置包括：
    /// - 玩家蛇身
    /// - 所有 AI 蛇身
    /// - 普通食物
    /// - 尸体食物（legacy_foods）
    /// - 超级食物
    /// - 炸弹
    ///
    /// # 返回值
    /// 返回一个空位置的坐标
    ///
    /// # 注意
    /// 调用前应确保 `empty_cell_count() > 0`，否则会陷入无限循环
    fn random_empty_position(&self) -> Position {
        let mut rng = rand::rng();

        loop {
            let candidate = Position {
                x: rng.random_range(0..self.width),
                y: rng.random_range(0..self.height),
            };

            if !self.player.body().contains(&candidate)
                && !self.foods.contains(&candidate)
                && !self.legacy_foods.contains(&candidate)
                && !self.super_foods.contains(&candidate)
                && !self.bombs.contains(&candidate)
                && !self
                    .enemies
                    .iter()
                    .any(|enemy| enemy.body().contains(&candidate))
            {
                return candidate;
            }
        }
    }

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

    /// 计算两个坐标之间的曼哈顿距离。
    pub(super) fn manhattan_distance(a: Position, b: Position) -> u16 {
        a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
    }

    /// 判断两个方向是否互为反方向。
    pub(super) fn is_opposite(current: Direction, next: Direction) -> bool {
        matches!(
            (current, next),
            (Direction::Up, Direction::Down)
                | (Direction::Down, Direction::Up)
                | (Direction::Left, Direction::Right)
                | (Direction::Right, Direction::Left)
        )
    }
}
