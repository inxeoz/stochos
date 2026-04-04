mod app;
mod backend;
mod config;
mod input;
mod macro_store;
mod mode;
mod render;

use nix::fcntl::{Flock, FlockArg};
use std::fs::OpenOptions;

struct LockGuard {
    _lock: Flock<std::fs::File>, // RAII → lock held for lifetime
}

fn acquire_lock() -> anyhow::Result<Option<LockGuard>> {
    let allow_multiple = std::env::args().any(|a| a == "--allow-multiple");
    if allow_multiple {
        return Ok(None);
    }

    let lock_path = "/tmp/stochos.lock";

    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(lock_path)?;

    let lock = match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
        Ok(lock) => lock,

        Err((_, nix::errno::Errno::EWOULDBLOCK)) => {
            eprintln!("App already running (use --allow-multiple to override)");
            std::process::exit(1);
        }

        Err((_, e)) => return Err(anyhow::anyhow!(e)),
    };

    Ok(Some(LockGuard { _lock: lock }))
}

fn main() -> anyhow::Result<()> {
    let _lock = acquire_lock()?; // keep lock alive

    config::init();

    #[cfg(feature = "wayland")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        if let Ok(mut b) = backend::wayland::WaylandBackend::new() {
            return app::run(&mut b);
        }
    }

    #[cfg(feature = "x11")]
    if std::env::var_os("DISPLAY").is_some() {
        let mut b = backend::x11::X11Backend::new()?;
        return app::run(&mut b);
    }

    anyhow::bail!("no display server found (need WAYLAND_DISPLAY or DISPLAY)")
}
