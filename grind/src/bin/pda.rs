//! This executable grinds squads v4 pubkeys. It does so by exploiting
//! pda signers. With an auxiliary program that allows for grinding of
//! pda pubkeys, we can create a pda that has a `creator_key` which
//! seeds a desired squards v4 multisig pubkey. Using pdas as signers is
//! roughly 25% faster in regards to scanning pubkeys when compared to
//! randomly generating keypairs, due to ed25519 keygen being expensive.
use std::time::Instant;

use multigrind::{
    fd_bs58_32::encode_32, EXPECTED_TIME_BETWEEN_HITS, MAX_LEN, TARGETS,
};
use num_format::{Locale, ToFormattedString};
use sha2::{Digest, Sha256};

/// Squads v4 pubkey "SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf"
// TODO: prod
pub const SQUADS_V4_PROGRAM: [u8; 32] = [
    6, 129, 196, 206, 71, 226, 35, 104, 184, 177, 85, 94, 200, 135,
    175, 9, 46, 252, 126, 251, 182, 108, 163, 245, 47, 191, 104, 212,
    172, 156, 183, 168,
];

// /// Squads v4 pubkey "STAG3xkFMyVK3sRtQhipsKuLpRGbgospDpVdNyJqDpS"
// /// (staging)
// pub const SQUADS_V4_PROGRAM: [u8; 32] = [
//     6, 133, 25, 90, 67, 62, 27, 69, 237, 51, 213, 33, 17, 206, 19,
// 176,     136, 45, 117, 199, 250, 78, 159, 114, 169, 7, 206, 227, 192,
// 95,     144, 63,
// ];

// Auxiliary pubkey "AuxokT3REMom8yP5TvuJQaUQUjtkHooQ48hTSQZiYd7W"
pub const AUXILIARY_PROGRAM_ID: [u8; 32] = [
    147, 74, 124, 6, 163, 74, 141, 119, 201, 254, 168, 158, 119, 151,
    17, 76, 113, 16, 243, 56, 94, 122, 178, 38, 18, 220, 192, 126, 58,
    98, 181, 49,
];

// seeds used by squads v4 for a multisig account
// seeds = [SEED_PREFIX, SEED_MULTISIG, create_key.key().as_ref()],
pub const SEED_PREFIX: &[u8] = b"multisig";
pub const SEED_MULTISIG: &[u8] = b"multisig";

/// Solana uses this as final seed to mark pda
/// All in all, we have sha256 of
/// [SEED_PREFIX, SEED_MULTISIG, create_key.key().as_ref(), &[bump],
/// PROGRAM_ID, &PDA_MARKER]
///
/// We can cache the digest of the first two and then proceed to digest
/// the final four elements. This cache costs a 112 byte memcpy instead
/// of digesting the first two seeds again.
pub const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

