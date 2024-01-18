pub mod fd_bs58_32;

#[rustfmt::skip]
// Some sample set of targets
pub const TARGETS: &[&str] = &[
    "Bitcoin",
    "Satoshi",
    "Nakamoto",
    "Proofofwork",
];

// Largest length in TARGETS
pub const MAX_LEN: usize = {
    let mut idx = 0;
    let mut len = 0;
    while idx < TARGETS.len() {
        let target = &TARGETS[idx];
        if target.len() > len {
            len = target.len();
        };
        idx += 1;
    }
    len
};

// Assert largest length is <=16
const _: () = assert!(MAX_LEN <= 16);

pub const MIN_LEN: usize = {
    let mut idx = 0;
    let mut len = usize::MAX;
    while idx < TARGETS.len() {
        let target = &TARGETS[idx];
        if target.len() < len {
            len = target.len();
        };
        idx += 1;
    }
    len
};

// Assert min length is >=5 (prevent spam)
const _: () = assert!(MAX_LEN >= 5);

// Check targets for invalid bs58
const _: () = {
    let mut idx = 0;
    while idx < TARGETS.len() {
        let target = &TARGETS[idx];
        let mut idx2 = 0;
        while idx2 < target.len() {
            assert!(target.as_bytes()[idx2] != b"l"[0], "{}", *target);
            assert!(target.as_bytes()[idx2] != b"O"[0], "{}", *target);
            assert!(target.as_bytes()[idx2] != b"0"[0], "{}", *target);
            assert!(target.as_bytes()[idx2] != b"I"[0], "{}", *target);
            idx2 += 1;
        }
        idx += 1;
    }
};

pub const EXPECTED_TIME_BETWEEN_HITS: u128 =
    approx_time_between_hits(&TARGETS, 13000);
#[allow(unused)]
mod private_test {
    use super::*;

    const EXPECTED_TIME_BETWEEN_HITS2: u128 =
        approx_time_between_hits(&["aaaaa"], 13000);
    /// approx_time_between_hits assumes all targets are unique
    const EXPECTED_TIME_BETWEEN_HITS3: u128 =
        approx_time_between_hits(&["aaaaa"; 5], 13000);

    /// This should be approximately 5 (and it is)
    const RATIO: u128 =
        EXPECTED_TIME_BETWEEN_HITS2 / EXPECTED_TIME_BETWEEN_HITS3;
}

/// This assumes targets are all valid bs58 and unique
const fn approx_time_between_hits(
    targets: &[&str],
    avg_time_per_check: u128,
) -> u128 {
    let mut total_scaled_probability = 0u128;
    let mut idx = 0;

    while idx < targets.len() {
        let length = targets[idx].len();
        // Assuming the probability is scaled by a factor
        let scaled_probability =
            58u128.pow(16) / 58u128.pow(length as u32);
        total_scaled_probability += scaled_probability;
        idx += 1;
    }

    if total_scaled_probability > 0 {
        // Scaling back the result
        avg_time_per_check * 58u128.pow(16) / total_scaled_probability
    } else {
        u128::MAX // Represents infinity in this context
    }
}
