//! 封装 AI 蛇的路径选择与安全性评估。

use std::collections::VecDeque;

use rand::Rng;
use rand::seq::SliceRandom;

use crate::config::game::{
    AI_RANDOM_WALK_CHANCE_PERCENT, AI_RANDOM_WALK_MAX_STEPS, AI_RANDOM_WALK_MIN_STEPS,
};

use super::{Direction, GameState, NavigationDecision, Position, Snake, SnakePlan};

const ALL_DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
];

/// 表示某个候选方向对 AI 的风险等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoveRisk {
    /// 不会立即死亡，且落点后的连通空间足够展开。
    Safe,
    /// 不会立即死亡，但可能把自己压进狭小区域。
    TightSpace,
    /// 会立即撞死或根本不是合法候选方向。
    Deadly,
}

/// 表示 AI 某一层策略产出的移动意图。
#[derive(Debug, Clone)]
struct NavigationIntent {
    /// 这层策略最想采用的方向。
    direction: Direction,
    /// 如果主意图风险过高，可用于逃生的候选方向顺序。
    escape_directions: Vec<Direction>,
    /// 应写回 AI 状态中的随机漫步剩余步数。
    random_walk_steps: u8,
    /// 逃生后是否继续维持随机漫步状态。
    preserve_random_walk: bool,
}

impl Snake {
    // ==================== Decision: 决定 AI 想走哪一类方向 ====================
    // 这一段负责生成移动意图与候选顺序，例如继续随机漫步、开始随机漫步、
    // 朝食物靠近，以及在主意图失败时挑选一个可接受的回退方向。

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
    /// 1. **继续随机漫步**：如果当前正处于随机漫步中，优先延续原方向
    /// 2. **触发随机漫步**：按配置概率进入随机漫步模式，持续配置指定的步数范围
    /// 3. **追逐食物**：计算最近的食物位置，选择更接近目标的方向
    /// 4. **风险筛选**：统一检查主意图是否安全，不安全时再从候选方向中逃生
    /// 5. **兜底保持方向**：当没有更好的候选方向时，保持当前方向
    ///
    /// # 返回值
    /// 返回包含方向、随机漫步步数和方向的导航决策
    fn choose_direction(&self, game: &GameState) -> NavigationDecision {
        let intent = if let Some(intent) = self.continue_random_walk_intent() {
            intent
        } else if let Some(intent) = self.start_random_walk_intent() {
            intent
        } else if let Some(intent) = self.food_navigation_intent(game) {
            intent
        } else {
            return Self::steady_navigation(self.direction());
        };

        self.resolve_navigation(game, intent)
    }

    /// 在已有随机漫步状态时，优先尝试延续原方向。
    fn continue_random_walk_intent(&self) -> Option<NavigationIntent> {
        let ai_state = self.ai_state();
        if ai_state.random_walk_steps == 0 {
            return None;
        }

        ai_state
            .random_walk_direction
            .map(|direction| NavigationIntent {
                direction,
                escape_directions: self.shuffled_directions(),
                random_walk_steps: ai_state.random_walk_steps.saturating_sub(1),
                preserve_random_walk: true,
            })
    }

    /// 按配置概率进入新的随机漫步。
    fn start_random_walk_intent(&self) -> Option<NavigationIntent> {
        let mut rng = rand::rng();
        if rng.random_range(0..100) >= AI_RANDOM_WALK_CHANCE_PERCENT {
            return None;
        }

        let steps = rng.random_range(AI_RANDOM_WALK_MIN_STEPS..=AI_RANDOM_WALK_MAX_STEPS);
        let directions = self.shuffled_directions();

        self.first_non_opposite_direction(&directions)
            .map(|direction| NavigationIntent {
                direction,
                escape_directions: directions,
                random_walk_steps: steps,
                preserve_random_walk: true,
            })
    }

    /// 先尝试朝最近食物前进。
    fn food_navigation_intent(&self, game: &GameState) -> Option<NavigationIntent> {
        let target = game.closest_consumable_to(self.head());
        let preferred = game.preferred_directions(self.head(), target);

        self.first_non_opposite_direction(&preferred)
            .map(|direction| NavigationIntent {
                direction,
                escape_directions: preferred,
                random_walk_steps: 0,
                preserve_random_walk: false,
            })
    }

    /// 返回一组打乱后的方向顺序，用于随机漫步或无偏逃生。
    fn shuffled_directions(&self) -> Vec<Direction> {
        let mut directions = ALL_DIRECTIONS.to_vec();
        let mut rng = rand::rng();
        directions.shuffle(&mut rng);
        directions
    }

    /// 返回候选列表中第一个非掉头方向。
    fn first_non_opposite_direction(&self, directions: &[Direction]) -> Option<Direction> {
        directions
            .iter()
            .copied()
            .find(|&direction| self.is_turn_allowed(direction))
    }

