use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::game::{GameState, RunState};

pub fn draw(frame: &mut Frame, game: &GameState) {
    let [header_area, body_area, footer_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .areas(frame.area());

    let header = Paragraph::new(Line::from("Rust Snake"))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Title"));

    let status_text = match game.run_state() {
        RunState::Running => Span::styled("Running", Style::default().fg(Color::Green)),
        RunState::Paused => Span::styled("Paused", Style::default().fg(Color::Yellow)),
    };

    let body = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Tick: "),
            Span::styled(game.tick_count().to_string(), Style::default().bold()),
        ]),
        Line::from(""),
        Line::from(vec![Span::raw("State: "), status_text]),
        Line::from(""),
        Line::from("基础 TUI 骨架已完成。"),
        Line::from("下一步会在这里绘制游戏网格与蛇。"),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title("Status"));

    let footer = Paragraph::new(Line::from("Space 暂停 | q 退出"))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Help"));

    frame.render_widget(header, header_area);
    frame.render_widget(body, body_area);
    frame.render_widget(footer, footer_area);
}
