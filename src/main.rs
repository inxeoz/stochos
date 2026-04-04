mod app;
mod backend;
mod config;
mod input;
mod macro_store;
mod mode;
mod render;

use fs2::FileExt;

fn main() -> anyhow::Result<()> {
    const LOCK: &str = "/tmp/stochos.lock";
    if std::env::args().any(|a| a == "--new") {
        let _ = std::fs::remove_file(LOCK);
    }

    let f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(LOCK)?;
    if f.try_lock_exclusive().is_err() {
        eprintln!("App already running");
        std::process::exit(1);
    }

    std::panic::set_hook(Box::new(move |_| {
        let _ = (f.unlock(), std::fs::remove_file(LOCK));
    }));

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
