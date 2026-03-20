# AGENTS.md

本文件为在此仓库中操作的 agent 提供指导。

## 项目概述

使用 ratatui 库实现的支持多 AI 敌蛇的终端贪吃蛇游戏。

## 构建/检查/测试命令

```bash
# 运行游戏
cargo run

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
- 使用 Rust 2024 版本（参见 Cargo.toml `edition = "2024"`）
- 使用 `cargo fmt` 保持一致的代码格式
- 使用 `cargo clippy` 捕获常见错误
- 优先使用早期返回和清晰的控制流，避免深层嵌套结构

### 导入顺序
- 按外部 crate、std、crate 的顺序分组导入
- 内部模块使用 `crate::` 绝对路径
- 示例（app.rs）：
```rust
use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use crate::game::{Direction, GameState, RunState};
use crate::render::{self, board_size_for_terminal, is_terminal_too_small};
```

### 类型和结构体
- 位置坐标和棋盘尺寸使用 `u16`
- 分数和计数器使用 `u32`
- tick 计数使用 `u64`
- 集合索引和长度使用 `usize`
- 蛇身使用 `VecDeque<Position>`（尾巴在前，头部在后）
- 可空值使用 `Option<T>`，优先使用 `.copied()` 或 `.cloned()` 而不是 `.unwrap_or()`
- 枚举匹配使用 `matches!()` 宏

### 命名规范
- **类型/结构体**：`PascalCase`（如 `GameState`、`EnemySnake`、`RunState`）
- **枚举成员**：`PascalCase`（如 `RunState::Running`、`Direction::Up`）
- **函数/方法**：`snake_case`（如 `set_direction`、`tick_count`）
- **常量**：`SCREAMING_SNAKE_CASE`（如 `TICK_RATE`、`MIN_BOARD_WIDTH`）
- **字段/变量**：`snake_case`（如 `tick_count`、`should_quit`）
- **布尔变量**：使用 `is_`、`has_`、`should_` 前缀（如 `is_running`、`should_quit`）
- **类型别名**：如果名称冲突，后缀 `Direction`（参见 render.rs 的 `Direction` 导入别名）

### 错误处理
- 应用层错误处理使用 `anyhow::Result<T>`
- 使用 `?` 操作符传播错误
- 可能失败的函数返回 `Result<...>` 并记录失败情况
- 使用 `anyhow::bail!()` 进行早期错误返回

### 枚举
- 简单枚举使用 `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
- 包含非 Copy 成员的枚举使用 `#[derive(Debug, Clone)]`
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
- 非显而易见的行为需要详细文档（参见 game.rs tick() 函数）
- 记录游戏逻辑的边缘情况（参见 game.rs 尾巴移动规则）

### 测试
- 测试位于源文件内的 `#[cfg(test)]` 模块中
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
- `main.rs` - 程序入口，初始化 App，处理 `-nocolor` 参数
- `app.rs` - 应用生命周期、终端设置、事件处理、主循环（160ms tick）
- `game.rs` - 核心游戏逻辑：GameState、蛇移动、AI、碰撞检测
- `render.rs` - 使用 ratatui 组件的 TUI 渲染
- `config.rs` - 配置常量（tick 速率 160ms、各类物品数量等）

### 关键模式
- **Tick 处理流程**：方向同步 → 玩家移动计算 → AI 移动规划 → 碰撞检测 → 状态更新
- **AI 决策分层**：随机漫步(15%) → 追逐食物 → 保方向 → 紧急逃生 → 兜底
- **尾巴移动规则**：吃食物时尾巴不移动（蛇身增长），用 `occupies_with_tail_rules` 判断
- **头撞头规则**：体型小死亡，同体型同死
- **蛇身存储**：`VecDeque<Position>`，索引 0 为尾巴，最后一个为头部
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
- 测试模块统一放在文件末尾的 `#[cfg(test)]` 模块中
- 配置常量按用途分组到 `config.rs` 的不同子模块中
