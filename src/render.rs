use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::config::render::{
    FOOTER_HEIGHT, HEADER_HEIGHT, INFO_HEIGHT, MIN_BOARD_HEIGHT, MIN_BOARD_WIDTH,
};
use crate::game::{Direction as SnakeDirection, EnemySnake, GameState, Position, RunState};
/// 普通文字的基础颜色。
const TEXT_COLOR: Color = Color::White;
/// 次要信息的弱化颜色。
const MUTED_COLOR: Color = Color::DarkGray;
/// 食物的强调颜色。
const FOOD_COLOR: Color = Color::Green;
/// 超级果实的强调颜色。
const SUPER_FRUIT_COLOR: Color = Color::LightYellow;
/// 炸弹的危险颜色。
const BOMB_COLOR: Color = Color::Red;
/// 主界面统一边框颜色。
const MAIN_BORDER_COLOR: Color = Color::White;

/// 根据当前游戏状态绘制整个界面。
///
/// 界面布局采用垂直三段式结构：
/// ```
/// ┌─────────────────────────────────────┐
/// │            Header (3行)              │  <- 顶部标题"Rust Snake"
/// ├─────────────────────────────────────┤
/// │           Status (4行)               │  <- 显示 Tick、分数、AI分数、数量、状态、方向
/// ├─────────────────────────────────────┤
/// │                                     │
/// │           Board (自适应)             │  <- 游戏棋盘 + 可能的弹窗提示
/// │                                     │
/// ├─────────────────────────────────────┤
/// │            Footer (3行)              │  <- 底部操作提示
/// └─────────────────────────────────────┘
/// ```
///
/// 颜色支持：
/// - `no_color = true` 时，所有元素使用默认灰度显示，适合不支持彩色的终端
/// - `no_color = false` 时，使用预定义的颜色方案区分不同元素
///
/// 状态相关的弹窗：
/// - Ready: 显示"按 Enter、Space 或方向键开始"提示
/// - Paused: 显示"游戏已暂停"提示
/// - GameOver: 显示"游戏结束"提示
/// - Running: 不显示弹窗，直接渲染棋盘内容
pub fn draw(frame: &mut Frame, game: &GameState, window_too_small: bool, no_color: bool) {
    // 终端窗口过小时，显示专用提示界面
    if window_too_small {
        draw_too_small(frame, no_color);
        return;
    }

    // 将终端区域划分为三部分：顶部标题区、中间内容区、底部帮助区
    let [header_area, body_area, footer_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Min(8),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .areas(frame.area());

    // 将中间内容区进一步划分为状态信息区和棋盘区
    let [info_area, board_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(INFO_HEIGHT), Constraint::Min(3)])
        .areas(body_area);

    // 创建顶部标题组件
    let header = Paragraph::new(Line::from("Rust Snake"))
        .alignment(Alignment::Center)
        .style(style_with_color(Color::LightCyan, no_color).add_modifier(Modifier::BOLD))
        .block(styled_block("Title", MAIN_BORDER_COLOR, no_color));

    // 根据游戏状态确定状态文本颜色
    let status_text = match game.run_state() {
        RunState::Ready => Span::styled("Ready", style_with_color(Color::Cyan, no_color)),
        RunState::Running => Span::styled("Running", style_with_color(Color::Green, no_color)),
        RunState::Paused => Span::styled("Paused", style_with_color(Color::Yellow, no_color)),
        RunState::GameOver => Span::styled("Game Over", style_with_color(Color::Red, no_color)),
    };

    // 将玩家方向转换为文本
    let direction_text = match game.direction() {
        SnakeDirection::Up => "Up",
        SnakeDirection::Down => "Down",
        SnakeDirection::Left => "Left",
        SnakeDirection::Right => "Right",
    };

    // 格式化所有 AI 的方向显示
    let ai_direction_spans = format_enemy_directions(game.enemies(), no_color);

    // 格式化所有 AI 的分数显示
    let ai_score_spans = format_enemy_scores(game.enemies(), no_color);

    // 创建状态信息面板，显示 Tick 数、分数、AI 分数、AI 数量、状态、方向等
    let mut info_rows = vec![
        vec![
            Span::styled("Tick: ", style_with_color(MUTED_COLOR, no_color)),
            Span::styled(
                game.tick_count().to_string(),
                style_with_color(TEXT_COLOR, no_color).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Score: ", style_with_color(MUTED_COLOR, no_color)),
            Span::styled(
                game.score().to_string(),
                style_with_color(Color::LightYellow, no_color).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Enemy Count: ", style_with_color(MUTED_COLOR, no_color)),
            Span::styled(
                game.enemy_count().to_string(),
                style_with_color(Color::LightCyan, no_color).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Enemy Score: ", style_with_color(MUTED_COLOR, no_color)),
        ],
        vec![
            Span::styled("State: ", style_with_color(MUTED_COLOR, no_color)),
            status_text,
            Span::raw("  "),
            Span::styled("Dir: ", style_with_color(MUTED_COLOR, no_color)),
            Span::styled(
                direction_text,
                style_with_color(Color::LightBlue, no_color).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("AI Dir: ", style_with_color(MUTED_COLOR, no_color)),
        ],
    ];
    info_rows[0].extend(ai_score_spans);
    info_rows[1].extend(ai_direction_spans);

    let info = Paragraph::new(info_rows.into_iter().map(Line::from).collect::<Vec<_>>())
        .block(styled_block("Status", MAIN_BORDER_COLOR, no_color));

    // 创建底部帮助栏，显示当前状态对应的操作提示
    let footer = Paragraph::new(Line::from(help_text(game.run_state())))
        .alignment(Alignment::Center)
        .style(style_with_color(MUTED_COLOR, no_color))
        .block(styled_block("Help", MAIN_BORDER_COLOR, no_color));

    // 将所有组件渲染到对应的区域
    frame.render_widget(header, header_area);
    frame.render_widget(info, info_area);
    draw_board(frame, board_area, game, no_color);
    frame.render_widget(footer, footer_area);
}

/// 判断当前终端尺寸是否已经小到无法稳定显示主界面。
pub fn is_terminal_too_small(width: u16, height: u16) -> bool {
    width < min_terminal_width() || height < min_terminal_height()
}

/// 根据终端尺寸估算可用棋盘大小，并保留最小可玩尺寸。
pub fn board_size_for_terminal(width: u16, height: u16) -> (u16, u16) {
    let board_width = width.saturating_sub(2).max(MIN_BOARD_WIDTH);
    let board_height = height
        .saturating_sub(HEADER_HEIGHT + FOOTER_HEIGHT + INFO_HEIGHT + 2)
        .max(MIN_BOARD_HEIGHT);

    (board_width, board_height)
}

/// 返回主界面正常显示所需的最小终端宽度。
fn min_terminal_width() -> u16 {
    MIN_BOARD_WIDTH + 2
}

/// 返回主界面正常显示所需的最小终端高度。
fn min_terminal_height() -> u16 {
    HEADER_HEIGHT + FOOTER_HEIGHT + INFO_HEIGHT + MIN_BOARD_HEIGHT + 2
}

/// 按当前状态绘制棋盘区域，提示页使用居中的内容块。
fn draw_board(frame: &mut Frame, area: ratatui::layout::Rect, game: &GameState, no_color: bool) {
    let board = Paragraph::new(render_live_board(game, no_color)).block(styled_block(
        "Board",
        MAIN_BORDER_COLOR,
        no_color,
    ));
    frame.render_widget(board, area);

    match game.run_state() {
        RunState::Running => {}
        RunState::Ready => draw_message_popup(
            frame,
            area,
            "Ready",
            Color::LightMagenta,
            &[
                "Rust Snake",
                "",
                "按 Enter、Space 或方向键开始",
                "使用 WASD 或方向键控制移动",
                "按 q 可随时退出",
            ],
            no_color,
        ),
        RunState::Paused => draw_message_popup(
            frame,
            area,
            "Paused",
            Color::LightMagenta,
            &["游戏已暂停", "", "按 Space 继续"],
            no_color,
        ),
        RunState::GameOver => draw_message_popup(
            frame,
            area,
            "Game Over",
            Color::Red,
            &["游戏结束", "", "按 r 重新开始"],
            no_color,
        ),
    }
}

/// 根据当前状态返回底部帮助文案。
fn help_text(state: RunState) -> &'static str {
    match state {
        RunState::Ready => "Enter / Space / 方向键开始 | q 退出 | 调整窗口会重开",
        RunState::Running => "WASD/方向键移动 | Space 暂停 | r 重开 | q 退出 | 调整窗口会重开",
        RunState::Paused => "Space 继续 | r 重开 | q 退出 | 调整窗口会重开",
        RunState::GameOver => "r 重新开始 | q 退出 | 调整窗口会重开",
    }
}

/// 渲染正常游玩中的棋盘内容。
///
/// 遍历棋盘上每一个格子，确定该格子应该显示什么字符和颜色。
///
/// **字符映射规则**：
/// | 元素 | 字符 | 颜色 |
/// |------|------|------|
/// | 玩家蛇头 | 玩家自身配置 | 玩家自身配置 |
/// | 玩家蛇身 | 玩家自身配置 | 玩家自身配置 |
/// | 食物 | * | 亮红色 (FOOD_COLOR) |
/// | 超级果实 | $ | 亮黄色 (SUPER_FRUIT_COLOR) |
/// | 炸弹 | X | 红色 (BOMB_COLOR) |
/// | 敌人蛇头 | A-F | 各自对应的亮色 |
/// | 敌人蛇身 | a-f | 各自对应的暗色 |
/// | 空地 | · | 暗灰色 (MUTED_COLOR) |
///
/// **渲染优先级**（从高到低）：
/// 1. 玩家蛇头（因为玩家是主要控制对象，需要醒目）
/// 2. 物品（普通食物、超级果实、炸弹）
/// 3. 玩家蛇身
/// 4. 敌人蛇（头和身）
/// 5. 空地
///
/// 敌人使用不同的字母来区分：
/// - 蛇头用大写字母：A, B, C, D, E, F（循环）
/// - 蛇身用小写字母：a, b, c, d, e, f（循环）
/// - 每条敌人蛇有配对的颜色（洋红、青、黄、红循环）
fn render_live_board(game: &GameState, no_color: bool) -> Vec<Line<'static>> {
    // 获取棋盘尺寸
    let (width, height) = game.board_size();

    // 提取玩家蛇头位置，避免在循环中重复访问
    let player = game.player();
    let player_head = Some(player.head());

    // 预分配行数组
    let mut rows = Vec::with_capacity(height as usize);

    // 按行遍历棋盘（从上到下）
    for y in 0..height {
        // 预分配每行的单元格数组
        let mut cells = Vec::with_capacity(width as usize);

        // 按列遍历（从左到右）
        for x in 0..width {
            let position = Position { x, y };

            // 根据渲染优先级确定该位置应该显示什么
            let cell = if Some(position) == player_head {
                // 玩家蛇头：优先级最高，用 @ 符号
                snake_cell(player.head_glyph(), player.head_color(), true, no_color)
            } else if game.foods().contains(&position) {
                // 食物：用 * 符号
                Span::styled(
                    "*",
                    style_with_color(FOOD_COLOR, no_color).add_modifier(Modifier::BOLD),
                )
            } else if game.super_foods().contains(&position) {
                // 超级果实：用 $ 符号
                Span::styled(
                    "$",
                    style_with_color(SUPER_FRUIT_COLOR, no_color).add_modifier(Modifier::BOLD),
                )
            } else if game.bombs().contains(&position) {
                // 炸弹：用 X 符号
                Span::styled(
                    "X",
                    style_with_color(BOMB_COLOR, no_color).add_modifier(Modifier::BOLD),
                )
            } else if player.body().contains(&position) {
                // 玩家蛇身：用 o 符号
                snake_cell(player.body_glyph(), player.body_color(), false, no_color)
            } else if let Some((enemy, is_head)) = enemy_cell(game.enemies(), position) {
                // AI 蛇身或蛇头：使用蛇自身绑定的外观
                let glyph = if is_head {
                    enemy.head_glyph()
                } else {
                    enemy.body_glyph()
                };
                let color = if is_head {
                    enemy.head_color()
                } else {
                    enemy.body_color()
                };
                snake_cell(glyph, color, is_head, no_color)
            } else {
                // 空地：用 · 符号
                Span::styled("·", style_with_color(MUTED_COLOR, no_color))
            };

            // 将单元格添加到当前行
            cells.push(cell);
        }

        // 将完成的行添加到结果中
        rows.push(Line::from(cells));
    }

    rows
}

/// 使用统一样式渲染一个蛇身单元格。
fn snake_cell(glyph: &'static str, color: Color, is_head: bool, no_color: bool) -> Span<'static> {
    let style = if is_head {
        style_with_color(color, no_color).add_modifier(Modifier::BOLD)
    } else {
        style_with_color(color, no_color)
    };

    Span::styled(glyph, style)
}

/// 将所有 AI 的方向格式化为可读字符串。
///
/// 格式示例："A^ Bv Cd" 表示：
/// - A 蛇向上了
/// - B 蛇向下了
/// - C 蛇向右了
///
/// 每个 AI 用一个字母（标签）+ 方向符号表示。
fn format_enemy_directions(enemies: &[EnemySnake], no_color: bool) -> Vec<Span<'static>> {
    if enemies.is_empty() {
        return vec![Span::styled("-", style_with_color(MUTED_COLOR, no_color))];
    }

    let mut spans = Vec::with_capacity(enemies.len() * 2);
    for (index, enemy) in enemies.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }

        spans.push(Span::styled(
            format!(
                "{}{}",
                enemy.head_glyph(),
                direction_label(enemy.direction())
            ),
            style_with_color(enemy.head_color(), no_color).add_modifier(Modifier::BOLD),
        ));
    }

    spans
}

