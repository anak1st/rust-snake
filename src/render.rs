use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::game::{Direction as SnakeDirection, GameState, Position, RunState};

/// 顶部标题栏的固定高度。
const HEADER_HEIGHT: u16 = 3;
/// 底部帮助栏的固定高度。
const FOOTER_HEIGHT: u16 = 3;
/// 状态信息区域的固定高度。
const INFO_HEIGHT: u16 = 6;
/// 允许的最小棋盘宽度，避免窗口过小时不可玩。
const MIN_BOARD_WIDTH: u16 = 10;
/// 允许的最小棋盘高度，避免窗口过小时不可玩。
const MIN_BOARD_HEIGHT: u16 = 6;

/// 根据当前游戏状态绘制整个界面。
pub fn draw(frame: &mut Frame, game: &GameState, window_too_small: bool) {
    if window_too_small {
        draw_too_small(frame);
        return;
    }

    let [header_area, body_area, footer_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Min(8),
            Constraint::Length(FOOTER_HEIGHT),
        ])
        .areas(frame.area());

    let [info_area, board_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(INFO_HEIGHT), Constraint::Min(3)])
        .areas(body_area);

    let header = Paragraph::new(Line::from("Rust Snake"))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Title"));

    let status_text = match game.run_state() {
        RunState::Ready => Span::styled("Ready", Style::default().fg(Color::Cyan)),
        RunState::Running => Span::styled("Running", Style::default().fg(Color::Green)),
        RunState::Paused => Span::styled("Paused", Style::default().fg(Color::Yellow)),
        RunState::GameOver => Span::styled("Game Over", Style::default().fg(Color::Red)),
    };

    let direction_text = match game.direction() {
        SnakeDirection::Up => "Up",
        SnakeDirection::Down => "Down",
        SnakeDirection::Left => "Left",
        SnakeDirection::Right => "Right",
    };

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Tick: "),
            Span::styled(game.tick_count().to_string(), Style::default().bold()),
        ]),
        Line::from(vec![
            Span::raw("Score: "),
            Span::styled(game.score().to_string(), Style::default().bold()),
        ]),
        Line::from(""),
        Line::from(vec![Span::raw("State: "), status_text]),
        Line::from(vec![
            Span::raw("Direction: "),
            Span::styled(direction_text, Style::default().bold()),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Status"));

    let footer = Paragraph::new(Line::from(help_text(game.run_state())))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Help"));

    frame.render_widget(header, header_area);
    frame.render_widget(info, info_area);
    draw_board(frame, board_area, game);
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
fn draw_board(frame: &mut Frame, area: ratatui::layout::Rect, game: &GameState) {
    let board = Paragraph::new(render_live_board(game))
        .block(Block::default().borders(Borders::ALL).title("Board"));
    frame.render_widget(board, area);

    match game.run_state() {
        RunState::Running => {}
        RunState::Ready => draw_message_popup(
            frame,
            area,
            "Ready",
            &["Rust Snake", "", "按 Enter、Space 或方向键开始", "使用 WASD 或方向键控制移动", "按 q 可随时退出"],
        ),
        RunState::Paused => {
            draw_message_popup(frame, area, "Paused", &["游戏已暂停", "", "按 Space 继续"])
        }
        RunState::GameOver => {
            draw_message_popup(frame, area, "Game Over", &["游戏结束", "", "按 r 重新开始"])
        }
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
fn render_live_board(game: &GameState) -> Vec<Line<'static>> {
    let (width, height) = game.board_size();
    let head = game.snake().back().copied();
    let mut rows = Vec::with_capacity(height as usize);

    for y in 0..height {
        let mut row = String::with_capacity(width as usize);

        for x in 0..width {
            let position = Position { x, y };
            let cell = if Some(position) == head {
                "@"
            } else if game.food() == position {
                "*"
            } else if game.snake().contains(&position) {
                "o"
            } else {
                "."
            };

            row.push_str(cell);
        }

        rows.push(Line::from(row));
    }

    rows
}

/// 在棋盘中央绘制一个整体居中、段内左对齐的提示面板。
fn draw_message_popup(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    title: &'static str,
    lines: &[&'static str],
) {
    let popup_height = (lines.len() as u16).saturating_add(2);
    let popup_area = centered_area(area, 40, popup_height);
    let content = lines.iter().map(|line| Line::from(*line)).collect::<Vec<_>>();
    let popup = Paragraph::new(content).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

/// 在终端过小时绘制提示界面。
fn draw_too_small(frame: &mut Frame) {
    let area = frame.area();
    let popup_area = centered_area(area, 42, 7);
    let popup = Paragraph::new(vec![
        Line::from("终端窗口过小"),
        Line::from(""),
        Line::from("请放大终端后继续游戏"),
        Line::from("调整到足够大小后会自动重开"),
        Line::from("按 q 退出"),
    ])
    .block(Block::default().borders(Borders::ALL).title("Window Too Small"));

    frame.render_widget(Clear, area);
    frame.render_widget(popup, popup_area);
}

/// 在指定区域中计算一个居中的内容块。
fn centered_area(area: ratatui::layout::Rect, width: u16, height: u16) -> ratatui::layout::Rect {
    let popup_width = width.min(area.width.saturating_sub(2)).max(1);
    let popup_height = height.min(area.height.saturating_sub(2)).max(1);

    let vertical: [ratatui::layout::Rect; 3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(popup_height),
            Constraint::Fill(1),
        ])
        .areas(area);

    let horizontal: [ratatui::layout::Rect; 3] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(popup_width),
            Constraint::Fill(1),
        ])
        .areas(vertical[1]);

    horizontal[1]
}
