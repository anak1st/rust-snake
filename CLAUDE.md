# CLAUDE.md

此文件为 Claude Code（claude.ai/code）在本仓库中进行代码开发时提供指导。

## 项目概览

这是一个基于终端的贪吃蛇游戏，使用 Rust 编写，并通过 ratatui 库进行 TUI 渲染。

## 常用命令

```bash
# 运行游戏
cargo run

# 运行测试
cargo test

# 构建发布版本
cargo build --release
```

## 架构

项目由四个主要模块组成：

- [src/main.rs](src/main.rs) - 程序入口，负责初始化 App 并启动游戏循环
- [src/app.rs](src/app.rs) - 应用生命周期管理、终端初始化、事件处理，以及固定 tick 频率（160ms）的主游戏循环
- [src/game.rs](src/game.rs) - 核心游戏逻辑：`GameState` 结构体管理蛇、食物、分数、方向与碰撞检测
- [src/render.rs](src/render.rs) - 使用 ratatui 组件进行 UI 渲染，绘制标题区、状态面板、游戏棋盘和底部信息

### 关键数据结构

- `GameState`（位于 [game.rs](src/game.rs)）- 包含蛇（以 `VecDeque<Position>` 表示）、食物位置、棋盘尺寸、分数和运行状态
- `RunState` - 枚举类型，包含 `Running`、`Paused`、`GameOver` 三种状态
- `Direction` - 枚举类型，包含 `Up`、`Down`、`Left`、`Right` 四个方向

### 依赖项

- `ratatui` - 终端 UI 框架
- `crossterm` - 终端控制与事件处理
- `rand` - 随机生成食物位置
- `anyhow` - 错误处理

## 操作说明

- `W/A/S/D` 或方向键 - 控制蛇移动
- `Space` - 暂停/继续
- `r` - 重新开始游戏
- `q` - 退出游戏

当终端窗口大小发生变化时，游戏会自动重新开始。
