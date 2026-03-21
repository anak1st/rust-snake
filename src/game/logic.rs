use std::collections::VecDeque;

use rand::Rng;

use crate::config::game::{
    BOMB_COUNT, CORPSE_DECAY_INTERVAL_TICKS, FOOD_COUNT, FOOD_GROWTH_AMOUNT, FOOD_SCORE_GAIN,
    SUPER_FOOD_COUNT, SUPER_FOOD_GROWTH_AMOUNT, SUPER_FOOD_SCORE_GAIN,
};

use super::{
    ConsumableKind, CorpsePiece, Direction, GameState, PendingEnemyRespawn, Position, RunState,
    SnakePlan, TileEffect,
};

impl GameState {
    /// 推进一帧游戏逻辑，处理玩家、AI、食物和碰撞。
    ///
    /// tick 是核心逻辑推进函数，处理流程分为以下几个阶段：
    ///
    /// - **尸块推进**：让已有尸块按时间独立腐化，并在整批尸块消失后触发敌蛇重生
    /// - **控制解析**：根据蛇的控制模式决定本帧实际方向
    /// - **玩家移动计算**：计算玩家下一步位置和格子效果
    /// - **敌蛇 AI 规划**：为每条仍存活的敌蛇预规划下一步移动
    /// - **碰撞检测**：判断玩家和 AI 是否会撞死
    /// - **头撞头结算**：处理玩家与 AI、AI 与 AI 的头撞头情况
    /// - **状态更新**：推进存活的蛇，并把死亡蛇拆成独立尸块
    /// - **收尾工作**：补充物品、递增 tick 计数
    pub fn tick(&mut self) {
        if self.state != RunState::Running {
            return;
        }

        self.recent_events.clear();
        self.advance_corpse_pieces();

        let player_plan = if self.player.is_ai_controlled() {
            Some(self.player.plan_ai_move(self))
        } else {
            self.player.sync_control_direction();
            None
        };
        let player_next = player_plan
            .map(|plan| plan.next_head)
            .unwrap_or_else(|| self.next_position(self.player_head(), self.player.direction()));
        let player_effect = player_plan
            .map(SnakePlan::tile_effect)
            .unwrap_or_else(|| self.tile_effect(player_next));

        let mut enemy_plans = Vec::with_capacity(self.enemies.len());
        for enemy_index in 0..self.enemies.len() {
            let enemy = &self.enemies[enemy_index];
            enemy_plans.push(enemy.is_alive().then(|| enemy.plan_ai_move(self)));
        }

        let player_grows = self.player.grows(player_effect.growth_amount);
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

        self.resolve_player_enemy_head_on(
            player_next,
            player_effect,
            &enemy_plans,
            &mut player_dies,
            &mut enemy_dies,
        );
        self.resolve_enemy_head_on(&enemy_plans, &mut enemy_dies);

        for (plan, dies) in enemy_plans.iter_mut().zip(enemy_dies.iter().copied()) {
            if let Some(plan) = plan {
                plan.crashes = dies;
            }
        }

        let player_body_before_crash = self.player.body().clone();
        let enemy_bodies_before_crash = self
            .enemies
            .iter()
            .map(|enemy| enemy.body().clone())
            .collect::<Vec<_>>();

        if !player_dies {
            self.advance_player(player_next, player_effect, player_plan);
        }

        for (enemy_index, plan) in enemy_plans.into_iter().enumerate() {
            let Some(plan) = plan else {
                continue;
            };

            if plan.crashes {
                let appearance = self.enemies[enemy_index].appearance;
                self.begin_enemy_corpse(
                    enemy_index,
                    &enemy_bodies_before_crash[enemy_index],
                    appearance,
                );
                self.enemies[enemy_index].remove_from_board();
                self.enemies[enemy_index].reset_score();
            } else {
                self.advance_enemy(enemy_index, plan);
            }
        }

        if player_dies {
            self.begin_player_corpse(&player_body_before_crash, self.player.appearance);
            self.player.remove_from_board();
            self.state = RunState::GameOver;
        }

        self.refill_items();
        self.tick_count += 1;
    }

    /// 让玩家蛇前进一步，并处理吃到物品后的增长。
    fn advance_player(&mut self, next_head: Position, effect: TileEffect, plan: Option<SnakePlan>) {
        if let Some(plan) = plan {
            self.player.apply_navigation(plan.navigation);
        }
        self.player
            .advance(next_head, effect.growth_amount, effect.score_gain);
        self.consume_tile(next_head, effect);
    }

    /// 让指定 AI 前进一步，并处理吃到物品后的增长。
    fn advance_enemy(&mut self, enemy_index: usize, plan: SnakePlan) {
        let enemy = &mut self.enemies[enemy_index];
        enemy.apply_navigation(plan.navigation);
        enemy.advance(plan.next_head, plan.growth_amount, plan.score_gain);
        self.consume_tile(plan.next_head, plan.tile_effect());
    }

