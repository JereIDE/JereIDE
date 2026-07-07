/// Round half-away-from-zero.
pub fn round(n: f64) -> f64 {
    if n >= 0.0 {
        (n + 0.5).floor()
    } else {
        (n - 0.5).ceil()
    }
}

// ── Fuzzy match ───────────────────────────────────────────────────────────────

/// Port of the C `f_fuzzy_match` algorithm.
/// Returns `None` if needle is not a subsequence of haystack.
/// Returns `Some(score)` otherwise (higher = better match).
/// When `files=true`, matches backwards for better filename relevance.
pub fn fuzzy_match(haystack: &str, needle: &str, files: bool) -> Option<i64> {
    let hb = haystack.as_bytes();
    let nb = needle.as_bytes();
    let h_len = hb.len();
    let n_len = nb.len();
    if n_len == 0 {
        return Some(-(h_len as i64) * 10);
    }
    let mut score: i64 = 0;
    let mut run: i64 = 0;
    let mut hi: isize = if files { h_len as isize - 1 } else { 0 };
    let mut ni: isize = if files { n_len as isize - 1 } else { 0 };
    let step: isize = if files { -1 } else { 1 };
    let in_h = |i: isize| i >= 0 && i < h_len as isize;
    let in_n = |i: isize| i >= 0 && i < n_len as isize;
    while in_h(hi) && in_n(ni) {
        while in_h(hi) && hb[hi as usize] == b' ' {
            hi += step;
        }
        while in_n(ni) && nb[ni as usize] == b' ' {
            ni += step;
        }
        if !in_h(hi) || !in_n(ni) {
            break;
        }
        let hc = hb[hi as usize];
        let nc = nb[ni as usize];
        if hc.eq_ignore_ascii_case(&nc) {
            score += run * 10 - if hc != nc { 1 } else { 0 };
            run += 1;
            ni += step;
        } else {
            score -= 10;
            run = 0;
        }
        hi += step;
    }
    if in_n(ni) {
        return None;
    }
    Some(score - h_len as i64 * 10)
}

// ── Path compare ──────────────────────────────────────────────────────────────

/// Port of the C `f_path_compare` natural-sort comparison.
/// Returns `true` if path1 should sort before path2.
/// Directories sort before files; numeric segments use natural ordering.
pub fn path_compare(path1: &str, type1: &str, path2: &str, type2: &str) -> bool {
    const SEP: u8 = b'/';
    let p1 = path1.as_bytes();
    let p2 = path2.as_bytes();
    let len1 = p1.len();
    let len2 = p2.len();
    let mut t1: i32 = if type1 != "dir" { 1 } else { 0 };
    let mut t2: i32 = if type2 != "dir" { 1 } else { 0 };
    let mut offset = 0usize;
    for k in 0..len1.min(len2) {
        if p1[k] != p2[k] {
            break;
        }
        if p1[k] == SEP {
            offset = k + 1;
        }
    }
    if p1[offset..].contains(&SEP) {
        t1 = 0;
    }
    if p2[offset..].contains(&SEP) {
        t2 = 0;
    }
    if t1 != t2 {
        return t1 < t2;
    }
    let same_len = len1 == len2;
    let mut cfr: i32 = -1;
    let mut i = offset;
    let mut j = offset;
    loop {
        if i > len1 || j > len2 {
            break;
        }
        let a = if i < len1 { p1[i] } else { 0u8 };
        let b = if j < len2 { p2[j] } else { 0u8 };
        if a == 0 || b == 0 {
            if cfr < 0 {
                cfr = 0;
            }
            if !same_len {
                cfr = if a == 0 { 1 } else { 0 };
            }
            break;
        }
        if a.is_ascii_digit() && b.is_ascii_digit() {
            let mut ii = 0;
            while i + ii < len1 && p1[i + ii].is_ascii_digit() {
                ii += 1;
            }
            let mut ij = 0;
            while j + ij < len2 && p2[j + ij].is_ascii_digit() {
                ij += 1;
            }
            let mut di: u64 = 0;
            for k in 0..ii {
                di = di
                    .saturating_mul(10)
                    .saturating_add((p1[i + k] - b'0') as u64);
            }
            let mut dj: u64 = 0;
            for k in 0..ij {
                dj = dj
                    .saturating_mul(10)
                    .saturating_add((p2[j + k] - b'0') as u64);
            }
            if di != dj {
                cfr = if di < dj { 1 } else { 0 };
                break;
            }
            i += 1;
            j += 1;
            continue;
        }
        if a == b {
            i += 1;
            j += 1;
            continue;
        }
        if a == SEP || b == SEP {
            cfr = if a == SEP { 1 } else { 0 };
            break;
        }
        let al = a.to_ascii_lowercase();
        let bl = b.to_ascii_lowercase();
        if al == bl {
            if same_len && cfr < 0 {
                cfr = if a > b { 1 } else { 0 };
            }
            i += 1;
            j += 1;
            continue;
        }
        cfr = if al < bl { 1 } else { 0 };
        break;
    }
    cfr != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_positive() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(2.4), 2.0);
    }

    #[test]
    fn round_negative() {
        assert_eq!(round(-2.5), -3.0);
        assert_eq!(round(-2.4), -2.0);
    }
}
