//! 汇总渲染配色与通用样式辅助函数。

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};

/// 普通文字的基础颜色。
pub(crate) const TEXT_COLOR: Color = Color::White;
/// 次要信息的弱化颜色。
pub(crate) const MUTED_COLOR: Color = Color::DarkGray;
/// 食物的强调颜色。
pub(crate) const FOOD_COLOR: Color = Color::Green;
/// 超级果实的强调颜色。
pub(crate) const SUPER_FRUIT_COLOR: Color = Color::LightYellow;
/// 炸弹的危险颜色。
pub(crate) const BOMB_COLOR: Color = Color::Red;
/// 主界面统一边框颜色。
pub(crate) const MAIN_BORDER_COLOR: Color = Color::White;

/// 创建带标题和边框样式的 Block。
///
/// 根据 `no_color` 参数决定是否应用颜色样式。
pub(crate) fn styled_block(
    title: &'static str,
    border_color: Color,
    no_color: bool,
) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(style_with_color(border_color, no_color))
        .title_style(style_with_color(border_color, no_color).add_modifier(Modifier::BOLD))
}

/// 创建带前景色的 Style。
///
/// 如果 `no_color` 为 true，则返回默认样式（无颜色）。
pub(crate) fn style_with_color(color: Color, no_color: bool) -> Style {
    if no_color {
        Style::default()
    } else {
        Style::default().fg(color)
    }
}
