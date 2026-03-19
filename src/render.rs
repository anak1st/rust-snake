use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
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
pub fn draw(frame: &mut Frame, game: &GameState) {
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

    let board = Paragraph::new(render_board_lines(game))
        .block(Block::default().borders(Borders::ALL).title("Board"));

    let footer = Paragraph::new(Line::from("WASD/方向键移动 | Space 暂停 | r 重开 | q 退出 | 调整窗口会重开"))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Help"));

    frame.render_widget(header, header_area);
    frame.render_widget(info, info_area);
    frame.render_widget(board, board_area);
    frame.render_widget(footer, footer_area);
}

/// 根据终端尺寸估算可用棋盘大小，并保留最小可玩尺寸。
pub fn board_size_for_terminal(width: u16, height: u16) -> (u16, u16) {
    let board_width = width.saturating_sub(2).max(MIN_BOARD_WIDTH);
    let board_height = height
        .saturating_sub(HEADER_HEIGHT + FOOTER_HEIGHT + INFO_HEIGHT + 2)
        .max(MIN_BOARD_HEIGHT);

    (board_width, board_height)
}

/// 把当前游戏棋盘转换成逐行渲染的文本内容。
fn render_board_lines(game: &GameState) -> Vec<Line<'static>> {
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
