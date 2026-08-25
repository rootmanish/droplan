//! Cryptographically secure token, id and PIN generation.
//!
//! All randomness comes from the operating system CSPRNG via `getrandom`.
//! Nothing here uses a seeded or thread-local PRNG.

use crate::error::{Error, Result};

/// Unambiguous alphabet: no `0`/`O`, `1`/`l`/`I`. 56 symbols, ~5.8 bits each.
const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";

/// Length of a share-session token. 22 symbols ~= 127 bits of entropy.
pub const SESSION_TOKEN_LEN: usize = 22;

/// Length of an opaque per-file id. Unguessable on its own, and additionally
/// only reachable behind a valid session token.
pub const FILE_ID_LEN: usize = 16;

/// Length of the cookie value handed out after a successful PIN unlock.
pub const UNLOCK_SECRET_LEN: usize = 24;

fn fill_random(buf: &mut [u8]) -> Result<()> {
    getrandom::fill(buf).map_err(|err| {
        tracing::error!(target: "droplan", "OS randomness unavailable: {err}");
        Error::Entropy
    })
}

/// Uniformly random string over [`ALPHABET`].
///
/// Uses rejection sampling so every symbol is equally likely; a naive `% 56`
/// would bias the first 32 symbols of the alphabet.
pub fn random_token(len: usize) -> Result<String> {
    // Largest multiple of the alphabet size that fits in a byte.
    let limit = (u8::MAX as usize + 1) - ((u8::MAX as usize + 1) % ALPHABET.len());
    let mut out = String::with_capacity(len);
    let mut buf = vec![0u8; len.saturating_mul(2).max(32)];

    while out.len() < len {
        fill_random(&mut buf)?;
        for &byte in buf.iter() {
            if (byte as usize) < limit {
                let idx = byte as usize % ALPHABET.len();
                out.push(ALPHABET[idx] as char);
                if out.len() == len {
                    break;
                }
            }
        }
    }
    Ok(out)
}

pub fn session_token() -> Result<String> {
    random_token(SESSION_TOKEN_LEN)
}

pub fn file_id() -> Result<String> {
    random_token(FILE_ID_LEN)
}

pub fn unlock_secret() -> Result<String> {
    random_token(UNLOCK_SECRET_LEN)
}

/// Numeric PIN, uniformly distributed, leading zeros preserved.
pub fn random_pin(digits: usize) -> Result<String> {
    let limit = (u8::MAX as usize + 1) - ((u8::MAX as usize + 1) % 10);
    let mut out = String::with_capacity(digits);
    let mut buf = vec![0u8; digits.saturating_mul(2).max(16)];

    while out.len() < digits {
        fill_random(&mut buf)?;
        for &byte in buf.iter() {
            if (byte as usize) < limit {
                out.push(char::from(b'0' + (byte % 10)));
                if out.len() == digits {
                    break;
                }
            }
        }
    }
    Ok(out)
}

/// Comparison whose running time does not depend on where the first
/// difference is, so a caller cannot probe a secret one byte at a time.
///
/// Length is compared up front, which is fine: token lengths are public.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn tokens_have_requested_length_and_alphabet() {
        for len in [1usize, 8, 22, 64] {
            let token = random_token(len).expect("rng");
            assert_eq!(token.chars().count(), len);
            assert!(
                token.bytes().all(|b| ALPHABET.contains(&b)),
                "unexpected symbol in {token}"
            );
        }
    }

    #[test]
    fn tokens_do_not_repeat() {
        let mut seen = HashSet::new();
        for _ in 0..512 {
            assert!(
                seen.insert(session_token().expect("rng")),
                "duplicate session token"
            );
        }
    }

    #[test]
    fn pins_are_numeric_and_padded() {
        for _ in 0..64 {
            let pin = random_pin(6).expect("rng");
            assert_eq!(pin.len(), 6);
            assert!(pin.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn constant_time_eq_matches_normal_equality() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
        assert!(!constant_time_eq("", "a"));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn alphabet_excludes_confusable_symbols() {
        for bad in *b"0O1lI" {
            assert!(
                !ALPHABET.contains(&bad),
                "{} should not be in the alphabet",
                bad as char
            );
        }
    }
}