    /// 判断玩家下一步是否会导致游戏结束。
    fn player_hits_hazard_or_self(&self, next_head: Position, player_effect: TileEffect) -> bool {
        self.hit_wall(next_head)
            || player_effect.hits_bomb
            || self.corpse_piece_occupies_position(next_head)
            || self.occupies_with_tail_rules(
                self.player.body(),
                next_head,
                self.player.grows(player_effect.growth_amount),
            )
    }

    /// 判断玩家下一步是否会撞上敌蛇身体；头撞头单独处理。
    fn player_hits_enemy_body(
        &self,
        next_head: Position,
        enemy_plans: &[Option<SnakePlan>],
    ) -> bool {
        self.enemies.iter().enumerate().any(|(enemy_index, _)| {
            self.enemy_occupies_position(enemy_index, next_head, enemy_plans)
        })
    }

    /// 判断指定 AI 下一步是否会撞上墙、炸弹、尸块或自身。
    fn enemy_hits_hazard_or_self(
        &self,
        enemy_index: usize,
        enemy_plans: &[Option<SnakePlan>],
    ) -> bool {
        let enemy = &self.enemies[enemy_index];
        let Some(plan) = enemy_plans[enemy_index] else {
            return false;
        };

        self.hit_wall(plan.next_head)
            || plan.hits_bomb
            || self.corpse_piece_occupies_position(plan.next_head)
            || self.occupies_with_tail_rules(
                enemy.body(),
                plan.next_head,
                enemy.grows(plan.growth_amount),
            )
    }

    /// 判断指定 AI 下一步是否会撞上玩家身体；头撞头单独处理。
    fn enemy_hits_player_body(
        &self,
        enemy_index: usize,
        player_grows: bool,
        player_next: Position,
        enemy_plans: &[Option<SnakePlan>],
    ) -> bool {
        let Some(plan) = enemy_plans[enemy_index] else {
            return false;
        };

        let next_head = plan.next_head;
        next_head != player_next
            && self.player.is_alive()
            && self.occupies_with_tail_rules(self.player.body(), next_head, player_grows)
    }

    /// 判断指定 AI 下一步是否会撞上其他 AI 身体；头撞头单独处理。
    fn enemy_hits_enemy_body(&self, enemy_index: usize, enemy_plans: &[Option<SnakePlan>]) -> bool {
        let Some(plan) = enemy_plans[enemy_index] else {
            return false;
        };

        self.enemies.iter().enumerate().any(|(other_index, _)| {
            other_index != enemy_index
                && self.enemy_occupies_position(other_index, plan.next_head, enemy_plans)
        })
    }

    /// 结算玩家与 AI 的头撞头规则：体型较小的一方死亡，同体型同死。
    pub(super) fn resolve_player_enemy_head_on(
        &self,
        player_next: Position,
        player_effect: TileEffect,
        enemy_plans: &[Option<SnakePlan>],
        player_dies: &mut bool,
        enemy_dies: &mut [bool],
    ) {
        let player_length = self.player.projected_length(player_effect.growth_amount);

        for (enemy_index, plan) in enemy_plans.iter().enumerate() {
            let Some(plan) = plan else {
                continue;
            };

            if plan.next_head != player_next {
                continue;
            }

            let enemy_length = self.enemies[enemy_index].projected_length(plan.growth_amount);

            if player_length > enemy_length {
                enemy_dies[enemy_index] = true;
            } else if player_length < enemy_length {
                *player_dies = true;
            } else {
                *player_dies = true;
                enemy_dies[enemy_index] = true;
            }
        }
    }

