//! Shared helper for the incremental scanners below: streamed chunks can split a
//! multi-byte UTF-8 character across two `feed()` calls, so any byte-offset math has to be
//! clamped back to a valid `char` boundary before slicing.

pub fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
