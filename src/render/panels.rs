use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::game::{Direction as SnakeDirection, GameState, RunState, Snake};

use super::style::{MAIN_BORDER_COLOR, MUTED_COLOR, TEXT_COLOR, style_with_color, styled_block};

pub(crate) fn draw_header(frame: &mut Frame, area: Rect, no_color: bool) {
    let header = Paragraph::new(Line::from("Rust Snake"))
        .alignment(Alignment::Center)
        .style(style_with_color(Color::LightCyan, no_color).add_modifier(Modifier::BOLD))
        .block(styled_block("Title", MAIN_BORDER_COLOR, no_color));
    frame.render_widget(header, area);
}

pub(crate) fn draw_status(frame: &mut Frame, area: Rect, game: &GameState, no_color: bool) {
    let status_text = match game.run_state() {
        RunState::Ready => Span::styled("Ready", style_with_color(Color::Cyan, no_color)),
        RunState::Running => Span::styled("Running", style_with_color(Color::Green, no_color)),
        RunState::Paused => Span::styled("Paused", style_with_color(Color::Yellow, no_color)),
        RunState::GameOver => Span::styled("Game Over", style_with_color(Color::Red, no_color)),
    };

    let direction_text = match game.direction() {
        SnakeDirection::Up => "Up",
        SnakeDirection::Down => "Down",
        SnakeDirection::Left => "Left",
        SnakeDirection::Right => "Right",
    };

    let ai_direction_spans = format_enemy_directions(game.enemies(), no_color);
    let ai_score_spans = format_enemy_scores(game.enemies(), no_color);

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

    frame.render_widget(info, area);
}

pub(crate) fn draw_footer(frame: &mut Frame, area: Rect, state: RunState, no_color: bool) {
    let footer = Paragraph::new(Line::from(help_text(state)))
        .alignment(Alignment::Center)
        .style(style_with_color(MUTED_COLOR, no_color))
        .block(styled_block("Help", MAIN_BORDER_COLOR, no_color));
    frame.render_widget(footer, area);
}

pub(crate) fn draw_state_overlay(
    frame: &mut Frame,
    area: Rect,
    state: RunState,
    no_color: bool,
    centered_area: fn(Rect, u16, u16) -> Rect,
) {
    match state {
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
            centered_area,
        ),
        RunState::Paused => draw_message_popup(
            frame,
            area,
            "Paused",
            Color::LightMagenta,
            &["游戏已暂停", "", "按 Space 继续"],
            no_color,
            centered_area,
        ),
        RunState::GameOver => draw_message_popup(
            frame,
            area,
            "Game Over",
            Color::Red,
            &["游戏结束", "", "按 r 重新开始"],
            no_color,
            centered_area,
        ),
    }
}

pub(crate) fn draw_too_small(
    frame: &mut Frame,
    no_color: bool,
    centered_area: fn(Rect, u16, u16) -> Rect,
) {
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

fn help_text(state: RunState) -> &'static str {
    match state {
        RunState::Ready => "Enter / Space / 方向键开始 | q 退出 | 调整窗口会重开",
        RunState::Running => "WASD/方向键移动 | Space 暂停 | r 重开 | q 退出 | 调整窗口会重开",
        RunState::Paused => "Space 继续 | r 重开 | q 退出 | 调整窗口会重开",
        RunState::GameOver => "r 重新开始 | q 退出 | 调整窗口会重开",
    }
}

fn format_enemy_directions(enemies: &[Snake], no_color: bool) -> Vec<Span<'static>> {
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

fn format_enemy_scores(enemies: &[Snake], no_color: bool) -> Vec<Span<'static>> {
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

fn direction_label(direction: SnakeDirection) -> &'static str {
    match direction {
        SnakeDirection::Up => "^",
        SnakeDirection::Down => "v",
        SnakeDirection::Left => "<",
        SnakeDirection::Right => ">",
    }
}

fn draw_message_popup(
    frame: &mut Frame,
    area: Rect,
    title: &'static str,
    border_color: Color,
    lines: &[&'static str],
    no_color: bool,
    centered_area: fn(Rect, u16, u16) -> Rect,
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
