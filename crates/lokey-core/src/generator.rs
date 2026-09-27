//! Random passwords for the generator dialog.
//!
//! Every chosen character set appears at least once, the rest is drawn from
//! all chosen sets together, and the result is shuffled so the guaranteed
//! characters are not always first. Each draw is uniform: `below` rejects the
//! top of the random range that would otherwise favour low indexes.

use zeroize::Zeroizing;

use crate::{Error, Result, crypto::random};

pub const MIN_LENGTH: usize = 8;
pub const MAX_LENGTH: usize = 128;

const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{}<>?,.;:";

/// Which kinds of character a generated password may contain.
#[derive(Clone, Copy, Debug)]
pub struct Charsets {
    pub upper: bool,
    pub lower: bool,
    pub digits: bool,
    pub symbols: bool,
}

impl Charsets {
    fn chosen(self) -> Vec<&'static [u8]> {
        [
            (self.upper, UPPER),
            (self.lower, LOWER),
            (self.digits, DIGITS),
            (self.symbols, SYMBOLS),
        ]
        .into_iter()
        .filter_map(|(on, set)| on.then_some(set))
        .collect()
    }

    /// Strength of a password of `length` drawn from these sets, in whole
    /// bits: `length × log2(pool size)`. The one-of-each guarantee costs a
    /// fraction of a bit, so this is a slight overestimate.
    pub fn bits(self, length: usize) -> u32 {
        let pool: usize = self.chosen().iter().map(|set| set.len()).sum();
        if pool == 0 {
            return 0;
        }
        (length as f64 * (pool as f64).log2()).floor() as u32
    }
}

/// A uniformly random index in `0..bound`. `bound` is at most `MAX_LENGTH`.
fn below(bound: usize) -> usize {
    let bound = bound as u64;
    let span = 1u64 << 32;
    // The largest multiple of `bound` that fits in a u32; draws at or above
    // it are thrown away so every index is equally likely.
    let limit = span - span % bound;
    loop {
        let draw = u64::from(u32::from_le_bytes(random::<4>()));
        if draw < limit {
            return (draw % bound) as usize;
        }
    }
}

/// Generates a password of `length` characters from `sets`.
pub fn generate(length: usize, sets: Charsets) -> Result<Zeroizing<String>> {
    if !(MIN_LENGTH..=MAX_LENGTH).contains(&length) {
        return Err(Error::Unsupported(
            "a generated password is 8 to 128 characters long",
        ));
    }
    let chosen = sets.chosen();
    if chosen.is_empty() {
        return Err(Error::Unsupported("choose at least one kind of character"));
    }
    let pool = chosen.concat();
    let mut chars: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(length));
    for set in &chosen {
        chars.push(set[below(set.len())]);
    }
    while chars.len() < length {
        chars.push(pool[below(pool.len())]);
    }
    // Fisher–Yates.
    for i in (1..chars.len()).rev() {
        chars.swap(i, below(i + 1));
    }
    Ok(Zeroizing::new(
        chars.iter().map(|&b| char::from(b)).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: Charsets = Charsets {
        upper: true,
        lower: true,
        digits: true,
        symbols: true,
    };
    const DIGITS_ONLY: Charsets = Charsets {
        upper: false,
        lower: false,
        digits: true,
        symbols: false,
    };
    const NONE: Charsets = Charsets {
        upper: false,
        lower: false,
        digits: false,
        symbols: false,
    };

    #[test]
    fn generate_returns_the_requested_length() {
        let password = generate(MAX_LENGTH, ALL).unwrap();

        assert_eq!(password.chars().count(), MAX_LENGTH);
    }

    #[test]
    fn generate_at_minimum_length_contains_every_chosen_set() {
        let password = generate(MIN_LENGTH, ALL).unwrap();

        assert!(password.bytes().any(|b| UPPER.contains(&b)));
        assert!(password.bytes().any(|b| LOWER.contains(&b)));
        assert!(password.bytes().any(|b| DIGITS.contains(&b)));
        assert!(password.bytes().any(|b| SYMBOLS.contains(&b)));
    }

    #[test]
    fn generate_never_uses_a_set_that_was_not_chosen() {
        let password = generate(MAX_LENGTH, DIGITS_ONLY).unwrap();

        assert!(password.bytes().all(|b| b.is_ascii_digit()));
    }

    #[test]
    fn generate_rejects_a_length_below_the_minimum() {
        assert!(matches!(
            generate(MIN_LENGTH - 1, ALL),
            Err(Error::Unsupported(_))
        ));
    }

    #[test]
    fn generate_rejects_a_length_above_the_maximum() {
        assert!(matches!(
            generate(MAX_LENGTH + 1, ALL),
            Err(Error::Unsupported(_))
        ));
    }

    #[test]
    fn generate_rejects_no_sets() {
        assert!(matches!(generate(20, NONE), Err(Error::Unsupported(_))));
    }

    #[test]
    fn below_stays_under_its_bound() {
        assert!((0..10_000).all(|_| below(3) < 3));
    }

    #[test]
    fn below_reaches_every_index() {
        let mut seen = [false; 5];

        (0..10_000).for_each(|_| seen[below(5)] = true);

        assert!(seen.iter().all(|&hit| hit));
    }

    #[test]
    fn bits_of_twenty_characters_from_all_sets_is_128() {
        assert_eq!(ALL.bits(20), 128);
    }

    #[test]
    fn bits_of_no_sets_is_zero() {
        assert_eq!(NONE.bits(20), 0);
    }
}
