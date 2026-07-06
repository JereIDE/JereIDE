use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

use super::font::{
    self, Antialiasing, FontInner, FontRef, GlyphInfo, Hinting, register_font, set_skip_prewarm,
};

/// Maximum number of (size, antialiasing, hinting) fallback variants kept
/// loaded at once. Oldest-loaded is evicted; its glyph caches go with it.
const MAX_VARIANTS: usize = 8;

type VariantKey = (u32, Antialiasing, Hinting);

/// A fallback font candidate: loaded on first use, never retried on failure.
enum Slot {
    Unloaded(PathBuf),
    Loaded(FontRef),
    Failed,
}

thread_local! {
    /// System fallback font slots, lazily loaded per (size, aa, hinting) variant.
    static VARIANTS: RefCell<Vec<(VariantKey, Vec<Slot>)>> = const { RefCell::new(Vec::new()) };
    /// Codepoints no font covered, deduped for the session.
    static UNCOVERED_SEEN: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
    /// Newly seen uncovered codepoints since the last `take_uncovered` drain.
    static UNCOVERED_NEW: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

/// Look up `codepoint` in the installed system fallback fonts at the given
/// size and rendering options, loading candidates on demand. Returns the
/// glyph and the chosen font's height, or None when no installed fallback
/// font covers the codepoint.
pub(crate) fn system_glyph(
    codepoint: u32,
    size: f32,
    antialiasing: Antialiasing,
    hinting: Hinting,
) -> Option<(GlyphInfo, i32)> {
    let key = (size.to_bits(), antialiasing, hinting);
    VARIANTS.with(|cell| {
        let mut variants = cell.borrow_mut();
        let idx = match variants.iter().position(|(k, _)| *k == key) {
            Some(i) => i,
            None => {
                if variants.len() >= MAX_VARIANTS {
                    variants.pop();
                }
                let slots = candidate_paths().into_iter().map(Slot::Unloaded).collect();
                variants.insert(0, (key, slots));
                0
            }
        };
        for slot in &mut variants[idx].1 {
            if let Slot::Unloaded(path) = slot {
                *slot = match load_font(path, size, antialiasing, hinting) {
                    Some(font) => Slot::Loaded(font),
                    None => Slot::Failed,
                };
            }
            let Slot::Loaded(font) = slot else { continue };
            let mut f = font.lock();
            let info = f.get_glyph(codepoint);
            if info.defined && (info.bitmap.is_some() || info.xadvance > 0.0) {
                return Some((info.clone(), f.height));
            }
        }
        None
    })
}

/// Record a codepoint that no configured or system font covers.
pub(crate) fn note_uncovered(codepoint: u32) {
    UNCOVERED_SEEN.with(|seen| {
        if seen.borrow_mut().insert(codepoint) {
            UNCOVERED_NEW.with(|new| new.borrow_mut().push(codepoint));
        }
    });
}

/// Drain the uncovered codepoints recorded since the last call.
pub fn take_uncovered() -> Vec<u32> {
    UNCOVERED_NEW.with(|new| std::mem::take(&mut *new.borrow_mut()))
}

/// Load one candidate at the given size and options; None when the file is
/// absent, unreadable, or malformed.
fn load_font(
    path: &std::path::Path,
    size: f32,
    aa: Antialiasing,
    hinting: Hinting,
) -> Option<FontRef> {
    if !path.is_file() {
        return None;
    }
    let prev = font::skip_prewarm();
    set_skip_prewarm(true);
    let loaded = FontInner::load(path.to_str()?, size, aa, hinting);
    set_skip_prewarm(prev);
    let font = Arc::new(Mutex::new(loaded.ok()?));
    register_font(&font);
    Some(font)
}

/// System fonts with broad script coverage, in priority order.
#[cfg(target_os = "windows")]
fn candidate_paths() -> Vec<PathBuf> {
    let dir = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\Windows"))
        .join("Fonts");
    [
        "msyh.ttc",     // Microsoft YaHei - Simplified Chinese (Win 8+)
        "msyh.ttf",     // Microsoft YaHei - Simplified Chinese (Win 7)
        "msjh.ttc",     // Microsoft JhengHei - Traditional Chinese (Win 8+)
        "msjh.ttf",     // Microsoft JhengHei - Traditional Chinese (Win 7)
        "YuGothM.ttc",  // Yu Gothic Medium - Japanese
        "msgothic.ttc", // MS Gothic - Japanese
        "malgun.ttf",   // Malgun Gothic - Korean
        "simsun.ttc",   // SimSun - Simplified Chinese
        "segoeui.ttf",  // Segoe UI - Arabic, Hebrew, Greek, Cyrillic, Armenian, Georgian
        "leelawui.ttf", // Leelawadee UI - Thai, Lao, Khmer, Buginese
        "Nirmala.ttc",  // Nirmala UI - Devanagari, Bengali, Tamil and other Indic scripts
        "ebrima.ttf",   // Ebrima - Ethiopic, N'Ko, Tifinagh, Vai, Adlam
        "micross.ttf",  // Microsoft Sans Serif - legacy broad coverage
        "seguisym.ttf", // Segoe UI Symbol
    ]
    .into_iter()
    .map(|name| dir.join(name))
    .collect()
}

/// System fonts with broad script coverage, in priority order.
#[cfg(target_os = "macos")]
fn candidate_paths() -> Vec<PathBuf> {
    [
        "/System/Library/Fonts/PingFang.ttc",              // Chinese
        "/System/Library/Fonts/Hiragino Sans GB.ttc",      // Simplified Chinese
        "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc", // Hiragino Kaku Gothic - Japanese
        "/System/Library/Fonts/AppleSDGothicNeo.ttc",      // Korean
        "/System/Library/Fonts/Thonburi.ttc",              // Thai
        "/System/Library/Fonts/GeezaPro.ttc",              // Arabic
        "/System/Library/Fonts/Supplemental/Khmer MN.ttc", // Khmer (not in Arial Unicode)
        "/System/Library/Fonts/Supplemental/Lao MN.ttc",   // Lao
        "/System/Library/Fonts/Supplemental/Devanagari Sangam MN.ttc", // Devanagari
        "/System/Library/Fonts/Supplemental/Tamil Sangam MN.ttc", // Tamil
        "/System/Library/Fonts/Supplemental/Kefa.ttc",     // Ethiopic (not in Arial Unicode)
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf", // broad coverage incl. kana
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

/// System fonts with broad script coverage, in priority order: the Noto CJK
/// collections, every per-script Noto Sans face installed, then the legacy
/// broad-coverage faces.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn candidate_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", // Debian/Ubuntu
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",      // Arch
        "/usr/share/fonts/google-noto-sans-cjk-vf-fonts/NotoSansCJK-VF.ttc", // Fedora
        "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();
    paths.extend(noto_script_fonts());
    paths.extend(
        [
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
            "/usr/share/fonts/wenquanyi/wqy-zenhei/wqy-zenhei.ttc",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", // symbols, extended Latin
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
        ]
        .into_iter()
        .map(PathBuf::from),
    );
    paths
}

/// Per-script `Noto*-Regular.ttf` faces (Thai, Arabic, Hebrew, Devanagari,
/// ...) from the distros' Noto directories. `NotoSans` faces are preferred
/// over stylistic families (Looped, Kufi, Rashi, ...); order is
/// deterministic.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn noto_script_fonts() -> Vec<PathBuf> {
    let dirs = [
        "/usr/share/fonts/truetype/noto", // Debian/Ubuntu
        "/usr/share/fonts/noto",          // Arch/Fedora
    ];
    let mut fonts: Vec<PathBuf> = dirs
        .into_iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("Noto") && n.ends_with("-Regular.ttf"))
        })
        .collect();
    fonts.sort_by_key(|p| {
        let name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        (!name.starts_with("NotoSans"), name)
    });
    fonts
}
