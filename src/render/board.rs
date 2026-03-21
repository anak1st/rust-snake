use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::game::{GameState, Position, Snake};

use super::style::{
    BOMB_COLOR, FOOD_COLOR, MAIN_BORDER_COLOR, MUTED_COLOR, SUPER_FRUIT_COLOR, style_with_color,
    styled_block,
};
use super::{ActiveCellFlash, AnimationFrame, CellFlashKind};

#[derive(Clone, Copy)]
struct BoardCell {
    glyph: &'static str,
    color: Color,
    bold: bool,
}

impl BoardCell {
    fn new(glyph: &'static str, color: Color, bold: bool) -> Self {
        Self { glyph, color, bold }
    }

    fn into_span(self, no_color: bool) -> Span<'static> {
        let mut style = style_with_color(self.color, no_color);
        if self.bold {
            style = style.add_modifier(Modifier::BOLD);
        }

        Span::styled(self.glyph, style)
    }
}

pub(crate) fn draw_board(
    frame: &mut Frame,
    area: Rect,
    game: &GameState,
    animation: &AnimationFrame,
    no_color: bool,
) {
    let board = Paragraph::new(render_live_board(game, animation, no_color)).block(styled_block(
        "Board",
        MAIN_BORDER_COLOR,
        no_color,
    ));
    frame.render_widget(board, area);
}

fn render_live_board(
    game: &GameState,
    animation: &AnimationFrame,
    no_color: bool,
) -> Vec<Line<'static>> {
    let (width, height) = game.board_size();
    let mut rows = Vec::with_capacity(height as usize);

    for y in 0..height {
        let mut cells = Vec::with_capacity(width as usize);
        for x in 0..width {
            let position = Position { x, y };
            let cell = board_cell_for_position(game, position);
            let animated = animate_cell(game, position, cell, animation);
            cells.push(animated.into_span(no_color));
        }
        rows.push(Line::from(cells));
    }

    rows
}

fn board_cell_for_position(game: &GameState, position: Position) -> BoardCell {
    let player = game.player();

    if player.is_alive() && position == player.head() {
        return BoardCell::new(player.head_glyph(), player.head_color(), true);
    }

    if player.is_alive() && player.body().contains(&position) {
        return BoardCell::new(player.body_glyph(), player.body_color(), false);
    }

    if let Some((enemy, is_head)) = enemy_cell(game.enemies(), position) {
        return if is_head {
            BoardCell::new(enemy.head_glyph(), enemy.head_color(), true)
        } else {
            BoardCell::new(enemy.body_glyph(), enemy.body_color(), false)
        };
    }

    if let Some(cell) = game.corpse_cell(position) {
        return BoardCell::new(cell.glyph(), cell.color(), cell.bold());
    }

    if game.foods().contains(&position) || game.legacy_foods().contains(&position) {
        return BoardCell::new("*", FOOD_COLOR, true);
    }

    if game.super_foods().contains(&position) {
        return BoardCell::new("$", SUPER_FRUIT_COLOR, true);
    }

    if game.bombs().contains(&position) {
        return BoardCell::new("X", BOMB_COLOR, true);
    }

    BoardCell::new("·", MUTED_COLOR, false)
}

fn animate_cell(
    game: &GameState,
    position: Position,
    cell: BoardCell,
    animation: &AnimationFrame,
) -> BoardCell {
    if let Some(flash) = active_flash_at(&animation.active_flashes, position) {
        return flash_cell(flash);
    }

    let cell = pulse_food_cell(game, position, cell, animation);
    pulse_super_food_cell(game, position, cell, animation)
}

fn pulse_food_cell(
    game: &GameState,
    position: Position,
    cell: BoardCell,
    animation: &AnimationFrame,
) -> BoardCell {
    if !game.foods().contains(&position) && !game.legacy_foods().contains(&position) {
        return cell;
    }

    if animation.food_pulse_on {
        BoardCell::new("*", Color::LightGreen, true)
    } else {
        BoardCell::new("*", FOOD_COLOR, true)
    }
}

fn pulse_super_food_cell(
    game: &GameState,
    position: Position,
    cell: BoardCell,
    animation: &AnimationFrame,
) -> BoardCell {
    if !game.super_foods().contains(&position) {
        return cell;
    }

    if animation.super_food_pulse_on {
        BoardCell::new("$", Color::White, true)
    } else {
        BoardCell::new("$", SUPER_FRUIT_COLOR, true)
    }
}

fn active_flash_at(flashes: &[ActiveCellFlash], position: Position) -> Option<ActiveCellFlash> {
    flashes
        .iter()
        .copied()
        .find(|flash| flash.position == position && flash.is_visible)
}

fn flash_cell(flash: ActiveCellFlash) -> BoardCell {
    match flash.kind {
        CellFlashKind::Food => BoardCell::new("*", Color::LightGreen, true),
    }
}

fn enemy_cell(enemies: &[Snake], position: Position) -> Option<(&Snake, bool)> {
    enemies.iter().find_map(|enemy| {
        if !enemy.is_alive() {
            None
        } else if Some(position) == enemy.body().back().copied() {
            Some((enemy, true))
        } else if enemy.body().contains(&position) {
            Some((enemy, false))
        } else {
            None
        }
    })
}
