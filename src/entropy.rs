//! Platform entropy, in the `rand_core` 0.10 vocabulary.
//!
//! Two sources with deliberately different type-level claims:
//!
//! - [`SystemEntropy`] — the operating environment's CSPRNG (`getrandom`:
//!   OS on native, `wasi:random` on WASI, `crypto.getRandomValues` in the
//!   browser). Implements [`CryptoRng`], so it satisfies the
//!   `CryptoRngCore` bound (`Rng + CryptoRng`) that `aloecrypt_core` and
//!   friends take — hand it to anything that needs real randomness.
//! - [`SeededEntropy`] — deterministic, seedable, replayable. Implements
//!   [`Rng`] but **not** [`CryptoRng`], on purpose: a replay harness or test
//!   can substitute it anywhere plain randomness is accepted (jitter, ids,
//!   sampling), and the compiler refuses it anywhere cryptographic
//!   randomness is required. Determinism must never be able to masquerade
//!   as entropy.
//!
//! The API discipline this splits into for consumers: take
//! `&mut impl CryptoRngCore` where the randomness is security-relevant, and
//! `&mut impl Rng` where it only needs to be arbitrary — the second is the
//! seam replay and tests substitute through.
//!
//! ```
//! use ego_platform::entropy::{CryptoRngCore, SeededEntropy, SystemEntropy};
//! use rand_core::Rng;
//!
//! fn keygen(rng: &mut impl CryptoRngCore) -> [u8; 32] {
//!     let mut key = [0u8; 32];
//!     rng.fill_bytes(&mut key);
//!     key
//! }
//! fn jitter(rng: &mut impl Rng) -> u64 {
//!     rng.next_u64() % 100
//! }
//!
//! let mut sys = SystemEntropy;
//! let _key = keygen(&mut sys);
//! let mut replay = SeededEntropy::from_u64(42);
//! let _j = jitter(&mut replay); // deterministic
//! let _j = jitter(&mut sys); // also fine
//! // keygen(&mut replay) does not compile: SeededEntropy is not CryptoRng.
//! ```

use rand_chacha::ChaCha20Rng;
use rand_core::{CryptoRng, Infallible, Rng, SeedableRng, TryCryptoRng, TryRng};

/// `Rng + CryptoRng`, as one nameable bound.
///
/// The same shape (and blanket) as `aloecrypt_core::rng::CryptoRngCore`, so
/// the two are satisfied by exactly the same types; ego-platform defines its
/// own rather than depending on a crypto crate from the platform layer.
pub trait CryptoRngCore: Rng + CryptoRng {}
impl<T: Rng + CryptoRng> CryptoRngCore for T {}

/// Fill `dest` from the platform CSPRNG.
///
/// # Panics
///
/// Panics if the platform entropy source fails, which on every supported
/// target means the environment itself is broken — the same posture as
/// `rand_core`'s `OsRng` unwrapping. Code that must survive that failure
/// uses `getrandom` directly.
pub fn fill(dest: &mut [u8]) {
    getrandom::getrandom(dest).expect("platform entropy source failed");
}

/// The operating environment's CSPRNG. Zero-sized; make one anywhere.
///
/// Backed by `getrandom` on every target. Marked [`CryptoRng`], so it
/// satisfies [`CryptoRngCore`] and can seed downstream generators (e.g.
/// `AloeRng::from_rng(&mut SystemEntropy)`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemEntropy;

// rand_core 0.10 blankets `Rng` over `TryRng<Error = Infallible>` and
// `CryptoRng` over `TryCryptoRng<Error = Infallible>`, so the Try pair is
// what a source implements -- the same route `aloecrypt_core`'s AloeRng
// takes.
impl TryRng for SystemEntropy {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        let mut b = [0u8; 4];
        fill(&mut b);
        Ok(u32::from_le_bytes(b))
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        let mut b = [0u8; 8];
        fill(&mut b);
        Ok(u64::from_le_bytes(b))
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Infallible> {
        fill(dest);
        Ok(())
    }
}

impl TryCryptoRng for SystemEntropy {}

/// A deterministic, seedable generator that cannot pose as entropy.
///
/// ChaCha20 underneath (the same core `aloecrypt_core`'s `AloeRng` carries),
/// but deliberately **not** marked [`CryptoRng`]: its whole point is to be
/// substitutable where arbitrary randomness is accepted — replay, tests,
/// reproducible simulation — and refused by the compiler where cryptographic
/// randomness is demanded.
#[derive(Debug, Clone)]
pub struct SeededEntropy(ChaCha20Rng);

impl SeededEntropy {
    /// A generator that will replay the same sequence for the same seed.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self(ChaCha20Rng::from_seed(seed))
    }

    /// Convenience over [`SeededEntropy::from_seed`] for tests.
    pub fn from_u64(seed: u64) -> Self {
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&seed.to_le_bytes());
        Self(ChaCha20Rng::from_seed(bytes))
    }
}

// TryRng but deliberately NOT TryCryptoRng: the blankets then grant `Rng`
// and withhold `CryptoRng`, which is the whole mechanism.
impl TryRng for SeededEntropy {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        Ok(self.0.next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        Ok(self.0.next_u64())
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Infallible> {
        self.0.fill_bytes(dest);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_entropy_produces_bytes() {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        SystemEntropy.fill_bytes(&mut a);
        SystemEntropy.fill_bytes(&mut b);
        assert_ne!(a, [0u8; 32], "all-zero entropy is a broken source");
        assert_ne!(a, b, "two draws agreeing at 256 bits is not chance");
    }

    #[test]
    fn seeded_entropy_replays() {
        let mut x = SeededEntropy::from_u64(7);
        let mut y = SeededEntropy::from_u64(7);
        let mut z = SeededEntropy::from_u64(8);
        assert_eq!(x.next_u64(), y.next_u64());
        assert_ne!(x.next_u64(), z.next_u64());
    }

    #[test]
    fn system_entropy_satisfies_the_crypto_bound() {
        fn takes_crypto(_r: &mut impl CryptoRngCore) {}
        takes_crypto(&mut SystemEntropy);
        // SeededEntropy deliberately does NOT compile here; see the module
        // docs. (A compile-fail test would pin that, but the absence of the
        // CryptoRng impl is the whole of the mechanism.)
    }
}
