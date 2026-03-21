//! 定义尸块数据，以及提供给渲染层的显示信息。

use ratatui::style::Color;

use super::Position;

/// 棋盘上的单个尸块。
///
/// 一条蛇死亡后会被拆成多个独立尸块，每个尸块都有自己的腐化时间。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpsePiece {
    position: Position,
    group_id: u64,
    glyph: &'static str,
    color: Color,
    bold: bool,
    decays_at_tick: u64,
}

/// 渲染层需要的尸块显示信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpseCell {
    glyph: &'static str,
    color: Color,
    bold: bool,
}

impl CorpsePiece {
    /// 创建一个新的尸块。
    pub(super) fn new(
        position: Position,
        group_id: u64,
        glyph: &'static str,
        color: Color,
        bold: bool,
        decays_at_tick: u64,
    ) -> Self {
        Self {
            position,
            group_id,
            glyph,
            color,
            bold,
            decays_at_tick,
        }
    }

    /// 返回尸块所在位置。
    pub fn position(&self) -> Position {
        self.position
    }

    /// 返回尸块所属的死亡批次编号。
    pub fn group_id(&self) -> u64 {
        self.group_id
    }

    /// 判断尸块是否已到达腐化时机。
    pub(super) fn should_decay(self, current_tick: u64) -> bool {
        current_tick >= self.decays_at_tick
    }

    /// 返回尸块的渲染信息。
    pub(super) fn cell(self) -> CorpseCell {
        CorpseCell {
            glyph: self.glyph,
            color: self.color,
            bold: self.bold,
        }
    }
}

impl CorpseCell {
    /// 返回渲染字符。
    pub fn glyph(&self) -> &'static str {
        self.glyph
    }

    /// 返回颜色。
    pub fn color(&self) -> Color {
        self.color
    }

    /// 返回是否需要加粗。
    pub fn bold(&self) -> bool {
        self.bold
    }
}
