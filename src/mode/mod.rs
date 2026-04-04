mod macro_bind_key;
mod macro_name;
mod macro_replay;
mod macro_search;
mod normal;
mod recording;

use crate::{
    backend::{Backend, KeyEvent},
    input::{
        dynamic_cols, dynamic_rows, keys_to_pos, InputState, sub_cols, sub_rows,
    },
    macro_store::{MacroAction, MacroStore},
    render::{render_grid, render_rec_indicator},
};

pub enum ModeTransition {
    Stay,
    Redraw,
    Enter(Mode),
    Back,
    Exit,
}

pub enum Mode {
    Normal {
        input_state: InputState,
        target: Option<(u32, u32)>,
        drag_origin: Option<(u32, u32)>,
    },
    MacroRecording {
        input_state: InputState,
        target: Option<(u32, u32)>,
        drag_origin: Option<(u32, u32)>,
        recorded_actions: Vec<MacroAction>,
        drag_start_keys: String,
    },
    MacroBindKey {
        actions: Vec<MacroAction>,
    },
    MacroName {
        bind_key: Option<char>,
        name: Vec<char>,
        actions: Vec<MacroAction>,
    },
    MacroReplayWait,
    MacroSearch {
        query: Vec<char>,
        selected: usize,
    },
}

impl Mode {
    pub fn handle_key<B: Backend>(
        &self,
        width: u32,
        height: u32,
        backend: &mut B,
        key: &KeyEvent,
        macro_store: &mut MacroStore,
    ) -> anyhow::Result<ModeTransition> {
        match self {
            Mode::Normal {
                input_state,
                target,
                drag_origin,
            } => normal::handle_key(
                width,
                height,
                key,
                backend,
                input_state,
                *target,
                *drag_origin,
            ),
            Mode::MacroRecording {
                input_state,
                target,
                drag_origin,
                recorded_actions,
                drag_start_keys,
            } => recording::handle_key(
                width,
                height,
                key,
                backend,
                input_state,
                *target,
                *drag_origin,
                recorded_actions,
                drag_start_keys,
            ),
            Mode::MacroBindKey { actions } => macro_bind_key::handle_key(key, actions),
            Mode::MacroName {
                bind_key,
                name,
                actions,
            } => macro_name::handle_key(key, *bind_key, name, actions, macro_store),
            Mode::MacroReplayWait => {
                macro_replay::handle_key(width, height, key, backend, macro_store)
            }
            Mode::MacroSearch { query, selected } => {
                macro_search::handle_key(width, height, key, backend, query, *selected, macro_store)
            }
        }
    }

    pub fn draw<B: Backend>(
        &self,
        backend: &mut B,
        pixels: &mut [u8],
        width: u32,
        height: u32,
        macro_store: &MacroStore,
    ) -> anyhow::Result<()> {
        match self {
            Mode::Normal {
                input_state,
                drag_origin,
                ..
            } => normal::draw(
                backend,
                pixels,
                width,
                height,
                input_state,
                drag_origin.is_some(),
            ),
            Mode::MacroRecording {
                input_state,
                drag_origin,
                ..
            } => recording::draw(
                backend,
                pixels,
                width,
                height,
                input_state,
                drag_origin.is_some(),
            ),
            Mode::MacroBindKey { .. } => macro_bind_key::draw(backend, pixels, width, height),
            Mode::MacroName { bind_key, name, .. } => {
                macro_name::draw(backend, pixels, width, height, name, *bind_key)
            }
            Mode::MacroReplayWait => macro_replay::draw(backend, pixels, width, height),
            Mode::MacroSearch { query, selected } => macro_search::draw(
                backend,
                pixels,
                width,
                height,
                query,
                *selected,
                macro_store,
            ),
        }
    }
}

pub(super) fn column_center(width: u32, height: u32, col: u32) -> Option<(u32, u32)> {
    let ncols = dynamic_cols(width);
    if col >= ncols {
        return None;
    }
    let cell_w = width / ncols;
    Some((col * cell_w + cell_w / 2, height / 2))
}

pub(super) fn main_cell_center(
    width: u32,
    height: u32,
    col: u32,
    row: u32,
) -> Option<(u32, u32)> {
    let ncols = dynamic_cols(width);
    let nrows = dynamic_rows(height);
    if col >= ncols || row >= nrows {
        return None;
    }
    let cell_w = width / ncols;
    let cell_h = height / nrows;
    Some((col * cell_w + cell_w / 2, row * cell_h + cell_h / 2))
}

pub(super) fn sub_cell_center(
    width: u32,
    height: u32,
    col: u32,
    row: u32,
    sub_col: u32,
    sub_row: u32,
) -> Option<(u32, u32)> {
    let ncols = dynamic_cols(width);
    let nrows = dynamic_rows(height);
    if col >= ncols || row >= nrows {
        return None;
    }
    if sub_col >= sub_cols() || sub_row >= sub_rows() {
        return None;
    }
    let cell_w = width / ncols;
    let cell_h = height / nrows;
    let sub_cell_w = cell_w / sub_cols();
    let sub_cell_h = cell_h / sub_rows();
    Some((
        col * cell_w + sub_col * sub_cell_w + sub_cell_w / 2,
        row * cell_h + sub_row * sub_cell_h + sub_cell_h / 2,
    ))
}

