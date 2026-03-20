use rand::Rng;
use ratatui::style::Color;
use std::collections::VecDeque;

/// 默认生成的 AI 敌蛇数量。
const AI_SNAKE_COUNT: usize = 3;
/// 默认同时生成的食物数量。
const FOOD_COUNT: usize = 4;

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

/// 蛇的固定外观信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnakeAppearance {
    /// 蛇头显示符号。
    head_glyph: &'static str,
    /// 蛇身显示符号。
    body_glyph: &'static str,
    /// 蛇头颜色。
    head_color: Color,
    /// 蛇身颜色。
    body_color: Color,
}

/// 玩家和 AI 共用的基础蛇状态。
#[derive(Debug, Clone)]
pub struct Snake {
    /// 当前已生效的移动方向。
    direction: Direction,
    /// 蛇身，尾部在前、头部在后。
    body: VecDeque<Position>,
    /// 当前累计得分。
    score: u32,
    /// 该蛇的固定外观配置。
    appearance: SnakeAppearance,
}

/// 玩家蛇的完整状态。
#[derive(Debug, Clone)]
pub struct PlayerSnake {
    /// 基础蛇状态。
    snake: Snake,
    /// 玩家最新输入、将在下一帧生效的方向。
    pending_direction: Direction,
}

/// 单条 AI 敌蛇的完整状态。
#[derive(Debug, Clone)]
pub struct EnemySnake {
    /// 基础蛇状态。
    snake: Snake,
    /// 随机漫步剩余步数，为 0 时表示追逐食物。
    random_walk_steps: u8,
    /// 随机漫步方向。
    random_walk_direction: Option<Direction>,
}

impl Snake {
    /// 创建一条新的蛇。
    fn new(body: VecDeque<Position>, direction: Direction, appearance: SnakeAppearance) -> Self {
        Self {
            direction,
            body,
            score: 0,
            appearance,
        }
    }

    /// 返回当前移动方向。
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// 返回蛇身坐标。
    pub fn body(&self) -> &VecDeque<Position> {
        &self.body
    }

    /// 返回累计得分。
    pub fn score(&self) -> u32 {
        self.score
    }

    /// 返回蛇头符号。
    pub fn head_glyph(&self) -> &'static str {
        self.appearance.head_glyph
    }

    /// 返回蛇身符号。
    pub fn body_glyph(&self) -> &'static str {
        self.appearance.body_glyph
    }

    /// 返回蛇头颜色。
    pub fn head_color(&self) -> Color {
        self.appearance.head_color
    }

    /// 返回蛇身颜色。
    pub fn body_color(&self) -> Color {
        self.appearance.body_color
    }

    /// 返回蛇头位置。如果身体为空，返回 (0, 0)。
    fn head(&self) -> Position {
        self.body.back().copied().unwrap_or(Position { x: 0, y: 0 })
    }
}

impl PlayerSnake {
    /// 创建玩家蛇。
    fn new(body: VecDeque<Position>, direction: Direction, appearance: SnakeAppearance) -> Self {
        Self {
            snake: Snake::new(body, direction, appearance),
            pending_direction: direction,
        }
    }

    /// 返回玩家当前已生效方向。
    pub fn direction(&self) -> Direction {
        self.snake.direction()
    }

    /// 返回玩家待生效方向。
    fn pending_direction(&self) -> Direction {
        self.pending_direction
    }

    /// 返回玩家蛇身坐标。
    pub fn body(&self) -> &VecDeque<Position> {
        self.snake.body()
    }

    /// 返回玩家当前得分。
    pub fn score(&self) -> u32 {
        self.snake.score()
    }

    /// 返回玩家蛇头符号。
    pub fn head_glyph(&self) -> &'static str {
        self.snake.head_glyph()
    }

    /// 返回玩家蛇身符号。
    pub fn body_glyph(&self) -> &'static str {
        self.snake.body_glyph()
    }

    /// 返回玩家蛇头颜色。
    pub fn head_color(&self) -> Color {
        self.snake.head_color()
    }

    /// 返回玩家蛇身颜色。
    pub fn body_color(&self) -> Color {
        self.snake.body_color()
    }

    /// 返回玩家蛇头位置。
    pub fn head(&self) -> Position {
        self.snake.head()
    }
}

