use super::{column_center, main_cell_center, Mode};
use anyhow::Ok;

use crate::{
    backend::{Backend, KeyEvent},
    input::{hints, sub_hints, InputState},
    macro_store::MacroAction,
    mode::{draw_grid, ModeTransition},
};

pub(super) fn handle_key<B: Backend>(
    width: u32,
    height: u32,
    key: &KeyEvent,
    backend: &mut B,
    input_state: &InputState,
    target: Option<(u32, u32)>,
    drag_origin: Option<(u32, u32)>,
) -> anyhow::Result<ModeTransition> {
    match key {
        KeyEvent::Close => {
            eprintln!("[debug] action: exit overlay");
            Ok(ModeTransition::Exit)
        }
        KeyEvent::Back => {
            eprintln!("[debug] action: back within selection");
            match input_state {
                InputState::First => Ok(ModeTransition::Stay),
                InputState::Second(_) => Ok(ModeTransition::Enter(Mode::Normal {
                    input_state: InputState::First,
                    target: None,
                    drag_origin,
                })),
                InputState::SubFirst { col, .. } => {
                    let target = column_center(width, height, *col);
                    if let Some((x, y)) = target {
                        backend.move_mouse(x, y)?;
                    }
                    Ok(ModeTransition::Enter(Mode::Normal {
                        input_state: InputState::Second(hints()[*col as usize]),
                        target,
                        drag_origin,
                    }))
                }
                InputState::Ready { col, row, .. } => {
                    let target = main_cell_center(width, height, *col, *row);
                    if let Some((x, y)) = target {
                        backend.move_mouse(x, y)?;
                    }
                    Ok(ModeTransition::Enter(Mode::Normal {
                        input_state: InputState::SubFirst {
                            col: *col,
                            row: *row,
                        },
                        target,
                        drag_origin,
                    }))
                }
            }
        }
        KeyEvent::Undo => {
            eprintln!("[debug] action: undo/back stack");
            Ok(ModeTransition::Back)
        }
        KeyEvent::Click => {
            if let Some((x, y)) = target {
                if let Some((ox, oy)) = drag_origin {
                    eprintln!(
                        "[debug] action: drag_select from ({}, {}) to ({}, {})",
                        ox, oy, x, y
                    );
                    backend.drag_select(ox, oy, x, y)?;
                } else {
                    eprintln!("[debug] action: click at ({}, {})", x, y);
                    backend.click(x, y)?;
                }
            } else {
                eprintln!("[debug] action: click ignored, no target");
            }
            Ok(ModeTransition::Exit)
        }
        KeyEvent::DoubleClick => {
            if let Some((x, y)) = target {
                if let Some((ox, oy)) = drag_origin {
                    eprintln!(
                        "[debug] action: drag_select from ({}, {}) to ({}, {})",
                        ox, oy, x, y
                    );
                    backend.drag_select(ox, oy, x, y)?;
                } else {
                    eprintln!("[debug] action: double_click at ({}, {})", x, y);
                    backend.double_click(x, y)?;
                }
            } else {
                eprintln!("[debug] action: double_click ignored, no target");
            }
            Ok(ModeTransition::Exit)
        }
        KeyEvent::RightClick if drag_origin.is_none() => {
            if let Some((x, y)) = target {
                eprintln!("[debug] action: right_click at ({}, {})", x, y);
                backend.right_click(x, y)?;
            } else {
                eprintln!("[debug] action: right_click ignored, no target");
            }
            Ok(ModeTransition::Exit)
        }
        KeyEvent::Char('/')
            if matches!(
                input_state,
                InputState::Ready { .. } | InputState::SubFirst { .. }
            ) =>
        {
            eprintln!("[debug] action: enter drag mode");
            Ok(ModeTransition::Enter(Mode::Normal {
                input_state: InputState::First,
                target: None,
                drag_origin: target,
            }))
        }
        KeyEvent::Char('@')
            if matches!(input_state, InputState::First) && drag_origin.is_none() =>
        {
            eprintln!("[debug] action: open macro replay wait");
            Ok(ModeTransition::Enter(Mode::MacroReplayWait))
        }
        KeyEvent::MacroRecord
            if matches!(input_state, InputState::First) && drag_origin.is_none() =>
        {
            eprintln!("[debug] action: start macro recording");
            Ok(ModeTransition::Enter(Mode::MacroRecording {
                input_state: InputState::First,
                target: None,
                drag_origin: None,
                recorded_actions: Vec::new(),
                drag_start_keys: String::new(),
            }))
        }
        KeyEvent::Char(ch)
            if hints().contains(ch)
                || (matches!(input_state, InputState::SubFirst { .. })
                    && sub_hints().contains(ch)) =>
        {
            let (new_input_state, new_target) = crate::mode::handle_char_input(
                width, height, backend, input_state, *ch, target, drag_origin, "",
            )?;
            Ok(ModeTransition::Enter(Mode::Normal {
                input_state: new_input_state,
                target: new_target,
                drag_origin,
            }))
        }
        KeyEvent::MacroMenu
            if matches!(
                input_state,
                InputState::SubFirst { .. } | InputState::Ready { .. }
            ) =>
        {
            eprintln!("[debug] action: open macro bind key menu");
            Ok(ModeTransition::Enter(Mode::MacroBindKey {
                actions: vec![MacroAction::Click(input_state.keys())],
            }))
        }
        KeyEvent::MacroMenu
            if matches!(input_state, InputState::First) && drag_origin.is_none() =>
        {
            eprintln!("[debug] action: open macro search");
            Ok(ModeTransition::Enter(Mode::MacroSearch {
                query: Vec::new(),
                selected: 0,
            }))
        }
        KeyEvent::ScrollUp => {
            if let Some((x, y)) = target {
                eprintln!(
                    "[debug] action: move_mouse to scroll target ({}, {}) before scroll_up",
                    x, y
                );
                backend.move_mouse(x, y)?;
            }
            eprintln!("[debug] action: scroll_up");
            backend.scroll_up()?;
            Ok(ModeTransition::Redraw)
        }
        KeyEvent::ScrollDown => {
            if let Some((x, y)) = target {
                eprintln!(
                    "[debug] action: move_mouse to scroll target ({}, {}) before scroll_down",
                    x, y
                );
                backend.move_mouse(x, y)?;
            }
            eprintln!("[debug] action: scroll_down");
            backend.scroll_down()?;
            Ok(ModeTransition::Redraw)
        }
        KeyEvent::ScrollLeft => {
            if let Some((x, y)) = target {
                eprintln!(
                    "[debug] action: move_mouse to scroll target ({}, {}) before scroll_left",
                    x, y
                );
                backend.move_mouse(x, y)?;
            }
            eprintln!("[debug] action: scroll_left");
            backend.scroll_left()?;
            Ok(ModeTransition::Redraw)
        }
        KeyEvent::ScrollRight => {
            if let Some((x, y)) = target {
                eprintln!(
                    "[debug] action: move_mouse to scroll target ({}, {}) before scroll_right",
                    x, y
                );
                backend.move_mouse(x, y)?;
            }
            eprintln!("[debug] action: scroll_right");
            backend.scroll_right()?;
            Ok(ModeTransition::Redraw)
        }
        _ => Ok(ModeTransition::Stay),
    }
}

pub(super) fn draw<B: Backend>(
    backend: &mut B,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    input_state: &InputState,
    dragging: bool,
) -> anyhow::Result<()> {
    draw_grid(pixels, width, height, input_state, dragging, false, backend)
}
