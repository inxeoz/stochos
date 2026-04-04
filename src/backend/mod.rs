use anyhow::Result;
use std::time::{Duration, Instant};

use crate::config::{config, Key, ScrollConfig};

/// A decoded key event, platform-agnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEvent {
    Char(char),
    Click,
    DoubleClick,
    RightClick,
    Close,
    Back,
    Undo,
    MacroMenu,
    MacroRecord,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollRepeat<KeyCode> {
    active: Option<ScrollRepeatState<KeyCode>>,
    delay: Duration,
    rate: u32,
    burst: Duration,
    enable_acceleration: bool,
    acceleration_time: Duration,
    initial_speed_multiplier: f64,
}

#[derive(Clone, Copy, Debug)]
struct ScrollRepeatState<KeyCode> {
    keycode: KeyCode,
    event: KeyEvent,
    next_at: Instant,
    stop_at: Instant,
    started_at: Instant,
    repeat_count: u32,
}

impl<KeyCode: Copy + Eq> ScrollRepeat<KeyCode> {
    pub fn from_config(config: &ScrollConfig) -> Self {
        Self {
            active: None,
            delay: Duration::from_millis(config.repeat_delay_ms),
            rate: config.repeat_rate,
            burst: Duration::from_millis(config.repeat_burst_ms),
            enable_acceleration: config.enable_acceleration,
            acceleration_time: Duration::from_secs_f64(config.acceleration_time_seconds),
            initial_speed_multiplier: config.initial_speed_multiplier,
        }
    }

    pub fn update_delay(&mut self, delay_ms: u64) {
        self.delay = Duration::from_millis(delay_ms);
    }

    pub fn update_rate(&mut self, rate: u32) {
        self.rate = rate;
    }

    pub fn schedule(&mut self, keycode: KeyCode, event: KeyEvent) {
        if !is_repeatable_key_event(event) || self.rate == 0 {
            self.active = None;
            return;
        }

        let now = Instant::now();
        self.active = Some(ScrollRepeatState {
            keycode,
            event,
            next_at: now + self.delay,
            stop_at: now + self.burst,
            started_at: now,
            repeat_count: 0,
        });
    }

    pub fn clear(&mut self, keycode: KeyCode) {
        if self.active.map(|repeat| repeat.keycode) == Some(keycode) {
            self.active = None;
        }
    }

    pub fn take_due(&mut self) -> Option<KeyEvent> {
        let repeat = self.active.as_mut()?;
        let now = Instant::now();
        if now >= repeat.stop_at {
            self.active = None;
            return None;
        }
        if now < repeat.next_at {
            return None;
        }

        // Extract config values to avoid borrowing self while active is mutably borrowed
        let enable_acceleration = self.enable_acceleration;
        let acceleration_time = self.acceleration_time;
        let rate = self.rate;
        let initial_speed_multiplier = self.initial_speed_multiplier;

        // Smooth acceleration: start at initial speed, gradually accelerate
        // This creates a natural feel similar to mouse scrolling
        let elapsed = now.duration_since(repeat.started_at);
        let interval = Self::calculate_scroll_interval_static(enable_acceleration, acceleration_time, rate, initial_speed_multiplier, elapsed);

        repeat.next_at = now + interval;
        repeat.repeat_count += 1;
        Some(repeat.event)
    }

    pub fn timeout(&self) -> Option<Duration> {
        let repeat = self.active?;
        Some(repeat.next_at.saturating_duration_since(Instant::now()))
    }

    pub fn is_same_key(&self, keycode: KeyCode, event: KeyEvent) -> bool {
        self.active
            .map(|repeat| repeat.keycode == keycode && repeat.event == event)
            .unwrap_or(false)
    }

    /// Calculate smooth scroll interval with acceleration curve
    /// Creates a natural scrolling feel by:
    /// 1. Starting at initial_speed_multiplier of target rate for first few repeats
    /// 2. Smoothly accelerating using easing curve over acceleration_time
    /// 3. Reaching full speed naturally, similar to mouse scroll behavior
    fn calculate_scroll_interval_static(
        enable_acceleration: bool,
        acceleration_time: Duration,
        rate: u32,
        initial_speed_multiplier: f64,
        elapsed: Duration,
    ) -> Duration {
        if rate == 0 {
            return Duration::from_secs(1);
        }

        let base_interval = 1.0 / rate as f64;

        if !enable_acceleration || elapsed >= acceleration_time {
            // Full speed reached or acceleration disabled
            return Duration::from_secs_f64(base_interval);
        }

        // Cubic ease-out curve: fast at start, slows down as it approaches target
        let progress = elapsed.as_secs_f64() / acceleration_time.as_secs_f64();
        let eased = 1.0 - (1.0 - progress).powi(3);

        // Interpolate from initial speed to full speed
        let speed_multiplier = initial_speed_multiplier + (1.0 - initial_speed_multiplier) * eased;
        Duration::from_secs_f64(base_interval / speed_multiplier)
    }
}

pub fn key_to_event(key: Key) -> Option<KeyEvent> {
    config().keys.to_event(key).or(match key {
        Key::Char(c) => Some(KeyEvent::Char(c)),
        _ => None,
    })
}

/// Platform backend — one implementation per OS/display-server.
///
/// `render.rs` produces a raw ARGB pixel buffer that every backend receives
/// unchanged via `present()`. All other methods are input/pointer control.
pub trait Backend {
    /// Screen dimensions in pixels.
    fn screen_size(&self) -> (u32, u32);

    /// Display a rendered ARGB8888 pixel buffer on the overlay.
    fn present(&mut self, pixels: &[u8], width: u32, height: u32) -> Result<()>;

    /// Move the mouse pointer to an absolute position.
    fn move_mouse(&mut self, x: u32, y: u32) -> Result<()>;

    /// Tear down the overlay, click at (x, y), then return.
    fn click(&mut self, x: u32, y: u32) -> Result<()>;

    /// Tear down the overlay, double click at (x, y), then return.
    fn double_click(&mut self, x: u32, y: u32) -> Result<()>;

    /// Tear down the overlay, right click at (x, y), then return.
    fn right_click(&mut self, x: u32, y: u32) -> Result<()>;

    /// Tear down the overlay, drag from (x1,y1) to (x2,y2), then return.
    fn drag_select(&mut self, x1: u32, y1: u32, x2: u32, y2: u32) -> Result<()>;

    /// Scroll the mouse wheel up.
    fn scroll_up(&mut self) -> Result<()>;

    /// Scroll the mouse wheel down.
    fn scroll_down(&mut self) -> Result<()>;

    /// Scroll the mouse wheel left (horizontal scroll).
    fn scroll_left(&mut self) -> Result<()>;

    /// Scroll the mouse wheel right (horizontal scroll).
    fn scroll_right(&mut self) -> Result<()>;

    /// Close the overlay without clicking.
    fn exit(&mut self) -> Result<()>;

    /// Block until the next key event. Returns None when the overlay closes.
    fn next_key(&mut self) -> Result<Option<KeyEvent>>;

    /// Recreate the overlay after a teardown (for macro recording).
    fn reopen(&mut self) -> Result<()>;
}

#[cfg(feature = "wayland")]
pub mod wayland;

#[cfg(feature = "x11")]
pub mod x11;

pub fn is_repeatable_key_event(event: KeyEvent) -> bool {
    matches!(
        event,
        KeyEvent::ScrollUp | KeyEvent::ScrollDown | KeyEvent::ScrollLeft | KeyEvent::ScrollRight
    )
}

pub mod keymap;
