# AGENTS.md

本文件为在此仓库中操作的 agent 提供指导。

## 项目概述

使用 `ratatui` 库实现的终端贪吃蛇游戏，支持多条 AI 敌蛇同场对战、尸体逐段腐化、物品脉冲和基础闪光动画。

## 构建/检查/测试命令

```bash
# 运行游戏
cargo run

# 启动无色版本
cargo run -- --no-color

# 让玩家蛇也由 AI 控制
cargo run -- --test-ai

# 查看帮助
cargo run -- --help

# 运行所有测试
cargo test

# 运行单个测试
cargo test test_name

# 构建发布版本
cargo build --release

# 检查代码格式
cargo fmt --check

# 自动格式化代码
cargo fmt

# 运行 clippy（代码检查）
cargo clippy

# 运行 clippy 并将所有警告视为错误
cargo clippy -- -D warnings
```

## 代码风格指南

### 通用原则

- 使用 Rust 2024 版本（参见 `Cargo.toml` 的 `edition = "2024"`）
- 使用 `cargo fmt` 保持一致的代码格式
- 使用 `cargo clippy` 捕获常见错误
- 优先使用早期返回和清晰的控制流，避免深层嵌套结构

### 导入顺序

- 按外部 crate、std、crate 的顺序分组导入
- 内部模块使用 `crate::` 绝对路径
- 示例（`app.rs`）：

```rust
use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;

use crate::config::app::{RENDER_FRAME_RATE_MS, TICK_RATE_MS};
use crate::game::{Direction, GameState, RunState};
use crate::render::{self, board_size_for_terminal, is_terminal_too_small};
```

### 类型和结构体

- 位置坐标和棋盘尺寸使用 `u16`
- 分数和计数器使用 `u32`
- tick 计数使用 `u64`
- 集合索引和长度使用 `usize`
- 蛇身使用 `VecDeque<Position>`（尾巴在前，头部在后）
- 可空值使用 `Option<T>`，优先使用 `.copied()` 或 `.cloned()`
- 枚举匹配使用 `matches!()` 宏

### 命名规范

- **类型/结构体**：`PascalCase`（如 `GameState`、`Snake`、`RunState`）
- **枚举成员**：`PascalCase`（如 `RunState::Running`、`Direction::Up`）
- **函数/方法**：`snake_case`（如 `set_direction`、`tick_count`）
- **常量**：`SCREAMING_SNAKE_CASE`（如 `TICK_RATE_MS`、`MIN_BOARD_WIDTH`）
- **字段/变量**：`snake_case`（如 `tick_count`、`should_quit`）
- **布尔变量**：使用 `is_`、`has_`、`should_` 前缀（如 `is_running`、`should_quit`）
- **类型别名**：若为避免命名冲突，优先使用语义化别名（参见 `panels.rs` 中的 `Direction as SnakeDirection`）

### 错误处理

- 应用层错误处理使用 `anyhow::Result<T>`
- 使用 `?` 操作符传播错误
- 可能失败的函数返回 `Result<...>`
- 使用 `anyhow::bail!()` 进行早期错误返回

### 枚举

- 简单枚举使用 `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
- 包含非 `Copy` 成员的枚举使用 `#[derive(Debug, Clone)]`
- 为每个枚举成员编写文档注释
- 示例：

```rust
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
```

### 文档

- 公共 API 文档使用中文 doc 注释（`///`）
- 复杂算法在函数体或文档注释中说明
- 非显而易见的行为需要详细文档
  参见 `src/game/logic.rs` 中 `tick()` 的阶段说明
- 记录游戏逻辑的边缘情况
  参见 `src/game/logic.rs` 中的尾巴移动规则、尸块腐化与头撞头结算

### 测试

- 测试位于源文件内的 `#[cfg(test)]` 模块或 `src/game/tests.rs`
- 使用描述性的测试函数名，并用文档注释说明验证内容
- 使用 `with_board_size(width, height)` 构造器创建测试游戏
- 测试示例：

```rust
#[cfg(test)]
mod tests {
    use super::{Direction, GameState, RunState};

    #[test]
    /// 验证每次 tick 都会让玩家蛇头向前推进一格。
    fn snake_moves_forward_on_tick() {
        let mut game = GameState::with_board_size(18, 8);
        game.start();
        // ...
    }
}
```

### 模块结构

- `src/main.rs` - 程序入口，解析 `--no-color`、`--test-ai`、`--help`
- `src/app.rs` - 应用生命周期、终端设置、事件处理、160ms 逻辑 tick 与 16ms 渲染调度、死亡回放
- `src/config.rs` - 应用、玩法、渲染配置常量（含 `TEST_AI_TICK_RATE_MS=32`、`DEATH_REPLAY_FRAME_COUNT=10`）
- `src/game/mod.rs` - `GameState`、共享类型与对外接口
- `src/game/logic.rs` - tick 结算、碰撞、物品消费、尸体腐化、延迟重生
- `src/game/ai.rs` - AI 路径选择与安全性评估
- `src/game/spawn.rs` - 玩家/敌蛇出生与敌蛇重生
- `src/game/snake.rs` - 蛇实体、控制模式、外观与增长逻辑
- `src/game/corpse.rs` - 尸块与渲染数据
- `src/game/tests.rs` - 核心规则回归测试
- `src/render/mod.rs` - 渲染入口与终端尺寸辅助
- `src/render/board.rs` - 棋盘格子绘制
- `src/render/panels.rs` - 标题栏、状态栏、帮助栏、弹窗
- `src/render/animation.rs` - 物品脉冲和闪光动画状态
- `src/render/style.rs` - 通用样式

### 关键模式

- **Tick 处理流程**：尸块推进 → 控制解析 → 玩家与 AI 移动规划 → 碰撞检测 → 头撞头结算 → 状态更新 → 物品补充
- **AI 决策分层**：继续随机漫步 → 以 `4%` 概率开始新漫步（`4-8` 步）→ 追逐最近可吃物 → 保方向 → 紧急逃生 → 兜底
- **尾巴移动规则**：吃食物或仍有 `pending_growth` 时尾巴不移动，占用判断必须考虑该规则
- **尸体流转规则**：蛇死亡后拆成多个 `CorpsePiece`，每隔 `2` 个 tick 逐段腐化为 `legacy_food`
- **敌蛇重生规则**：对应批次尸块全部腐化后才尝试重生，重生分数会清零
- **头撞头规则**：体型小死亡，同体型同死
- **死亡回放规则**：玩家死亡后保留最近 `10` 帧历史，可通过 `A`/`D` 或方向键浏览，`Esc`/`Space` 退出
- **蛇身存储**：`VecDeque<Position>`，索引 `0` 为尾巴，最后一个为头部
- **边界安全**：使用 `saturating_add/sub` 防止越界
- **枚举匹配**：使用 `matches!()` 宏而非 `==`

### 依赖

- `ratatui` - 终端 UI 框架
- `crossterm` - 终端控制和事件处理
- `rand` - 随机数生成（使用 `rand::rng()` 获取随机数生成器）
- `anyhow` - 错误处理

### 代码组织

- 每个源文件顶部的文档注释（`//!`）描述该模块的目的
- 私有辅助函数放在实现块之后或文件末尾
- 测试模块统一放在文件末尾的 `#[cfg(test)]` 模块或 `src/game/tests.rs`
- 配置常量按用途分组到 `config.rs` 的不同子模块中
