/// 应用层配置。
pub mod app {
    /// 控制游戏逻辑推进频率，决定蛇移动速度。
    pub const TICK_RATE_MS: u64 = 160;
    /// 控制界面渲染频率，为后续动画和过渡效果预留更细的时间粒度。
    pub const RENDER_FRAME_RATE_MS: u64 = 33;
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
    /// AI 对非撞墙风险的规避概率，`100` 表示总会躲避，`0` 表示完全不躲。
    pub const AI_NON_WALL_AVOIDANCE_CHANCE_PERCENT: u8 = 65;
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