    /// 结算所有 AI 之间的头撞头规则：体型较小的一方死亡，同体型同死。
    fn resolve_enemy_head_on(&self, enemy_plans: &[Option<SnakePlan>], enemy_dies: &mut [bool]) {
        for enemy_index in 0..enemy_plans.len() {
            let Some(enemy_plan) = enemy_plans[enemy_index] else {
                continue;
            };

            for other_index in (enemy_index + 1)..enemy_plans.len() {
                let Some(other_plan) = enemy_plans[other_index] else {
                    continue;
                };

                if enemy_plan.next_head != other_plan.next_head {
                    continue;
                }

                let enemy_length =
                    self.enemies[enemy_index].projected_length(enemy_plan.growth_amount);
                let other_length =
                    self.enemies[other_index].projected_length(other_plan.growth_amount);

                if enemy_length > other_length {
                    enemy_dies[other_index] = true;
                } else if enemy_length < other_length {
                    enemy_dies[enemy_index] = true;
                } else {
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
        if !self.player.is_alive() {
            return false;
        }

        self.occupies_with_tail_rules(
            self.player.body(),
            position,
            self.player.grows(growth_amount),
        )
    }

    /// 判断指定 AI 蛇在本 tick 结束前是否占据指定位置。
    pub(super) fn enemy_occupies_position(
        &self,
        enemy_index: usize,
        position: Position,
        enemy_plans: &[Option<SnakePlan>],
    ) -> bool {
        let enemy = &self.enemies[enemy_index];
        if !enemy.is_alive() {
            return false;
        }

        let growth_amount = enemy_plans
            .get(enemy_index)
            .and_then(|plan| plan.as_ref())
            .map(|plan| plan.growth_amount)
            .unwrap_or(0);

        self.occupies_with_tail_rules(enemy.body(), position, enemy.grows(growth_amount))
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

    /// 将玩家尸体拆成独立尸块。
    fn begin_player_corpse(
        &mut self,
        body: &VecDeque<Position>,
        appearance: super::SnakeAppearance,
    ) {
        self.begin_corpse(body, appearance, None);
    }

    /// 将敌蛇尸体拆成独立尸块，并记录其重生依赖。
    pub(super) fn begin_enemy_corpse(
        &mut self,
        enemy_index: usize,
        body: &VecDeque<Position>,
        appearance: super::SnakeAppearance,
    ) {
        self.begin_corpse(body, appearance, Some(enemy_index));
    }

    /// 将一条蛇的整段身体拆成独立尸块。
    fn begin_corpse(
        &mut self,
        body: &VecDeque<Position>,
        appearance: super::SnakeAppearance,
        enemy_index: Option<usize>,
    ) {
        if body.is_empty() {
            return;
        }

        let group_id = self.next_corpse_group_id;
        self.next_corpse_group_id += 1;

        let body_len = body.len();
        for (index, &segment) in body.iter().enumerate() {
            let is_head = index + 1 == body_len;
            let step = (body_len - index) as u64;
            let decays_at_tick = self.tick_count + CORPSE_DECAY_INTERVAL_TICKS.saturating_mul(step);
            let (glyph, color, bold) = if is_head {
                (appearance.head_glyph, appearance.head_color, true)
            } else {
                (appearance.body_glyph, appearance.body_color, false)
            };

            self.corpse_pieces.push(CorpsePiece::new(
                segment,
                group_id,
                glyph,
                color,
                bold,
                decays_at_tick,
            ));
        }

        if let Some(enemy_index) = enemy_index {
            self.pending_enemy_respawns.push(PendingEnemyRespawn {
                group_id,
                enemy_index,
            });
        }
    }

    /// 推进所有尸块的独立腐化过程。
    fn advance_corpse_pieces(&mut self) {
        let current_tick = self.tick_count;
        let mut decayed_positions = Vec::new();

        self.corpse_pieces.retain(|piece| {
            if piece.should_decay(current_tick) {
                decayed_positions.push(piece.position());
                false
            } else {
                true
            }
        });

        for position in decayed_positions {
            self.drop_legacy_at(position);
        }

        let mut pending_index = 0;
        while pending_index < self.pending_enemy_respawns.len() {
            let pending = self.pending_enemy_respawns[pending_index];
            let still_has_pieces = self
                .corpse_pieces
                .iter()
                .any(|piece| piece.group_id() == pending.group_id);

            if !still_has_pieces && self.respawn_enemy(pending.enemy_index) {
                self.pending_enemy_respawns.swap_remove(pending_index);
            } else {
                pending_index += 1;
            }
        }
    }

    /// 判断当前是否仍有尸块占据指定位置。
    pub(super) fn corpse_piece_occupies_position(&self, position: Position) -> bool {
        self.corpse_pieces
            .iter()
            .any(|piece| piece.position() == position)
    }

    /// 将一个尸块所在格转成普通食物。
    fn drop_legacy_at(&mut self, position: Position) {
        if !self.foods.contains(&position)
            && !self.legacy_foods.contains(&position)
            && !self.super_foods.contains(&position)
            && !self.bombs.contains(&position)
            && !self.player.body().contains(&position)
            && !self
                .enemies
                .iter()
                .filter(|enemy| enemy.is_alive())
                .any(|enemy| enemy.body().contains(&position))
            && !self.corpse_piece_occupies_position(position)
        {
            self.legacy_foods.push(position);
            self.recent_events
                .push(super::GameEvent::CorpseFoodCreated(position));
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
                .filter(|enemy| enemy.is_alive())
                .map(|enemy| enemy.body().len())
                .sum::<usize>();
        let occupied_by_items = self.foods.len()
            + self.legacy_foods.len()
            + self.super_foods.len()
            + self.bombs.len()
            + self.corpse_pieces.len();

        area.saturating_sub(occupied_by_snakes + occupied_by_items)
    }

    /// 随机生成一个不与任意蛇身、尸块或食物重叠的位置。
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
                && !self.corpse_piece_occupies_position(candidate)
                && !self
                    .enemies
                    .iter()
                    .filter(|enemy| enemy.is_alive())
                    .any(|enemy| enemy.body().contains(&candidate))
            {
                return candidate;
            }
        }
    }
}
