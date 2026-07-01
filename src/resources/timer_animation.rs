use ratatui::{
    style::Stylize,
    text::{Line, Span},
};

use crate::resources::color_palette::{
    RUNNING_TIMER_COLOR, RUNNING_TIMER_COLOR_2, RUNNING_TIMER_COLOR_3,
};

pub fn create_timer_animation<'a>() -> Vec<Line<'a>> {
    let line_common = Span::raw(" ").bg(RUNNING_TIMER_COLOR);
    let line_mid_light = Span::raw(" ").bg(RUNNING_TIMER_COLOR_2);
    let line_max_light = Span::raw(" ").bg(RUNNING_TIMER_COLOR_3);

    let vec = Vec::from([
        Line::from_iter([
            line_common.clone(),
            line_common.clone(),
            line_common.clone(),
            line_common.clone(),
            line_common.clone(),
            line_common.clone(),
            line_common.clone(),
            line_common.clone(),
        ]),
        Line::from_iter([
            line_common.clone(),
            line_common.clone(),
            line_common.clone(),
            line_mid_light.clone(),
            line_mid_light.clone(),
            line_common.clone(),
            line_common.clone(),
            line_common.clone(),
        ]),
        Line::from_iter([
            line_common.clone(),
            line_common.clone(),
            line_mid_light.clone(),
            line_max_light.clone(),
            line_max_light.clone(),
            line_mid_light.clone(),
            line_common.clone(),
            line_common.clone(),
        ]),
        Line::from_iter([
            line_common.clone(),
            line_common.clone(),
            line_common.clone(),
            line_mid_light.clone(),
            line_mid_light.clone(),
            line_common.clone(),
            line_common.clone(),
            line_common.clone(),
        ]),
    ]);

    vec
}
