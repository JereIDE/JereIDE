use parking_lot::Mutex;
use std::sync::LazyLock;

// ── Clip rect stack ─────────────────────────────────────────────────────────

static CLIP_STACK: LazyLock<Mutex<Vec<[f64; 4]>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Initialize the clip stack with a full-screen rect.
pub fn clip_init(w: f64, h: f64) {
    let mut stack = CLIP_STACK.lock();
    stack.clear();
    stack.push([0.0, 0.0, w, h]);
    #[cfg(feature = "sdl")]
    crate::renderer::with_cache(|c| {
        c.push_set_clip(crate::renderer::RenRect {
            x: 0,
            y: 0,
            w: w as i32,
            h: h as i32,
        });
    });
}