    /// 判断某个方向是否不是当前方向的直接反向。
    fn is_turn_allowed(&self, direction: Direction) -> bool {
        !self.direction().is_opposite(direction)
    }

    /// 返回一个不携带随机漫步状态的普通导航结果。
    fn steady_navigation(direction: Direction) -> NavigationDecision {
        NavigationDecision {
            direction,
            random_walk_steps: 0,
            random_walk_direction: None,
        }
    }

    /// 统一处理某个意图的风险检查与避险回退。
    ///
    /// 如果主意图方向足够安全，则直接采用；
    /// 否则只在这里集中挑选逃生方向。
    fn resolve_navigation(&self, game: &GameState, intent: NavigationIntent) -> NavigationDecision {
        if self
            .direction_risk(game, intent.direction)
            .accept_for_intent()
        {
            return self.navigation_for_direction(
                intent.direction,
                intent.random_walk_steps,
                intent.preserve_random_walk,
            );
        }

        self.choose_escape_navigation(
            game,
            intent.escape_directions,
            intent.random_walk_steps,
            intent.preserve_random_walk,
        )
        .unwrap_or_else(|| Self::steady_navigation(self.direction()))
    }

    /// 按给定方向和 AI 状态生成最终导航结果。
    fn navigation_for_direction(
        &self,
        direction: Direction,
        random_walk_steps: u8,
        preserve_random_walk: bool,
    ) -> NavigationDecision {
        NavigationDecision {
            direction,
            random_walk_steps,
            random_walk_direction: preserve_random_walk.then_some(direction),
        }
    }

    /// 在意图方向不可接受时，从候选集中挑一个尽量安全的逃生方向。
    ///
    /// 优先选择 `Safe`，其次才接受 `TightSpace`。
    fn choose_escape_navigation(
        &self,
        game: &GameState,
        directions: impl IntoIterator<Item = Direction>,
        random_walk_steps: u8,
        preserve_random_walk: bool,
    ) -> Option<NavigationDecision> {
        self.choose_escape_direction(game, directions)
            .map(|direction| {
                self.navigation_for_direction(direction, random_walk_steps, preserve_random_walk)
            })
    }

    /// 从候选方向中挑选一个可存活的方向。
    ///
    /// 如果存在 `Safe` 方向，优先返回；
    /// 否则回退到第一个 `TightSpace` 方向，避免在绝境中直接放弃可走的路。
    fn choose_escape_direction(
        &self,
        game: &GameState,
        directions: impl IntoIterator<Item = Direction>,
    ) -> Option<Direction> {
        let mut fallback = None;

        for direction in directions {
            match self.direction_risk(game, direction) {
                MoveRisk::Safe => return Some(direction),
                MoveRisk::TightSpace if fallback.is_none() => fallback = Some(direction),
                MoveRisk::TightSpace | MoveRisk::Deadly => {}
            }
        }

        fallback
    }

    /// 评估某个方向对当前 AI 的风险等级。
    fn direction_risk(&self, game: &GameState, direction: Direction) -> MoveRisk {
        if self.direction().is_opposite(direction) {
            return MoveRisk::Deadly;
        }

        let next = game.next_position(self.head(), direction);
        if !game.snake_step_is_safe(self, next) {
            return MoveRisk::Deadly;
        }

        if !game.snake_step_has_adequate_space(self, next) {
            return MoveRisk::TightSpace;
        }

        MoveRisk::Safe
    }
}

impl MoveRisk {
    /// 判断这个风险等级是否足以接受为“主意图方向”。
    fn accept_for_intent(self) -> bool {
        matches!(self, Self::Safe)
    }
}

impl GameState {
    // ==================== Risk: 判断某一步是否危险或会立即死亡 ====================
    // 这一段只回答“这步能不能走、风险有多大”，不负责决定优先走哪条路线。
    // 这里会综合墙体、尸块、炸弹、其他蛇占位和头撞头输赢等规则。

    /// 判断一条蛇按当前环境前进一步是否安全（不会立即撞死）。
    ///
    /// 处理步骤：
    /// - 检查是否撞墙
    /// - 检查蛇是否存活
    /// - 检查是否撞到自身或尸块（考虑尾巴移动规则）
    /// - 检查是否撞到炸弹或其他蛇
    pub(super) fn snake_step_is_safe(&self, snake: &Snake, next: Position) -> bool {
        if self.hit_wall(next) || !snake.is_alive() {
            return false;
        }

        let effect = self.tile_effect(next);
        let my_projected_length = snake.projected_length(effect.growth_amount);

        if self.occupies_with_tail_rules(snake.body(), next, snake.grows(effect.growth_amount))
            || self.corpse_piece_occupies_position(next)
        {
            return false;
        }

        let hits_non_wall_hazard = self.bombs.contains(&next)
            || self.other_snakes_occupy_position(snake, next)
            || self.other_snake_can_win_head_on(snake, next, my_projected_length);

        !hits_non_wall_hazard
    }