pub(super) fn handle_char_input<B: Backend>(
    width: u32,
    height: u32,
    backend: &mut B,
    input_state: &InputState,
    ch: char,
    target: Option<(u32, u32)>,
    drag_origin: Option<(u32, u32)>,
    debug_prefix: &str,
) -> anyhow::Result<(InputState, Option<(u32, u32)>)> {
    use crate::input::hints;
    use crate::input::sub_hints;
    use crate::input::sub_cols;

    match input_state {
        InputState::First => {
            let col = hints().iter().position(|c| *c == ch).unwrap_or(0) as u32;
            let ncols = dynamic_cols(width);
            if col < ncols {
                let cell_w = width / ncols;
                let cx = col * cell_w + cell_w / 2;
                let cy = height / 2;
                eprintln!(
                    "[debug] action: {} first hint key '{}' selected column {} center=({}, {})",
                    debug_prefix, ch, col, cx, cy
                );
                backend.move_mouse(cx, cy)?;
                Ok((InputState::Second(ch), Some((cx, cy))))
            } else {
                eprintln!("[debug] action: {} first hint key '{}'", debug_prefix, ch);
                Ok((InputState::Second(ch), target))
            }
        }
        InputState::Second(first) => {
            let col = hints().iter().position(|c| *c == *first).unwrap_or(0) as u32;
            let row = hints().iter().position(|c| *c == ch).unwrap_or(0) as u32;
            let ncols = dynamic_cols(width);
            let nrows = dynamic_rows(height);

            // Reject if outside the currently rendered grid
            if col >= ncols || row >= nrows {
                eprintln!(
                    "[debug] action: {} second hint '{}' ignored, grid position out of bounds",
                    debug_prefix, ch
                );
                return Ok((input_state.clone(), target));
            }

            let cell_w = width / ncols;
            let cell_h = height / nrows;
            let cx = col * cell_w + cell_w / 2;
            let cy = row * cell_h + cell_h / 2;

            eprintln!("[debug] action: {} move_mouse to main cell ({}, {})", debug_prefix, cx, cy);
            backend.move_mouse(cx, cy)?;

            Ok((InputState::SubFirst { col, row }, Some((cx, cy))))
        }
        InputState::SubFirst { col, row } => {
            if let Some(idx) = sub_hints().iter().position(|c| *c == ch) {
                let sub_col = idx as u32 % sub_cols();
                let sub_row = idx as u32 / sub_cols();
                if let Some((cx, cy)) = sub_cell_center(
                    width,
                    height,
                    *col,
                    *row,
                    sub_col,
                    sub_row,
                ) {
                    eprintln!(
                        "[debug] action: {} move_mouse to sub cell ({}, {})",
                        debug_prefix, cx, cy
                    );
                    backend.move_mouse(cx, cy)?;

                    return Ok((
                        InputState::Ready {
                            col: *col,
                            row: *row,
                            sub_col,
                            sub_row,
                        },
                        Some((cx, cy)),
                    ));
                }
            }
            Ok((input_state.clone(), target))
        }
        InputState::Ready { .. } => Ok((input_state.clone(), target)),
    }
}

pub(super) fn draw_grid(
    pixels: &mut [u8],
    w: u32,
    h: u32,
    state: &InputState,
    dragging: bool,
    recording: bool,
    backend: &mut dyn Backend,
) -> anyhow::Result<()> {
    render_grid(pixels, w, h, state, dragging);
    if recording {
        render_rec_indicator(pixels, w);
    }
    backend.present(pixels, w, h)
}

pub(super) fn replay_macro(
    actions: &[MacroAction],
    w: u32,
    h: u32,
    backend: &mut dyn Backend,
) -> anyhow::Result<()> {
    for action in actions {
        match action {
            MacroAction::Move(keys) => {
                if let Some((x, y)) = keys_to_pos(keys, w, h) {
                    backend.move_mouse(x, y)?;
                }
            }
            MacroAction::Click(keys) => {
                if let Some((x, y)) = keys_to_pos(keys, w, h) {
                    backend.click(x, y)?;
                }
            }
            MacroAction::DoubleClick(keys) => {
                if let Some((x, y)) = keys_to_pos(keys, w, h) {
                    backend.double_click(x, y)?;
                }
            }
            MacroAction::RightClick(keys) => {
                if let Some((x, y)) = keys_to_pos(keys, w, h) {
                    backend.right_click(x, y)?;
                }
            }
            MacroAction::Drag(start_keys, end_keys) => {
                if let (Some((x1, y1)), Some((x2, y2))) =
                    (keys_to_pos(start_keys, w, h), keys_to_pos(end_keys, w, h))
                {
                    backend.drag_select(x1, y1, x2, y2)?;
                }
            }
        }
    }
    Ok(())
}
