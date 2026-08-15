//! Simple monotonic id generator, e.g. `call_lz3k2n1_1`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn to_base36(mut n: u128) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

pub fn generate_id(prefix: &str) -> String {
    let count = COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    format!("{prefix}_{}_{count}", to_base36(millis))
}
