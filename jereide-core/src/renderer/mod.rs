mod cache;
mod fallback;
pub(crate) mod font;

pub(crate) use cache::{RenColor, RenRect, group_text_width};
pub(crate) use fallback::take_uncovered;
pub(crate) use font::{Antialiasing, FontInner, FontRef, Hinting};

use cache::RenCache;
use sdl3_sys::everything::*;

// ── Thread-local renderer state ───────────────────────────────────────────────

thread_local! {
    static CACHE: std::cell::RefCell<Option<RenCache>> =
        const { std::cell::RefCell::new(None) };
}

pub(crate) fn with_cache<F: FnOnce(&mut RenCache)>(f: F) {
    CACHE.with(|c| {
        let mut borrow = c.borrow_mut();
        if borrow.is_none() {
            *borrow = Some(RenCache::new());
        }
        f(borrow.as_mut().unwrap());
    });
}

/// Push a draw_text command directly to the thread-local cache.
/// Returns the new x position after the text.
#[allow(non_snake_case)]
pub fn CACHE_DRAW_TEXT(
    fonts: std::sync::Arc<[FontRef]>,
    text: &str,
    x: f32,
    y: i32,
    color: RenColor,
    tab_offset: f32,
) -> f32 {
    CACHE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if borrow.is_none() {
            *borrow = Some(RenCache::new());
        }
        borrow
            .as_mut()
            .unwrap()
            .push_draw_text(fonts, text, x, y, color, tab_offset)
    })
}

/// Native begin_frame: initialize the render cache for a new frame.
pub fn native_begin_frame() {
    let (w, h) = crate::window::get_drawable_size();
    with_cache(|c| {
        if crate::window::take_needs_invalidate() {
            c.invalidate();
        }
        c.begin_frame(w, h);
    });
}

// Canonical render target: a private `RGBA32` surface at the window's physical
// resolution. The window surface's pixel format differs per platform (BGRA on
// Windows/Linux, RGBA on macOS) and its native resolution may also differ from
// `SDL_GetWindowSizeInPixels`, which previously caused an R/B channel swap
// (everything pink/blue) and inconsistent scaling on non-macOS backends. By
// always drawing into our own RGBA32 surface and blitting it to the window
// surface, SDL performs the format conversion and any needed scaling, so the
// result is identical on every platform.
thread_local! {
    static OFFSCREEN: std::cell::RefCell<Option<*mut SDL_Surface>> =
        const { std::cell::RefCell::new(None) };
    static OFFSCREEN_SIZE: std::cell::RefCell<(i32, i32)> =
        const { std::cell::RefCell::new((0, 0)) };
}

/// Return a private `RGBA32` surface sized to `(w, h)`, recreating it only when
/// the size changes. Caller must not store the pointer across frames; it is
/// owned by this module and destroyed on resize / `destroy_offscreen`.
unsafe fn ensure_offscreen(w: i32, h: i32) -> *mut SDL_Surface {
    OFFSCREEN.with(|os| {
        let mut guard = os.borrow_mut();
        let size = OFFSCREEN_SIZE.with(|s| *s.borrow());
        let mismatch = guard.is_none() || size != (w, h);
        if mismatch {
            if let Some(old) = guard.take() {
                unsafe { SDL_DestroySurface(old) };
            }
            let surf = unsafe { SDL_CreateSurface(w.max(1), h.max(1), SDL_PIXELFORMAT_RGBA32) };
            *guard = if surf.is_null() { None } else { Some(surf) };
            OFFSCREEN_SIZE.with(|s| *s.borrow_mut() = (w, h));
        }
        guard.unwrap_or(std::ptr::null_mut())
    })
}

/// Drop the offscreen surface (e.g. on cache teardown).
pub fn destroy_offscreen() {
    OFFSCREEN.with(|os| {
        if let Some(old) = os.borrow_mut().take() {
            unsafe { SDL_DestroySurface(old) };
        }
        OFFSCREEN_SIZE.with(|s| *s.borrow_mut() = (0, 0));
    });
}

/// Native end_frame: compute dirty rects and render to the SDL surface.
pub fn native_end_frame() {
    CACHE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(cache) = borrow.as_mut() else { return };
        let dirty = cache.compute_dirty_rects();
        if dirty.is_empty() {
            return;
        }
        let commands = &cache.commands;
        let arena = &cache.text_arena;
        crate::window::with_window_surface(|win_surface, window| {
            let (pw, ph) = crate::window::get_drawable_size();
            // SAFETY: surfaces are valid for this call; we're on the main thread.
            unsafe {
                let off = ensure_offscreen(pw, ph);
                if off.is_null() {
                    // Fallback: draw straight to the window surface (legacy path).
                    cache::render_dirty_rects(win_surface, commands, arena, &dirty);
                } else {
                    cache::render_dirty_rects(off, commands, arena, &dirty);
                    // Convert RGBA->window format and scale to the window's
                    // native buffer in one blit. Null rects = full-surface copy.
                    SDL_BlitSurfaceScaled(
                        off,
                        std::ptr::null(),
                        win_surface,
                        std::ptr::null(),
                        SDL_ScaleMode::PIXELART,
                    );
                }
                // Update the whole window surface to avoid partial-update seams
                // when the blit scales between offscreen and window resolutions.
                let full = SDL_Rect {
                    x: 0,
                    y: 0,
                    w: (*win_surface).w,
                    h: (*win_surface).h,
                };
                SDL_UpdateWindowSurfaceRects(window, &full, 1);
            }
            crate::window::show_if_hidden();
        });
    });
}


/// Drop per-window caches that are cheap to rebuild on next draw.
/// Called when the window is occluded/hidden so we don't hold onto
/// megabytes of glyph bitmaps and render-cache command buffers while
/// the compositor isn't showing our frames.
pub fn drop_caches() {
    CACHE.with(|c| {
        *c.borrow_mut() = None;
    });
    destroy_offscreen();
    font::clear_glyph_caches();
}

/// macOS memory-pressure level.  `Some(0)` normal, `Some(1)` warn,
/// `Some(2)` critical, `None` when the sysctl isn't available (non-
/// macOS or the node doesn't exist on the running kernel).
#[cfg(target_os = "macos")]
pub fn macos_memory_pressure_level() -> Option<u32> {
    use std::ffi::CString;
    let name = CString::new("kern.memorystatus_vm_pressure_level").ok()?;
    let mut value: u32 = 0;
    let mut size: libc::size_t = std::mem::size_of::<u32>();
    // SAFETY: `name` is a NUL-terminated C string we just created;
    // `value` and `size` are valid for reads/writes of sizeof(u32).
    let rc = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            &mut value as *mut u32 as *mut libc::c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc == 0 { Some(value) } else { None }
}

#[cfg(not(target_os = "macos"))]
pub fn macos_memory_pressure_level() -> Option<u32> {
    None
}