/// 将所有 AI 的分数格式化为可读字符串。
///
/// 格式示例："A:3 B:5 C:2" 表示：
/// - A 蛇得 3 分
/// - B 蛇得 5 分
/// - C 蛇得 2 分
fn format_enemy_scores(enemies: &[EnemySnake], no_color: bool) -> Vec<Span<'static>> {
    if enemies.is_empty() {
        return vec![Span::styled("-", style_with_color(MUTED_COLOR, no_color))];
    }

    let mut spans = Vec::with_capacity(enemies.len() * 2);
    for (index, enemy) in enemies.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }

        spans.push(Span::styled(
            format!("{}:{}", enemy.head_glyph(), enemy.score()),
            style_with_color(enemy.head_color(), no_color).add_modifier(Modifier::BOLD),
        ));
    }

    spans
}

/// 将蛇的移动方向转换为符号表示。
fn direction_label(direction: SnakeDirection) -> &'static str {
    match direction {
        SnakeDirection::Up => "^",
        SnakeDirection::Down => "v",
        SnakeDirection::Left => "<",
        SnakeDirection::Right => ">",
    }
}

/// 检查指定位置是否有 AI 蛇占据。
///
/// **返回值**：
/// - `None`：该位置没有被任何 AI 占据
/// - `Some((index, true))`：该位置是第 index 条 AI 的蛇头
/// - `Some((index, false))`：该位置是第 index 条 AI 的蛇身（非头）
fn enemy_cell(enemies: &[EnemySnake], position: Position) -> Option<(&EnemySnake, bool)> {
    enemies.iter().find_map(|enemy| {
        if Some(position) == enemy.body().back().copied() {
            Some((enemy, true))
        } else if enemy.body().contains(&position) {
            Some((enemy, false))
        } else {
            None
        }
    })
}