impl EnemySnake {
    /// 创建一条新的 AI 蛇，初始随机漫步步数为 0。
    fn new(body: VecDeque<Position>, direction: Direction, appearance: SnakeAppearance) -> Self {
        Self {
            snake: Snake::new(body, direction, appearance),
            random_walk_steps: 0,
            random_walk_direction: None,
        }
    }

    /// 返回 AI 当前移动方向。
    pub fn direction(&self) -> Direction {
        self.snake.direction()
    }

    /// 返回 AI 蛇身坐标。
    pub fn body(&self) -> &VecDeque<Position> {
        self.snake.body()
    }

    /// 返回 AI 当前累计得分。
    pub fn score(&self) -> u32 {
        self.snake.score()
    }

    /// 返回 AI 蛇头符号。
    pub fn head_glyph(&self) -> &'static str {
        self.snake.head_glyph()
    }

    /// 返回 AI 蛇身符号。
    pub fn body_glyph(&self) -> &'static str {
        self.snake.body_glyph()
    }

    /// 返回 AI 蛇头颜色。
    pub fn head_color(&self) -> Color {
        self.snake.head_color()
    }

    /// 返回 AI 蛇身颜色。
    pub fn body_color(&self) -> Color {
        self.snake.body_color()
    }

    /// 返回 AI 蛇头位置。如果身体为空，返回 (0, 0)。
    fn head(&self) -> Position {
        self.snake.head()
    }
}

impl SnakeAppearance {
    /// 返回玩家蛇的外观配置。
    fn player() -> Self {
        Self {
            head_glyph: "@",
            body_glyph: "o",
            head_color: Color::White,
            body_color: Color::White,
        }
    }