    /// 判断这一步走完后，蛇头所在连通区域是否仍足以容纳自身长度。
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

    // ==================== Projection: 投影一步后的蛇身与占位状态 ====================
    // 这一段负责把“如果走出这一步，身体会变成什么样”算出来，
    // 供风险评估与空间分析复用，避免把增长、尾巴移动等细节散落在各处。

    /// 判断一条蛇在不掉头的前提下，下一步是否有机会到达指定位置。
    fn snake_can_reach_position_next_tick(&self, snake: &Snake, position: Position) -> bool {
        ALL_DIRECTIONS.into_iter().any(|direction| {
            !snake.direction().is_opposite(direction)
                && self.next_position(snake.head(), direction) == position
        })
    }

    /// 判断一条蛇在下一步是否存在“会吃到东西从而保留尾巴”的可能。
    fn snake_might_grow_next_tick(&self, snake: &Snake) -> bool {
        snake.pending_growth > 0
            || ALL_DIRECTIONS
                .into_iter()
                .filter(|&direction| !snake.direction().is_opposite(direction))
                .any(|direction| {
                    let next = self.next_position(snake.head(), direction);
                    !self.hit_wall(next) && self.tile_effect(next).growth_amount > 0
                })
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

    // ==================== Space Analysis: 评估落点后的可展开空间 ====================
    // 这一段不关心目标值高不高，只关心“走到这里之后还有没有足够空间活下去”。
    // 主要通过构造阻挡图和 flood fill 来估算可达区域大小。

    /// 统计一条蛇完成指定落点后，蛇头仍能抵达的活动格数量。
    fn reachable_space_after_step(
        &self,
        snake: &Snake,
        next: Position,
        growth_amount: u16,
    ) -> usize {
        let mut blocked = vec![false; usize::from(self.width) * usize::from(self.height)];

        for corpse in &self.corpse_pieces {
            blocked[self.board_index(corpse.position())] = true;
        }

        for bomb in &self.bombs {
            blocked[self.board_index(*bomb)] = true;
        }

        if self.player.is_alive() && !std::ptr::eq(&self.player, snake) {
            self.mark_body_as_blocked(
                &mut blocked,
                self.player.body(),
                self.snake_might_grow_next_tick(&self.player),
            );
        }

        for enemy in &self.enemies {
            if enemy.is_alive() && !std::ptr::eq(enemy, snake) {
                self.mark_body_as_blocked(
                    &mut blocked,
                    enemy.body(),
                    self.snake_might_grow_next_tick(enemy),
                );
            }
        }

        let projected_body = self.projected_body_after_step(snake, next, growth_amount);
        for segment in projected_body
            .iter()
            .take(projected_body.len().saturating_sub(1))
        {
            blocked[self.board_index(*segment)] = true;
        }

        let start = self.board_index(next);
        blocked[start] = false;

        let mut visited = vec![false; blocked.len()];
        let mut frontier = VecDeque::from([next]);
        visited[start] = true;
        let mut reachable = 0;

        while let Some(position) = frontier.pop_front() {
            reachable += 1;

            for direction in ALL_DIRECTIONS {
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
    fn mark_body_as_blocked(&self, blocked: &mut [bool], body: &VecDeque<Position>, grows: bool) {
        for (index, segment) in body.iter().enumerate() {
            let is_tail = index == 0;
            if is_tail && !grows {
                continue;
            }

            blocked[self.board_index(*segment)] = true;
        }
    }

    /// 将棋盘坐标转换为连续数组下标。
    fn board_index(&self, position: Position) -> usize {
        usize::from(position.y) * usize::from(self.width) + usize::from(position.x)
    }

    // ==================== Targeting: 生成朝目标靠近的方向偏好 ====================
    // 这一段只负责“更想往哪边靠近目标”，例如寻找最近可吃物、给出靠近目标的方向序。
    // 它不处理安全性，真正的风险筛选会在上面的 Risk/Decision 阶段完成。

    /// 返回离指定坐标最近的一颗可食用物品。
    fn closest_consumable_to(&self, origin: Position) -> Position {
        self.foods
            .iter()
            .chain(self.legacy_foods.iter())
            .chain(self.super_foods.iter())
            .copied()
            .min_by_key(|food| origin.manhattan_distance(*food))
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

        for direction in ALL_DIRECTIONS {
            if !directions.contains(&direction) {
                directions.push(direction);
            }
        }

        directions
    }
}
