/// 应用层配置。
pub mod app {
    /// 控制游戏逻辑推进频率，决定蛇移动速度。
    pub const TICK_RATE_MS: u64 = 160;
    /// `--test-ai` 模式下使用的更快逻辑推进频率，便于观察 AI 对战。
    pub const TEST_AI_TICK_RATE_MS: u64 = 32;
    /// 控制界面渲染频率，为后续动画和过渡效果预留更细的时间粒度。
    pub const RENDER_FRAME_RATE_MS: u64 = 16;
    /// 死亡回放最多保留的历史逻辑帧数。
    pub const DEATH_REPLAY_FRAME_COUNT: usize = 10;
}

/// 游戏玩法相关配置。
pub mod game {
    /// 默认棋盘宽度。
    pub const DEFAULT_BOARD_WIDTH: u16 = 16;
    /// 默认棋盘高度。
    pub const DEFAULT_BOARD_HEIGHT: u16 = 12;
    /// 默认生成的 AI 敌蛇数量。
    pub const AI_SNAKE_COUNT: usize = 4;
    /// 默认同时生成的食物数量。
    pub const FOOD_COUNT: usize = 4;
    /// 默认同时生成的超级食物数量。
    pub const SUPER_FOOD_COUNT: usize = 1;
    /// 默认同时生成的炸弹数量。
    pub const BOMB_COUNT: usize = 2;
    /// 普通食物提供的增长节数。
    pub const FOOD_GROWTH_AMOUNT: u16 = 1;
    /// 普通食物提供的分数。
    pub const FOOD_SCORE_GAIN: u32 = 1;
    /// 超级食物提供的增长节数。
    pub const SUPER_FOOD_GROWTH_AMOUNT: u16 = 4;
    /// 超级食物提供的分数。
    pub const SUPER_FOOD_SCORE_GAIN: u32 = 4;
    /// 单个尸块每隔多少个逻辑 tick 腐化成食物。
    pub const CORPSE_DECAY_INTERVAL_TICKS: u64 = 2;
    /// AI 触发随机漫游的概率，`5` 表示每次决策有 5% 概率进入随机漫游。
    pub const AI_RANDOM_WALK_CHANCE_PERCENT: u8 = 4;
    /// AI 随机漫游持续的最少步数。
    pub const AI_RANDOM_WALK_MIN_STEPS: u8 = 4;
    /// AI 随机漫游持续的最多步数。
    pub const AI_RANDOM_WALK_MAX_STEPS: u8 = 8;
    /// AI 进入某片区域前，要求该区域至少比自身预测长度额外大出的格子数。
    pub const AI_REGION_SIZE_MARGIN: usize = 2;
}

/// 渲染布局相关配置。
pub mod render {
    /// 顶部标题栏的固定高度。
    pub const HEADER_HEIGHT: u16 = 3;
    /// 底部帮助栏的固定高度。
    pub const FOOTER_HEIGHT: u16 = 3;
    /// 状态信息区域的固定高度。
    pub const INFO_HEIGHT: u16 = 4;
    /// 允许的最小棋盘宽度，避免窗口过小时不可玩。
    pub const MIN_BOARD_WIDTH: u16 = 10;
    /// 允许的最小棋盘高度，避免窗口过小时不可玩。
    pub const MIN_BOARD_HEIGHT: u16 = 6;
}