    /// 按固定槽位返回 AI 的外观配置。
    fn for_slot(slot: usize) -> Self {
        match slot % 6 {
            0 => Self {
                head_glyph: "A",
                body_glyph: "a",
                head_color: Color::LightMagenta,
                body_color: Color::Magenta,
            },
            1 => Self {
                head_glyph: "B",
                body_glyph: "b",
                head_color: Color::LightCyan,
                body_color: Color::Cyan,
            },
            2 => Self {
                head_glyph: "C",
                body_glyph: "c",
                head_color: Color::LightYellow,
                body_color: Color::Yellow,
            },
            3 => Self {
                head_glyph: "D",
                body_glyph: "d",
                head_color: Color::LightRed,
                body_color: Color::Red,
            },
            4 => Self {
                head_glyph: "E",
                body_glyph: "e",
                head_color: Color::LightBlue,
                body_color: Color::Blue,
            },
            _ => Self {
                head_glyph: "F",
                body_glyph: "f",
                head_color: Color::White,
                body_color: Color::Gray,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct NavigationDecision {
    direction: Direction,
    random_walk_steps: u8,
    random_walk_direction: Option<Direction>,
}

#[derive(Debug, Clone, Copy)]
struct EnemyPlan {
    next_head: Position,
    eats_food: bool,
    navigation: NavigationDecision,
    crashes: bool,
}

/// 封装一局贪吃蛇的完整状态。
pub struct GameState {
    /// 棋盘宽度，单位为网格数。
    width: u16,
    /// 棋盘高度，单位为网格数。
    height: u16,
    /// 已推进的逻辑帧数。
    tick_count: u64,
    /// 游戏当前运行状态。
    state: RunState,
    /// 玩家蛇。
    player: PlayerSnake,
    /// 所有 AI 敌蛇。
    enemies: Vec<EnemySnake>,
    /// 当前棋盘上的所有食物位置。
    foods: Vec<Position>,
}

impl GameState {
    /// 创建一个默认尺寸（16x12）的游戏状态。
    ///
    /// 初始化后游戏处于 Ready 状态，需要玩家按键才开始。
    /// 默认生成 3 条 AI 蛇和 4 颗食物。
    pub fn new() -> Self {
        Self::with_board_size(16, 12)
    }

    /// 按指定棋盘尺寸初始化一局新游戏。
    pub fn with_board_size(width: u16, height: u16) -> Self {
        let player = PlayerSnake::new(
            Self::spawn_player_snake(width, height),
            Direction::Right,
            SnakeAppearance::player(),
        );

        let mut game = Self {
            width,
            height,
            tick_count: 0,
            state: RunState::Ready,
            player,
            enemies: Vec::with_capacity(AI_SNAKE_COUNT),
            foods: Vec::new(),
        };

        for slot in 0..AI_SNAKE_COUNT {
            let Some(enemy) = game.try_spawn_enemy_for_slot(slot) else {
                break;
            };
            game.enemies.push(enemy);
        }

        game.refill_foods();
        game
    }

    /// 进入运行状态，开始或继续推进游戏。
    pub fn start(&mut self) {
        if matches!(self.state, RunState::Ready | RunState::Paused) {
            self.state = RunState::Running;
        }
    }

    /// 推进一帧游戏逻辑，处理玩家、AI、食物和碰撞。
    ///
    /// 游戏 tick 是核心逻辑推进函数，每 160ms 被调用一次。
    /// 处理流程分为以下几个阶段：
    ///
    /// 1. **方向同步**：将 pending_direction（玩家输入）同步为实际生效的 direction
    ///
    /// 2. **玩家移动计算**：
    ///    - 根据当前位置和方向计算下一步位置
    ///    - 判断该位置是否有食物
    ///
    /// 3. **AI 移动规划**：
    ///    - 为每条 AI 敌蛇计算下一步的移动意图（方向、是否吃到食物等）
    ///    - 所有 AI 的规划在碰撞判断之前完成，确保公平性
    ///
    /// 4. **碰撞检测**：
    ///    - 先判断玩家是否会撞墙、撞自身或撞 AI
    ///    - 若玩家死亡，立即结束游戏
    ///    - 再判断每个 AI 是否会撞死（撞墙、撞玩家、撞其他AI）
    ///
    /// 5. **状态更新**：
    ///    - 玩家蛇前进一步，吃食物则增长，否则移除尾巴
    ///    - 各 AI 蛇按各自的 plan 前进一步
    ///    - 撞死的 AI 在同一帧重生到新位置
    ///    - 补充被吃掉的食物
    ///    - tick 计数器递增
    pub fn tick(&mut self) {
        // 检查游戏是否正在运行，非运行状态直接返回
        if self.state != RunState::Running {
            return;
        }

        // 将待生效的方向同步为实际生效的方向
        self.player.snake.direction = self.player.pending_direction();

        // 计算玩家下一步的位置，并判断是否会吃到食物
        let player_next = self.next_position(self.player_head(), self.player.direction());
        let player_eats = self.foods.contains(&player_next);

        // 为所有 AI 预规划下一步的移动（方向、是否吃食物等）
        let mut enemy_plans = Vec::with_capacity(self.enemies.len());
        for enemy_index in 0..self.enemies.len() {
            enemy_plans.push(self.plan_enemy_move(enemy_index));
        }

        // 检测玩家是否会碰撞死亡（撞墙、撞自身、撞AI、头碰头）
        let player_crashes = self.player_collides(player_next, player_eats, &enemy_plans);
        if player_crashes {
            self.state = RunState::GameOver;
            return;
        }

        // 玩家成功移动，吃食物则增长，否则移除尾巴
        self.advance_player(player_next, player_eats);

        // 检测每个 AI 是否会碰撞死亡
        let crash_flags = (0..enemy_plans.len())
            .map(|enemy_index| {
                self.enemy_collides(enemy_index, player_next, player_eats, &enemy_plans)
            })
            .collect::<Vec<_>>();

        // 将碰撞检测结果写回 AI 规划中
        for (plan, crashes) in enemy_plans.iter_mut().zip(crash_flags.into_iter()) {
            plan.crashes = crashes;
        }

        // 根据碰撞检测结果，更新每个 AI 的状态（移动或重生）
        for (enemy_index, plan) in enemy_plans.into_iter().enumerate() {
            if plan.crashes {
                self.respawn_enemy(enemy_index);
            } else {
                self.advance_enemy(enemy_index, plan);
            }
        }

        // 补充被吃掉的食物，保持食物数量达标
        self.refill_foods();

        // tick 计数器递增，记录游戏进行的时间
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
        if Self::is_opposite(self.player.direction(), direction) {
            return;
        }

        self.player.pending_direction = direction;
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
        self.player.score()
    }

    /// 返回 AI 数量。
    pub fn enemy_count(&self) -> usize {
        self.enemies.len()
    }

    /// 返回当前运行状态。
    pub fn run_state(&self) -> RunState {
        self.state
    }

    /// 返回玩家当前生效的移动方向。
    pub fn direction(&self) -> Direction {
        self.player.direction()
    }

    /// 返回玩家蛇身坐标队列，尾部在前，头部在后。
    pub fn snake(&self) -> &VecDeque<Position> {
        self.player.body()
    }

    /// 返回所有 AI 敌蛇。
    pub fn enemies(&self) -> &[EnemySnake] {
        &self.enemies
    }

    /// 返回玩家蛇。
    pub fn player(&self) -> &PlayerSnake {
        &self.player
    }

    /// 返回当前所有食物位置。
    pub fn foods(&self) -> &[Position] {
        &self.foods
    }

    /// 返回玩家蛇头位置。
    fn player_head(&self) -> Position {
        self.player.head()
    }

    /// 让玩家蛇前进一步，并处理吃食物后的增长。
    fn advance_player(&mut self, next_head: Position, eats_food: bool) {
        self.player.snake.body.push_back(next_head);
        if eats_food {
            self.player.snake.score += 1;
            self.remove_food(next_head);
        } else {
            self.player.snake.body.pop_front();
        }
    }

    /// 让指定 AI 前进一步，并处理吃食物后的增长。
    fn advance_enemy(&mut self, enemy_index: usize, plan: EnemyPlan) {
        let enemy = &mut self.enemies[enemy_index];
        enemy.snake.direction = plan.navigation.direction;
        enemy.random_walk_steps = plan.navigation.random_walk_steps;
        enemy.random_walk_direction = plan.navigation.random_walk_direction;
        enemy.snake.body.push_back(plan.next_head);

        if plan.eats_food {
            enemy.snake.score += 1;
            self.remove_food(plan.next_head);
        } else {
            enemy.snake.body.pop_front();
        }
    }

    /// 判断玩家下一步是否会导致游戏结束。
    ///
    /// 玩家死亡有四种可能：
    ///
    /// 1. **撞墙**：下一步位置超出棋盘边界
    ///    - 通过 `hit_wall(next_head)` 判断
    ///
    /// 2. **撞自身**：下一步撞到自己的身体
    ///    - 需要考虑"尾巴移动规则"：如果玩家要吃食物，尾巴不会移动，
    ///      此时尾巴所在位置仍被视为被身体占用
    ///    - 通过 `occupies_with_tail_rules(self.player.body(), next_head, player_eats)` 判断
    ///
    /// 3. **撞 AI**：下一步撞到任意一条 AI 蛇的身体
    ///    - 同样需要考虑 AI 是否会吃食物（尾巴是否移动）
    ///    - 通过遍历所有 AI 的身体判断
    ///
    /// 4. **被 AI 头撞**：AI 的下一步位置与玩家下一步位置相同（头碰头）
    ///    - AI 规划已经包含它们的下一步位置
    ///    - 通过 `enemy_plans.iter().any(|plan| plan.next_head == next_head)` 判断
    fn player_collides(
        &self,
        next_head: Position,
        player_eats: bool,
        enemy_plans: &[EnemyPlan],
    ) -> bool {
        self.hit_wall(next_head)
            || self.occupies_with_tail_rules(self.player.body(), next_head, player_eats)
            || self
                .enemies
                .iter()
                .zip(enemy_plans.iter())
                .any(|(enemy, plan)| {
                    self.occupies_with_tail_rules(enemy.body(), next_head, plan.eats_food)
                })
            || enemy_plans.iter().any(|plan| plan.next_head == next_head)
    }

    /// 判断指定 AI 下一步是否会撞死；撞死后会在同一帧重生。
    ///
    /// AI 死亡有六种可能：
    ///
    /// 1. **撞墙**：下一步位置超出棋盘边界
    ///
    /// 2. **撞自身**：下一步撞到自己的身体
    ///    - 需要考虑该 AI 自己是否会吃食物
    ///
    /// 3. **撞玩家**：下一步撞到玩家蛇的身体
    ///    - 需要考虑玩家是否会吃食物（玩家尾巴是否移动）
    ///
    /// 4. **撞其他 AI**：下一步撞到其他 AI 蛇的身体
    ///    - 需要考虑其他 AI 是否会吃食物
    ///    - 排除自己（other_index != enemy_index）
    ///
    /// 5. **被玩家头撞**：玩家的下一步位置与该 AI 的下一步位置相同
    ///    - 这是玩家"先发制人"的情况，玩家走在 AI 前面
    ///
    /// 6. **被其他 AI 头撞**：其他 AI 的下一步位置与该 AI 的下一步位置相同
    ///    - 两条 AI 蛇头碰头的情况
    ///    - 排除自己（other_index != enemy_index）
    fn enemy_collides(
        &self,
        enemy_index: usize,
        player_next: Position,
        player_eats: bool,
        enemy_plans: &[EnemyPlan],
    ) -> bool {
        let enemy = &self.enemies[enemy_index];
        let plan = enemy_plans[enemy_index];

        self.hit_wall(plan.next_head)
            || self.occupies_with_tail_rules(enemy.body(), plan.next_head, plan.eats_food)
            || self.occupies_with_tail_rules(self.player.body(), plan.next_head, player_eats)
            || self.enemies.iter().zip(enemy_plans.iter()).enumerate().any(
                |(other_index, (other_enemy, other_plan))| {
                    other_index != enemy_index
                        && self.occupies_with_tail_rules(
                            other_enemy.body(),
                            plan.next_head,
                            other_plan.eats_food,
                        )
                },
            )
            || plan.next_head == player_next
            || enemy_plans
                .iter()
                .enumerate()
                .any(|(other_index, other_plan)| {
                    other_index != enemy_index && other_plan.next_head == plan.next_head
                })
    }

    /// 为一条 AI 计算下一步移动意图。
    fn plan_enemy_move(&self, enemy_index: usize) -> EnemyPlan {
        let navigation = self.choose_enemy_direction(enemy_index);
        let enemy = &self.enemies[enemy_index];
        let next_head = self.next_position(enemy.head(), navigation.direction);
        let eats_food = self.foods.contains(&next_head);

        EnemyPlan {
            next_head,
            eats_food,
            navigation,
            crashes: false,
        }
    }

    /// 为 AI 敌蛇选择下一步的移动方向。
    ///
    /// AI 决策采用分层策略：
    ///
    /// **第一层：随机漫步（Random Walk）**
    /// - 当 `random_walk_steps > 0` 时，AI 正在执行随机漫步
    /// - 优先保持当前漫步方向（如果安全）
    /// - 否则随机选择一个新的安全漫步方向
    /// - 每走一步，漫步计数器递减
    ///
    /// **第二层：随机决定是否开始新漫步（15% 概率）**
    /// - 以 15% 概率触发随机漫步行为
    /// - 随机选择一个安全方向作为漫步方向
    /// - 漫步步数随机设为 5-15 步
    /// - 这是为了增加 AI 行为的不可预测性
    ///
    /// **第三层：追逐食物（Food Chase）**
    /// - 找到离 AI 最近的食物（曼哈顿距离）
    /// - 计算到达食物的优先方向列表（先水平后垂直，或反之）
    /// - 按优先级尝试每个方向，选择第一个安全的
    ///
    /// **第四层：保持当前方向**
    /// - 如果优先方向都不安全，尝试保持原方向（如果安全）
    ///
    /// **第五层：紧急逃生**
    /// - 如果所有前进方向都不安全，从所有安全方向中随机选一个
    /// - 排除会导致掉头的反向方向
    ///
    /// **兜底策略**
    /// - 如果仍然没有安全方向，保持原方向不变（可能下一秒就撞死）
    fn choose_enemy_direction(&self, enemy_index: usize) -> NavigationDecision {
        let enemy = &self.enemies[enemy_index];

        // 检查是否正在进行随机漫步
        if enemy.random_walk_steps > 0 {
            // 尝试保持当前漫步方向（如果安全）
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

            // 当前漫步方向不安全，重新选择随机漫步方向
            let walk_dir = self.random_walk_direction(enemy_index, enemy.direction());
            return NavigationDecision {
                direction: walk_dir,
                random_walk_steps: enemy.random_walk_steps.saturating_sub(1),
                random_walk_direction: Some(walk_dir),
            };
        }

        // 15% 概率开始新的随机漫步
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

        // 默认行为：追逐最近的食物
        let target = self.closest_food_to(enemy.head());
        let preferred = self.preferred_directions(enemy.head(), target);

        // 按优先级尝试每个方向，选择第一个安全的
        for direction in preferred {
            // 跳过会导致掉头的方向
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

        // 优先方向都不安全，尝试保持当前方向
        let next = self.next_position(enemy.head(), enemy.direction());
        if self.enemy_step_is_safe(enemy_index, next) {
            return NavigationDecision {
                direction: enemy.direction(),
                random_walk_steps: 0,
                random_walk_direction: None,
            };
        }

        // 所有前进方向都不安全，从所有安全方向中随机选一个
        let safe_dirs = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
        .into_iter()
        .filter(|&d| {
            !Self::is_opposite(enemy.direction(), d)
                && self.enemy_step_is_safe(enemy_index, self.next_position(enemy.head(), d))
        });

        if let Some(direction) = safe_dirs.into_iter().next() {
            return NavigationDecision {
                direction,
                random_walk_steps: 0,
                random_walk_direction: None,
            };
        }

        // 兜底：没有安全方向，保持原方向不变
        NavigationDecision {
            direction: enemy.direction(),
            random_walk_steps: 0,
            random_walk_direction: None,
        }
    }

    /// 为随机漫步选择一个安全的方向。
    ///
    /// 从四个方向中随机尝试，返回第一个安全的方向。
    /// 安全的定义是：不会撞墙、不会撞玩家、不会撞其他 AI。
    ///
    /// **选择顺序**：
    /// 1. 随机打乱四个方向的顺序
    /// 2. 遍历每个方向，排除掉头的反向方向
    /// 3. 检查该方向的下一个位置是否安全
    /// 4. 返回第一个安全方向
    ///
    /// **兜底逻辑**：
    /// - 如果所有方向都不安全，尝试保持当前方向
    /// - 如果当前方向也不安全，返回 Direction::Up（兜底）
    fn random_walk_direction(&self, enemy_index: usize, current_direction: Direction) -> Direction {
        let all = [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ];
        let mut rng = rand::rng();
        let mut safe_directions = Vec::new();

        // 随机尝试每个方向，将安全的收集起来
        for _ in 0..all.len() {
            // 随机选择一个方向
            let direction = all[rng.random_range(0..all.len())];

            // 跳过会导致掉头的方向
            if Self::is_opposite(current_direction, direction) {
                continue;
            }

            // 检查该方向是否安全
            let next = self.next_position(self.enemies[enemy_index].head(), direction);
            if self.enemy_step_is_safe(enemy_index, next) {
                safe_directions.push(direction);
            }
        }

        // 返回第一个收集到的安全方向
        if let Some(&direction) = safe_directions.first() {
            return direction;
        }

        // 所有方向都不安全，尝试保持当前方向
        let next = self.next_position(self.enemies[enemy_index].head(), current_direction);
        if self.enemy_step_is_safe(enemy_index, next) {
            return current_direction;
        }

        // 兜底：返回 Direction::Up
        Direction::Up
    }

    /// 判断 AI 的下一步位置是否安全（不会立即撞死）。
    ///
    /// 安全意味着下一步位置：
    /// 1. 不超出棋盘边界（不是撞墙）
    /// 2. 不与玩家蛇身重叠（不是被玩家吃掉）
    /// 3. 不与任何其他 AI 蛇身重叠（不是被其他 AI 吃掉）
    ///
    /// 注意：这里不检查该位置是否与食物重叠，因为吃食物是好事。
    /// 注意：不检查自己是否会吃食物（尾巴规则），因为吃食物后尾巴会扩展。
    fn enemy_step_is_safe(&self, enemy_index: usize, next: Position) -> bool {
        // 检查是否会撞墙
        if self.hit_wall(next) {
            return false;
        }

        // 检查是否撞到自己的尾巴（假设自己不会吃食物）
        if self.occupies_with_tail_rules(self.enemies[enemy_index].body(), next, false) {
            return false;
        }

        // 检查是否撞到玩家蛇身（假设玩家不会吃食物）
        if self.occupies_with_tail_rules(self.player.body(), next, false) {
            return false;
        }

        // 检查是否撞到其他 AI 蛇身（假设其他 AI 不会吃食物）
        !self.enemies.iter().enumerate().any(|(other_index, enemy)| {
            other_index != enemy_index && self.occupies_with_tail_rules(enemy.body(), next, false)
        })
    }

    /// 让 AI 重生到远离玩家的位置，避免卡死后整局无法继续。
    fn respawn_enemy(&mut self, enemy_index: usize) {
        let score = self.enemies[enemy_index].snake.score;

        if let Some(replacement) = self.try_spawn_enemy_for_slot(enemy_index) {
            self.enemies[enemy_index] = replacement;
            self.enemies[enemy_index].snake.score = score;
        } else {
            self.enemies[enemy_index].snake.score = score;
        }
    }

    /// 尝试在指定 slot 位置生成一条 AI 蛇。
    ///
    /// 这个函数用于初始化时生成多条 AI。它会尝试把 AI 放置在
    /// 远离玩家的位置，实现分散spawn的效果。
    ///
    /// **算法步骤**：
    /// 1. 棋盘尺寸过小时直接返回 None
    /// 2. 按与玩家所在行的距离对所有行排序，距离远的优先
    /// 3. 对排序后的每行，从右到左尝试放置水平蛇身
    /// 4. 检查放置位置是否有效（不与玩家、食物、其他 AI 重叠）
    /// 5. 如果都没成功，fallback 到 `try_spawn_enemy` 随机生成
    fn try_spawn_enemy_for_slot(&self, slot: usize) -> Option<EnemySnake> {
        // 棋盘太小无法放置 AI 蛇，直接返回 None
        if self.width < 3 && self.height < 3 {
            return None;
        }

        // 获取玩家所在的行
        let player_row = self.player_head().y;

        // 生成所有行的列表，并按与玩家距离排序（距离远的优先）
        let mut rows = (0..self.height).collect::<Vec<_>>();
        rows.sort_by_key(|row| row.abs_diff(player_row));
        rows.reverse();

        // 通过轮转实现多个 slot 之间的分散
        let row_count = rows.len();
        if row_count > 0 {
            rows.rotate_left(slot % row_count);
        }

        // 遍历每行，从右到左尝试放置水平蛇身
        for y in rows {
            for head_x in (0..=self.width.saturating_sub(3)).rev() {
                // 创建一条水平放置的敌蛇，头部朝左
                let enemy = EnemySnake::new(
                    Self::horizontal_enemy_body(head_x, y),
                    Direction::Left,
                    SnakeAppearance::for_slot(slot),
                );

                // 检查放置位置是否有效
                if self.enemy_spawn_is_valid(enemy.body()) {
                    return Some(enemy);
                }
            }
        }

        // 所有预定位置都无效，fallback 到随机生成
        self.try_spawn_enemy(slot)
    }

    /// 随机尝试生成一条 AI 蛇，最多尝试 256 次。
    ///
    /// 每次随机生成一个水平和垂直的蛇身布局，检查位置是否有效。
    /// 如果棋盘太小（小于 3x3），直接返回 None。
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
    /// 有效意味着蛇身不与以下任何元素重叠：
    /// 1. 玩家蛇身的任何一段
    /// 2. 任何食物位置
    /// 3. 任何其他 AI 蛇身的任何一段
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

        !self
            .enemies
            .iter()
            .any(|enemy| enemy.body().iter().any(|segment| body.contains(segment)))
    }

    /// 返回离指定坐标最近的一颗食物。
    fn closest_food_to(&self, origin: Position) -> Position {
        self.foods
            .iter()
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
    ///
    /// 这个函数处理贪吃蛇游戏中的一个关键细节：**尾巴移动规则**。
    ///
    /// 正常情况下，蛇移动时尾巴会向前移动一格。但如果蛇刚吃了食物，
    /// 尾巴不会移动（因为食物位置变成了新的尾巴），蛇身长度因此增加一格。
    ///
    /// **判断逻辑**：
    /// - `index == 0` 的 segment 是尾巴
    /// - 如果 `grows == true`（蛇要吃食物），尾巴不会移动，
    ///   此时尾巴位置仍被视为被身体占用（因为它下一秒还在那里）
    /// - 如果 `grows == false`（蛇不吃食物），尾巴会移动走，
    ///   此时尾巴位置不被视为被占用（下一秒那里是空的）
    ///
    /// **返回值**：如果指定位置被蛇身占用（考虑上述尾巴规则），返回 true
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
        while self.foods.len() < self.target_food_count() {
            let food = self.random_empty_position();
            self.foods.push(food);
        }
    }

    /// 计算目标食物数量。
    ///
    /// 目标食物数量 = min(FOOD_COUNT, 棋盘剩余空格数)
    ///
    /// 其中"棋盘剩余空格数" = 棋盘总面积 - 玩家蛇身长度 - 所有 AI 蛇身长度之和。
    /// 这个限制确保食物不会生成在蛇身上。
    fn target_food_count(&self) -> usize {
        let occupied = self.player.body().len()
            + self
                .enemies
                .iter()
                .map(|enemy| enemy.body().len())
                .sum::<usize>();
        let area = self.width as usize * self.height as usize;
        FOOD_COUNT.min(area.saturating_sub(occupied))
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

            if !self.player.body().contains(&candidate)
                && !self.foods.contains(&candidate)
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

    /// 随机生成一条长度为 3 的敌蛇及其初始方向。
    fn spawn_enemy_snake(width: u16, height: u16) -> (VecDeque<Position>, Direction) {
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
    ///
    /// 生成一条长度为 3 的水平蛇身：
    /// - head_x: 蛇头 x 坐标
    /// - y: 蛇身所在的 y 坐标
    /// - 蛇身从左到右：tail=(head_x, y), middle=(head_x+1, y), head=(head_x+2, y)
    fn horizontal_enemy_body(head_x: u16, y: u16) -> VecDeque<Position> {
        let mut snake = VecDeque::new();
        snake.push_back(Position { x: head_x + 2, y });
        snake.push_back(Position { x: head_x + 1, y });
        snake.push_back(Position { x: head_x, y });
        snake
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
    /// 验证初始敌蛇数量正确，并且都与玩家分离。
    fn enemy_snakes_start_separate_from_player() {
        let game = GameState::with_board_size(20, 10);

        assert_eq!(game.enemy_count(), 3);
        assert!(
            game.enemies()
                .iter()
                .flat_map(|enemy| enemy.body().iter())
                .all(|segment| !game.snake().contains(segment))
        );
    }

    #[test]
    /// 验证初始敌蛇之间也不会互相重叠。
    fn enemy_snakes_start_separate_from_each_other() {
        let game = GameState::with_board_size(20, 10);

        for (index, enemy) in game.enemies().iter().enumerate() {
            for other in game.enemies().iter().skip(index + 1) {
                assert!(
                    enemy
                        .body()
                        .iter()
                        .all(|segment| !other.body().contains(segment))
                );
            }
        }
    }
}
