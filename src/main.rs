use std::time::{SystemTime, UNIX_EPOCH};

use smithay_client_toolkit::output;
use wayland_client::{globals::registry_queue_init, protocol::wl_pointer::ButtonState, Connection};
use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1;

mod state;
use state::State;

static BTN_LEFT: u32 = 0x110;

fn main() -> anyhow::Result<()> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init::<State>(&conn)?;
    let qh = event_queue.handle();

    let _ = output::OutputState::new(&globals, &qh);
    let manager: ZwlrVirtualPointerManagerV1 = globals.bind(&qh, 1..=2, ())?;

    let v_pointer = manager.create_virtual_pointer(None, &qh, ());

    v_pointer.motion_absolute(timestamp(), 100, 100, 1920, 1080);
    v_pointer.frame();

    v_pointer.button(timestamp(), BTN_LEFT, ButtonState::Pressed);
    v_pointer.frame();

    v_pointer.button(timestamp(), BTN_LEFT, ButtonState::Released);
    v_pointer.frame();

    conn.flush()?;

    event_queue.roundtrip(&mut State)?;

    v_pointer.destroy();
    manager.destroy();

    Ok(())
}

fn timestamp() -> u32 {
    return SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32;
}
