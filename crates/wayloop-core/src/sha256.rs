//! SHA-256 (FIPS 180-4), written by hand so that the core needs no hashing crate (SPEC-0001
//! plan, D3).
//!
//! The hash identifies content and protects no secret, so the implementation makes no attempt
//! at constant time. It is checked against the published test vectors and, on random inputs,
//! against a hashing crate used in tests only.

use wayloop_schemas::InputHash;

/// Length of a digest, in bytes: the digest an input hash is written from.
const DIGEST_LEN: usize = InputHash::DIGEST_LEN;

/// Length of a block, in bytes.
const BLOCK_LEN: usize = 64;

/// Length of the message length that ends the padding, in bytes.
const LENGTH_LEN: usize = 8;

/// Initial hash value: the first 32 bits of the fractional parts of the square roots of the
/// first eight primes (FIPS 180-4, 5.3.3).
const INITIAL: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// Round constants: the first 32 bits of the fractional parts of the cube roots of the first
/// sixty-four primes (FIPS 180-4, 4.2.2).
const ROUND: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// The SHA-256 digest of `message`.
///
/// FIPS 180-4 defines SHA-256 for messages shorter than 2^64 bits (2^61 bytes). The length is
/// computed in 128 bits, so it cannot overflow, and written in the padding modulo 2^64 bits as
/// the standard lays it out; a message of 2^61 bytes or more, outside the domain of SHA-256,
/// cannot be held in memory on any target Rust supports.
#[must_use]
pub fn sha256(message: &[u8]) -> [u8; DIGEST_LEN] {
    let mut state = INITIAL;
    let blocks = message.chunks_exact(BLOCK_LEN);
    let rest = blocks.remainder();
    for block in blocks {
        compress(&mut state, block);
    }

    // Padding (FIPS 180-4, 5.1.1): the rest of the message, the bit 1, zeros, then the length
    // in bits as a 64-bit big-endian number, in one block or two.
    let mut tail = [0u8; 2 * BLOCK_LEN];
    for (to, from) in tail.iter_mut().zip(rest) {
        *to = *from;
    }
    if let Some(marker) = tail.get_mut(rest.len()) {
        *marker = 0x80;
    }
    let tail_len = if rest.len() < BLOCK_LEN - LENGTH_LEN {
        BLOCK_LEN
    } else {
        2 * BLOCK_LEN
    };
    let bits = (message.len() as u128) * 8;
    let bits = bits.to_be_bytes();
    let low_bits = bits.get(bits.len() - LENGTH_LEN..).unwrap_or_default();
    if let Some(field) = tail.get_mut(tail_len - LENGTH_LEN..tail_len) {
        for (to, from) in field.iter_mut().zip(low_bits) {
            *to = *from;
        }
    }
    for block in tail.chunks_exact(BLOCK_LEN).take(tail_len / BLOCK_LEN) {
        compress(&mut state, block);
    }

    let mut digest = [0u8; DIGEST_LEN];
    for (bytes, word) in digest.chunks_exact_mut(4).zip(state) {
        for (to, from) in bytes.iter_mut().zip(word.to_be_bytes()) {
            *to = from;
        }
    }
    digest
}

/// Processes one block of 64 bytes (FIPS 180-4, 6.2.2).
#[allow(
    clippy::many_single_char_names,
    reason = "the working variables keep the names of FIPS 180-4, so the code reads against it"
)]
fn compress(state: &mut [u32; 8], block: &[u8]) {
    // The message schedule is kept as a window of its last sixteen words: before round t,
    // window[0] is W[t]; window[1], window[9] and window[14] give W[t+16] with it.
    let mut window = [0u32; 16];
    for (word, bytes) in window.iter_mut().zip(block.chunks_exact(4)) {
        *word = bytes
            .iter()
            .fold(0u32, |word, byte| (word << 8) | u32::from(*byte));
    }

    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for constant in ROUND {
        let w = window[0];
        let t1 = h
            .wrapping_add(e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25))
            .wrapping_add((e & f) ^ (!e & g))
            .wrapping_add(constant)
            .wrapping_add(w);
        let t2 = (a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22))
            .wrapping_add((a & b) ^ (a & c) ^ (b & c));
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);

        let w1 = window[1];
        let w14 = window[14];
        let next = (w14.rotate_right(17) ^ w14.rotate_right(19) ^ (w14 >> 10))
            .wrapping_add(window[9])
            .wrapping_add(w1.rotate_right(7) ^ w1.rotate_right(18) ^ (w1 >> 3))
            .wrapping_add(w);
        window.rotate_left(1);
        window[15] = next;
    }

    for (word, add) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *word = word.wrapping_add(add);
    }
}

#[cfg(test)]
mod tests {
    use super::sha256;
    use crate::test_support::SplitMix64;
    use sha2::{Digest, Sha256};

    fn hex(digest: &[u8]) -> String {
        use core::fmt::Write;
        digest.iter().fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
    }

    // Published vectors: FIPS 180-4 examples (NIST CSRC, "Examples with intermediate values",
    // SHA256.pdf and SHA2_Additional.pdf) and the long messages of the NIST test suite.
    #[test]
    fn the_empty_message() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn the_one_block_message_abc() {
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn the_448_bit_two_block_message() {
        assert_eq!(
            hex(&sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn the_896_bit_message() {
        assert_eq!(
            hex(&sha256(
                b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
            )),
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"
        );
    }

    #[test]
    fn one_million_times_a() {
        assert_eq!(
            hex(&sha256(&vec![b'a'; 1_000_000])),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn agrees_with_a_hashing_crate_on_random_inputs() {
        let mut random = SplitMix64(0x5741_594c_4f4f_5021);
        // Every length around the padding boundaries (55, 56, 63, 64 bytes and their
        // multiples), then random lengths up to four kibibytes.
        let mut lengths: Vec<u64> = (0..=300).collect();
        lengths.extend((0..200).map(|_| random.next() % 4096));
        for length in lengths {
            let message: Vec<u8> = (0..length)
                .map(|_| random.next().to_le_bytes()[0])
                .collect();
            assert_eq!(
                sha256(&message)[..],
                Sha256::digest(&message)[..],
                "length {length}"
            );
        }
    }
}
