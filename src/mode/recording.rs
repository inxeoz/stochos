use super::{column_center, main_cell_center, Mode};
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
    recorded_actions: &[MacroAction],
    drag_start_keys: &str,
) -> anyhow::Result<ModeTransition> {
    match key {
        KeyEvent::Back => {
            eprintln!("[debug] action: recording back within selection");
            match input_state {
                InputState::First => Ok(ModeTransition::Stay),
                InputState::Second(_) => Ok(ModeTransition::Enter(Mode::MacroRecording {
                    input_state: InputState::First,
                    target: None,
                    drag_origin,
                    recorded_actions: recorded_actions.to_vec(),
                    drag_start_keys: drag_start_keys.to_owned(),
                })),
                InputState::SubFirst { col, .. } => {
                    let target = column_center(width, height, *col);
                    if let Some((x, y)) = target {
                        backend.move_mouse(x, y)?;
                    }
                    Ok(ModeTransition::Enter(Mode::MacroRecording {
                        input_state: InputState::Second(hints()[*col as usize]),
                        target,
                        drag_origin,
                        recorded_actions: recorded_actions.to_vec(),
                        drag_start_keys: drag_start_keys.to_owned(),
                    }))
                }
                InputState::Ready { col, row, .. } => {
                    let target = main_cell_center(width, height, *col, *row);
                    if let Some((x, y)) = target {
                        backend.move_mouse(x, y)?;
                    }
                    Ok(ModeTransition::Enter(Mode::MacroRecording {
                        input_state: InputState::SubFirst {
                            col: *col,
                            row: *row,
                        },
                        target,
                        drag_origin,
                        recorded_actions: recorded_actions.to_vec(),
                        drag_start_keys: drag_start_keys.to_owned(),
                    }))
                }
            }
        }
        KeyEvent::Undo => {
            eprintln!("[debug] action: recording undo/back stack");
            Ok(ModeTransition::Back)
        }
        KeyEvent::MacroRecord => {
            eprintln!("[debug] action: stop macro recording");
            if recorded_actions.is_empty() {
                Ok(ModeTransition::Enter(Mode::Normal {
                    input_state: InputState::First,
                    target: None,
                    drag_origin: None,
                }))
            } else {
                Ok(ModeTransition::Enter(Mode::MacroBindKey {
                    actions: recorded_actions.to_vec(),
                }))
            }
        }
        KeyEvent::Close => {
            eprintln!("[debug] action: close recording and return to normal mode");
            Ok(ModeTransition::Enter(Mode::Normal {
                input_state: InputState::First,
                target: None,
                drag_origin: None,
            }))
        }
        KeyEvent::Char(ch)
            if hints().contains(ch)
                || (matches!(input_state, InputState::SubFirst { .. })
                    && sub_hints().contains(ch)) =>
        {
            let (new_input_state, new_target) = crate::mode::handle_char_input(
                width, height, backend, input_state, *ch, target, drag_origin, "recording",
            )?;
            Ok(ModeTransition::Enter(Mode::MacroRecording {
                input_state: new_input_state,
                target: new_target,
                drag_origin,
                recorded_actions: recorded_actions.to_vec(),
                drag_start_keys: drag_start_keys.to_owned(),
            }))
        }
        KeyEvent::Click | KeyEvent::DoubleClick | KeyEvent::RightClick
            if target.is_some() && drag_origin.is_none() =>
        {
            let (x, y) = target.unwrap();
            let current_keys = input_state.keys();
            let mut new_actions = recorded_actions.to_vec();
            match key {
                KeyEvent::Click => {
                    eprintln!("[debug] action: recording click at ({}, {})", x, y);
                    backend.click(x, y)?;
                    new_actions.push(MacroAction::Click(current_keys));
                }
                KeyEvent::DoubleClick => {
                    eprintln!("[debug] action: recording double_click at ({}, {})", x, y);
                    backend.double_click(x, y)?;
                    new_actions.push(MacroAction::DoubleClick(current_keys));
                }
                KeyEvent::RightClick => {
                    eprintln!("[debug] action: recording right_click at ({}, {})", x, y);
                    backend.right_click(x, y)?;
                    new_actions.push(MacroAction::RightClick(current_keys));
                }
                _ => {}
            }
            backend.reopen()?;
            Ok(ModeTransition::Enter(Mode::MacroRecording {
                input_state: InputState::First,
                target: None,
                drag_origin: None,
                recorded_actions: new_actions,
                drag_start_keys: String::new(),
            }))
        }
        KeyEvent::Click | KeyEvent::DoubleClick if target.is_some() => {
            let (x, y) = target.unwrap();
            let current_keys = input_state.keys();
            let mut new_actions = recorded_actions.to_vec();
            eprintln!(
                "[debug] action: recording drag_select from ({}, {}) to ({}, {})",
                drag_origin.unwrap().0,
                drag_origin.unwrap().1,
                x,
                y
            );
            backend.drag_select(drag_origin.unwrap().0, drag_origin.unwrap().1, x, y)?;
            new_actions.push(MacroAction::Drag(drag_start_keys.to_owned(), current_keys));
            backend.reopen()?;
            Ok(ModeTransition::Enter(Mode::MacroRecording {
                input_state: InputState::First,
                target: None,
                drag_origin: None,
                recorded_actions: new_actions,
                drag_start_keys: String::new(),
            }))
        }
        KeyEvent::MacroMenu
            if target.is_some()
                && drag_origin.is_none()
                && matches!(
                    input_state,
                    InputState::SubFirst { .. } | InputState::Ready { .. }
                ) =>
        {
            let mut new_actions = recorded_actions.to_vec();
            eprintln!("[debug] action: recording move only");
            new_actions.push(MacroAction::Move(input_state.keys()));
            Ok(ModeTransition::Enter(Mode::MacroRecording {
                input_state: InputState::First,
                target: None,
                drag_origin: None,
                recorded_actions: new_actions,
                drag_start_keys: String::new(),
            }))
        }
        KeyEvent::Char('/') if drag_origin.is_some() => {
            eprintln!("[debug] action: recording cancel drag mode");
            Ok(ModeTransition::Enter(Mode::MacroRecording {
                input_state: InputState::First,
                target: None,
                drag_origin: None,
                recorded_actions: recorded_actions.to_vec(),
                drag_start_keys: String::new(),
            }))
        }
        KeyEvent::Char('/')
            if matches!(
                input_state,
                InputState::Ready { .. } | InputState::SubFirst { .. }
            ) =>
        {
            eprintln!("[debug] action: recording enter drag mode");
            Ok(ModeTransition::Enter(Mode::MacroRecording {
                input_state: InputState::First,
                target,
                drag_origin: target,
                recorded_actions: recorded_actions.to_vec(),
                drag_start_keys: input_state.keys(),
            }))
        }
        KeyEvent::ScrollUp => {
            if let Some((x, y)) = target {
                eprintln!(
                    "[debug] action: recording move_mouse to scroll target ({}, {}) before scroll_up",
                    x, y
                );
                backend.move_mouse(x, y)?;
            }
            eprintln!("[debug] action: recording scroll_up");
            backend.scroll_up()?;
            Ok(ModeTransition::Redraw)
        }
        KeyEvent::ScrollDown => {
            if let Some((x, y)) = target {
                eprintln!(
                    "[debug] action: recording move_mouse to scroll target ({}, {}) before scroll_down",
                    x, y
                );
                backend.move_mouse(x, y)?;
            }
            eprintln!("[debug] action: recording scroll_down");
            backend.scroll_down()?;
            Ok(ModeTransition::Redraw)
        }
        KeyEvent::ScrollLeft => {
            if let Some((x, y)) = target {
                eprintln!(
                    "[debug] action: recording move_mouse to scroll target ({}, {}) before scroll_left",
                    x, y
                );
                backend.move_mouse(x, y)?;
            }
            eprintln!("[debug] action: recording scroll_left");
            backend.scroll_left()?;
            Ok(ModeTransition::Redraw)
        }
        KeyEvent::ScrollRight => {
            if let Some((x, y)) = target {
                eprintln!(
                    "[debug] action: recording move_mouse to scroll target ({}, {}) before scroll_right",
                    x, y
                );
                backend.move_mouse(x, y)?;
            }
            eprintln!("[debug] action: recording scroll_right");
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
    draw_grid(pixels, width, height, input_state, dragging, true, backend)
}