fn main() {
    println!(
        "expected time between hits = {} nanos",
        EXPECTED_TIME_BETWEEN_HITS.to_formatted_string(&Locale::en)
    );
    let mut logger = logfather::Logger::new();
    logger.file(true);
    logger.path("seeds");
    logger.terminal(true);

    // We can cache the digest of the first two seeds and then proceed
    // to digest the final four elements. This cache costs a 112
    // byte memcpy instead of digesting the first two seeds again.
    let pre_seed = get_preseed();

    // Contiguous bytes for the next four seeds
    const PUBKEY_LEN: usize = 32;
    const BUMP_LEN: usize = 1;
    let mut four_seeds = [0; PUBKEY_LEN
        + BUMP_LEN
        + SQUADS_V4_PROGRAM.len()
        + PDA_MARKER.len()];

    // The program id bytes and PDA marker are fixed.
    // We will only need to modify the first 33 bytes.
    // First write program id
    four_seeds
        [PUBKEY_LEN + BUMP_LEN..PUBKEY_LEN + BUMP_LEN + PUBKEY_LEN]
        .copy_from_slice(&SQUADS_V4_PROGRAM);
    // Then write pda marker
    four_seeds[PUBKEY_LEN + BUMP_LEN + PUBKEY_LEN..]
        .copy_from_slice(PDA_MARKER);

    // We will need scratch space for the pda generation with the
    // auxiliary program seeds = [u128, bump, AUXILIARY_PROGRAM_ID,
    // PDA_MARKER]
    //
    // Don't need space for the pubkey itself as it will be written
    // directly into four_seeds.
    const U128_LEN: usize = core::mem::size_of::<u128>();
    let mut auxiliary_pda_seeds = [0; U128_LEN
        + BUMP_LEN
        + AUXILIARY_PROGRAM_ID.len()
        + PDA_MARKER.len()];

    // The program id bytes and PDA marker are fixed.
    // We will only need to modify the first 16 bytes.
    // First write program id
    auxiliary_pda_seeds
        [U128_LEN + BUMP_LEN..U128_LEN + BUMP_LEN + PUBKEY_LEN]
        .copy_from_slice(&AUXILIARY_PROGRAM_ID);
    // Then write pda marker
    auxiliary_pda_seeds[U128_LEN + BUMP_LEN + PUBKEY_LEN..]
        .copy_from_slice(PDA_MARKER);

    let mut rng = fastrand::Rng::new();
    let string: &'static mut str = "x".repeat(50).leak();

    // Final location of bytes
    let mut multisig_pda_bytes = [0; 32];

    loop {
        let timer = Instant::now();
        const ITERS: u128 = 1_000_000;

        // Tight loop
        // 1) Re-seed auxiliary pda
        // 2) Calculate auxiliary pda
        // 3) Calculate squads v4 pda
        // 4) Check if pda is a target pda
        for _ in 0..ITERS {
            // 1) Re-seed auxiliary pda
            // (only the first 16 bytes)
            rng.fill(&mut auxiliary_pda_seeds[..U128_LEN]);

            // 2) Calculate auxiliary pda
            // Need to find bump that results in pda
            'bump_loop: for bump in (0..=u8::MAX).rev() {
                // Update bump
                auxiliary_pda_seeds[U128_LEN] = bump;

                // Get pda candidate bytes
                // (write directly into squads v4 pda seeds)
                Sha256::new()
                    .chain_update(auxiliary_pda_seeds)
                    .finalize_into(
                        (&mut four_seeds[..PUBKEY_LEN]).into(),
                    );

                // Check if bytes are pda
                if offcurve(&four_seeds[..PUBKEY_LEN]) {
                    break 'bump_loop;
                }
            }

            // 3) Calculate squads v4 pda
            // Need to find bump that results in pda
            'bump_loop: for bump in (0..=u8::MAX).rev() {
                // Update bump
                four_seeds[PUBKEY_LEN] = bump;

                // Get pda candidate bytes
                pre_seed
                    .clone()
                    .chain_update(four_seeds)
                    .finalize_into((&mut multisig_pda_bytes).into());

                // Check if bytes are pda
                if offcurve(&multisig_pda_bytes) {
                    break 'bump_loop;
                }
            }

            // 4) Check if pda is a target pda.
            encode_32(&multisig_pda_bytes[..], &mut *string);

            // This branch is easy to predict (skip almost always)
            // Performance-wise, this is the sad path lol
            if let Some(target) = TARGETS
                .iter()
                .find(|t| ***t == string[..t.len()])
            {
                // Extract payload
                let seed_we_care_about =
                    &auxiliary_pda_seeds[..U128_LEN];
                logfather::info!(&format!(
                    "found {} in {}. seed: {:?}",
                    target,
                    &string[..MAX_LEN + 2],
                    seed_we_care_about
                ));
            }
        }

        println!(
            "avg iter {} nanos",
            timer.elapsed().as_nanos() / ITERS
        );
    }
}

#[inline(always)]
fn get_preseed() -> Sha256 {
    Sha256::new()
        .chain_update(SEED_PREFIX)
        .chain_update(SEED_MULTISIG)
}

#[inline(always)]
pub fn offcurve(bytes: &[u8]) -> bool {
    curve25519_dalek::edwards::CompressedEdwardsY::from_slice(bytes)
        .decompress()
        .is_none()
}
