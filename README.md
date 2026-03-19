# rust-snake

一个基于 Rust、`ratatui` 和 `crossterm` 的终端贪吃蛇项目。

当前仓库已经具备一个可运行的原型版本，支持基础贪吃蛇玩法、终端自适应棋盘，以及窗口调整后的自动重开。

## 当前功能

- 终端 TUI 界面
- 固定 tick 游戏循环
- `WASD` 和方向键控制
- `Space` 暂停
- `r` 重开
- `q` 退出
- 吃到食物后增长
- 撞墙或撞到自己后结束
- 按当前窗口大小初始化棋盘
- 调整窗口后按新尺寸自动开始新游戏
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

- `W` / `A` / `S` / `D`：移动
- 方向键：移动
- `Space`：暂停或继续
- `r`：重新开始
- `q`：退出游戏

## 当前实现说明

当前版本已经完成了第一版核心玩法，但仍然偏向原型：

- 棋盘目前使用字符渲染
- 还没有单独的开始页
- 游戏结束提示还可以更明显
- 终端过小场景还没有单独处理

## 后续计划

- 优化棋盘视觉效果
- 增加开始页和结束页提示
- 增加窗口过小时的提示
- 补充更多测试用例

## 开发状态

当前状态：可运行原型
