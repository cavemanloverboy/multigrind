//! Magnetar Fields:
//!
//! This is taken almost verbatim from fd_bs58 crate (open source), but
//! made allocation free by taking in a `&mut str` instead of outputing
//! a new string. It retains all of comments.

const INTERMEDIATE_SZ_32: usize = 9; // Computed by ceil(log_(58^5) (256^32-1))
const BINARY_SZ_32: usize = 8; // 32 / 4
const RAW58_SZ_32: usize = 45;
const BYTE_COUNT_32: usize = 32;

const R1_DIV: u64 = 656_356_768; //  58^5

const BASE58_CHARS: &[char; 58] = &[
    '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D',
    'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f',
    'g', 'h', 'i', 'j', 'k', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't',
    'u', 'v', 'w', 'x', 'y', 'z',
];

const ENC_TABLE_32: [[u64; INTERMEDIATE_SZ_32 - 1]; BINARY_SZ_32] = [
    [
        513_735,
        77_223_048,
        437_087_610,
        300_156_666,
        605_448_490,
        214_625_350,
        141_436_834,
        379_377_856,
    ],
    [
        0,
        78_508,
        646_269_101,
        118_408_823,
        91_512_303,
        209_184_527,
        413_102_373,
        153_715_680,
    ],
    [
        0,
        0,
        11_997,
        486_083_817,
        3_737_691,
        294_005_210,
        247_894_721,
        289_024_608,
    ],
    [
        0,
        0,
        0,
        1_833,
        324_463_681,
        385_795_061,
        551_597_588,
        21_339_008,
    ],
    [0, 0, 0, 0, 280, 127_692_781, 389_432_875, 357_132_832],
    [0, 0, 0, 0, 0, 42, 537_767_569, 410_450_016],
    [0, 0, 0, 0, 0, 0, 6, 356_826_688],
    [0, 0, 0, 0, 0, 0, 0, 1],
];

#[inline(always)]
pub fn encode_32<I: AsRef<[u8]>>(input: I, out: &mut str) -> usize {
    let bytes: &[u8; 32] = input.as_ref().try_into().unwrap();
    // Count leading zeros
    let mut in_leading_0s = 0;
    while in_leading_0s < BYTE_COUNT_32 {
        if bytes[in_leading_0s] != 0 {
            break;
        }
        in_leading_0s += 1;
    }

    let mut binary: [u32; BINARY_SZ_32] = [0; BINARY_SZ_32];
    let bytes_as_u32: &[u32] = unsafe {
        // Cast a reference to bytes as a reference to u32
        std::slice::from_raw_parts(
            bytes.as_ptr() as *const u32,
            bytes.len() / std::mem::size_of::<u32>(),
        )
    };

    /* X = sum_i bytes[i] * 2^(8*(BYTE_CNT-1-i)) */

    /* Convert N to 32-bit limbs:
    X = sum_i binary[i] * 2^(32*(BINARY_SZ-1-i)) */

    for i in 0..BINARY_SZ_32 {
        binary[i] = bytes_as_u32[i].to_be(); // Convert to big-endian
                                             // (network byte order)
    }

    let mut intermediate: [u64; INTERMEDIATE_SZ_32] =
        [0; INTERMEDIATE_SZ_32];

    /* Convert to the intermediate format:
      X = sum_i intermediate[i] * 58^(5*(INTERMEDIATE_SZ-1-i))
    Initially, we don't require intermediate[i] < 58^5, but we do want
    to make sure the sums don't overflow. */

    /* The worst case is if binary[7] is (2^32)-1. In that case
    intermediate[8] will be be just over 2^63, which is fine. */

    for i in 0..BINARY_SZ_32 {
        for j in 0..INTERMEDIATE_SZ_32 - 1 {
            intermediate[j + 1] +=
                u64::from(binary[i]) * ENC_TABLE_32[i][j];
        }
    }

    /* Now we make sure each term is less than 58^5. Again, we have to be
    a bit careful of overflow.
    For N==32, in the worst case, as before, intermediate[8] will be
    just over 2^63 and intermediate[7] will be just over 2^62.6.  In
    the first step, we'll add floor(intermediate[8]/58^5) to
    intermediate[7].  58^5 is pretty big though, so intermediate[7]
    barely budges, and this is still fine.
    For N==64, in the worst case, the biggest entry in intermediate at
    this point is 2^63.87, and in the worst case, we add (2^64-1)/58^5,
    which is still about 2^63.87. */

    for i in (1..INTERMEDIATE_SZ_32).rev() {
        intermediate[i - 1] += intermediate[i] / R1_DIV;
        intermediate[i] %= R1_DIV;
    }

    let mut raw_base58: [u8; RAW58_SZ_32] = [0; RAW58_SZ_32];

    for i in 0..INTERMEDIATE_SZ_32 {
        /* We know intermediate[ i ] < 58^5 < 2^32 for all i, so casting to
        a uint is safe.  GCC doesn't seem to be able to realize this, so
        when it converts ulong/ulong to a magic multiplication, it
        generates the single-op 64b x 64b -> 128b mul instruction.  This
        hurts the CPU's ability to take advantage of the ILP here. */
        let v = intermediate[i] as u32;
        raw_base58[5 * i + 4] = (v % 58) as u8;
        raw_base58[5 * i + 3] = (v / 58 % 58) as u8;
        raw_base58[5 * i + 2] = (v / 3364 % 58) as u8;
        raw_base58[5 * i + 1] = (v / 195112 % 58) as u8;
        raw_base58[5 * i] = (v / 11316496) as u8; // This one is known
                                                  // to be less than 58
    }

    /* Finally, actually convert to the string.  We have to ignore all the
    leading zeros in raw_base58 and instead insert in_leading_0s
    leading '1' characters.  We can show that raw_base58 actually has
    at least in_leading_0s, so we'll do this by skipping the first few
    leading zeros in raw_base58. */

    let mut raw_leading_0s = 0;
    while raw_leading_0s < RAW58_SZ_32 {
        if raw_base58[raw_leading_0s] != 0 {
            break;
        }
        raw_leading_0s += 1;
    }

    /* It's not immediately obvious that raw_leading_0s >= in_leading_0s,
    but it's true.  In base b, X has floor(log_b X)+1 digits.  That
    means in_leading_0s = N-1-floor(log_256 X) and raw_leading_0s =
    RAW58_SZ-1-floor(log_58 X).  Let X<256^N be given and consider:
    raw_leading_0s - in_leading_0s =
      =  RAW58_SZ-N + floor( log_256 X ) - floor( log_58 X )
      >= RAW58_SZ-N - 1 + ( log_256 X - log_58 X ) .
    log_256 X - log_58 X is monotonically decreasing for X>0, so it
    achieves it minimum at the maximum possible value for X, i.e.
    256^N-1.
      >= RAW58_SZ-N-1 + log_256(256^N-1) - log_58(256^N-1)
    When N==32, RAW58_SZ is 45, so this gives skip >= 0.29
    When N==64, RAW58_SZ is 90, so this gives skip >= 1.59.
    Regardless, raw_leading_0s - in_leading_0s >= 0. */

    // SAFETY:
    // We are only writing bs58 characters which are all valid utf8
    let out_bytes = unsafe { out.as_bytes_mut() };
    let skip = raw_leading_0s - in_leading_0s;
    let end = RAW58_SZ_32 - skip;
    for i in 0..end {
        let idx = raw_base58[skip + i];
        out_bytes[i] = BASE58_CHARS[idx as usize] as u8;
    }

    end
}
