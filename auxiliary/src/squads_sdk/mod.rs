//! Minimal squads sdk for relevant instructions

use solana_program::pubkey::Pubkey;

// TODO: prod
// pub const SQUADS_V4_PROGRAM: Pubkey = solana_program::pubkey!(
//     "SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf"
// );

/// Staging squads
pub const SQUADS_V4_PROGRAM: Pubkey = solana_program::pubkey!(
    "STAG3xkFMyVK3sRtQhipsKuLpRGbgospDpVdNyJqDpS"
);

pub mod config_transaction;
pub use config_transaction::*;
pub mod multisig_config;
pub use multisig_config::*;
pub mod multisig_create_v2;
pub use multisig_create_v2::*;
pub mod proposal;
pub use proposal::*;

/// Borsh size = 33
pub struct Member {
    pub key: [u8; 32],
    pub permissions: Permissions,
}

#[derive(Clone, Copy)]
pub enum Permission {
    Initiate = 1 << 0,
    Vote = 1 << 1,
    Execute = 1 << 2,
}

/// Borsh size = 1
pub struct Permissions {
    pub mask: u8,
}
impl Permissions {
    pub const fn new(permissions: &[Permission]) -> Self {
        let mut mask = 0;
        let mut idx = 0;
        while idx < permissions.len() {
            mask |= permissions[idx] as u8;
            idx += 1;
        }
        Self { mask }
    }
}