/// 在棋盘中央绘制一个整体居中、段内左对齐的提示面板。
fn draw_message_popup(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    title: &'static str,
    border_color: Color,
    lines: &[&'static str],
    no_color: bool,
) {
    let popup_height = (lines.len() as u16).saturating_add(2);
    let popup_area = centered_area(area, 40, popup_height);
    let content = lines
        .iter()
        .map(|line| Line::from(Span::styled(*line, style_with_color(TEXT_COLOR, no_color))))
        .collect::<Vec<_>>();
    let popup = Paragraph::new(content).block(styled_block(title, border_color, no_color));
    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

/// 在终端过小时绘制提示界面。
fn draw_too_small(frame: &mut Frame, no_color: bool) {
    let area = frame.area();
    let popup_area = centered_area(area, 42, 7);
    let popup = Paragraph::new(vec![
        Line::from(Span::styled(
            "终端窗口过小",
            style_with_color(Color::LightYellow, no_color).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "请放大终端后继续游戏",
            style_with_color(TEXT_COLOR, no_color),
        )),
        Line::from(Span::styled(
            "调整到足够大小后会自动重开",
            style_with_color(TEXT_COLOR, no_color),
        )),
        Line::from(Span::styled(
            "按 q 退出",
            style_with_color(MUTED_COLOR, no_color),
        )),
    ])
    .block(styled_block(
        "Window Too Small",
        Color::LightYellow,
        no_color,
    ));

    frame.render_widget(Clear, area);
    frame.render_widget(popup, popup_area);
}

/// 创建带颜色边框的统一面板样式。
fn styled_block(title: &'static str, border_color: Color, no_color: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(style_with_color(border_color, no_color))
        .title_style(style_with_color(border_color, no_color).add_modifier(Modifier::BOLD))
}

/// 根据是否关闭颜色返回对应的文本样式。
fn style_with_color(color: Color, no_color: bool) -> Style {
    if no_color {
        Style::default()
    } else {
        Style::default().fg(color)
    }
}

/// 在指定区域中计算一个居中的内容块。
fn centered_area(area: ratatui::layout::Rect, width: u16, height: u16) -> ratatui::layout::Rect {
    // 限制 popup 的最大尺寸，确保不超过可用区域
    let popup_width = width.min(area.width.saturating_sub(2)).max(1);
    let popup_height = height.min(area.height.saturating_sub(2)).max(1);

    // 垂直方向分割：上下填充 + 中间内容
    let vertical: [ratatui::layout::Rect; 3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(popup_height),
            Constraint::Fill(1),
        ])
        .areas(area);

    // 水平方向分割：左右填充 + 中间内容
    let horizontal: [ratatui::layout::Rect; 3] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(popup_width),
            Constraint::Fill(1),
        ])
        .areas(vertical[1]);

    // 返回居中的内容区域
    horizontal[1]
}
