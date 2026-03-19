# rust-snake

一个基于 Rust、`ratatui` 和 `crossterm` 的终端贪吃蛇项目，支持双蛇对战。

## 游戏截图
![](docs/screenshot.png)

## 当前功能

- 终端 TUI 界面
- 固定 tick 游戏循环（160ms）
- `WASD` 和方向键控制玩家移动
- `Enter` / `Space` / 方向键开始游戏
- `Space` 暂停/继续
- `r` 重新开始
- `q` 退出
- 玩家蛇(@)吃到食物后增长
- AI 敌蛇(X)会自动追逐食物
- 敌蛇撞死后会重生，避免游戏无法继续
- 玩家撞墙、撞自己、或撞到敌蛇后游戏结束
- 玩家与敌蛇分数显示
- 按当前窗口大小初始化棋盘
- 调整窗口后按新尺寸自动开始新游戏
- 终端窗口过小时显示提示
- 支持 `-nocolor` 启动无色版本

## 技术栈

- [Rust](https://www.rust-lang.org/)
- [ratatui](https://github.com/ratatui/ratatui)
- [crossterm](https://github.com/crossterm-rs/crossterm)
- [rand](https://crates.io/crates/rand)
- [anyhow](https://crates.io/crates/anyhow)

## 项目结构

```text
src/
  main.rs     # 程序入口
  app.rs      # 终端生命周期、事件循环、输入处理
  game.rs     # 游戏状态与核心规则
  render.rs   # UI 布局与棋盘渲染
```

## 运行方式

确保本地已经安装 Rust 工具链，然后在项目根目录执行：

```bash
cargo run
```

启动无色版本：

```bash
cargo run -- -nocolor
```

## 测试

运行单元测试：

```bash
cargo test
```

## 操作说明

- `W` / `A` / `S` / `D` 或方向键：移动
- `Enter` / `Space` / 方向键：开始游戏
- `Space`：暂停或继续
- `r`：重新开始
- `q`：退出游戏

## 游戏说明

- 玩家蛇(@)与 AI 敌蛇(X)同屏对战
- 双方都能吃食物(*)增长，食物同时存在 4 颗
- 玩家撞墙、撞自己、或撞到敌蛇则游戏结束
- 敌蛇撞死后会立即重生，避免游戏无法继续
- 调整窗口大小会自动按新尺寸重新开始

## 后续计划

- 优化棋盘视觉效果
- 增加难度选择
- 补充更多测试用例

## 开发状态

当前状态：功能完整的双蛇对战版本
