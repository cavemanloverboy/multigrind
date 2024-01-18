use solana_program::pubkey::Pubkey;

use crate::squads_sdk::{Member, Permission, Permissions};

/// Discriminator + borsh args size
pub const MULTISIG_CREATE_ARGS_SIZE: usize =
    8 + (33 + 2 + 37 + 4 + 1 + 1);

/// Borsh size = 33 + 2 + 37 + 4 + 1
pub struct MultisigCreateArgsV2 {
    /// The authority that can configure the multisig: add/remove
    /// members, change the threshold, etc. Should be set to `None`
    /// for autonomous multisigs.
    ///
    /// Borsh size when Some = 33
    pub config_authority: Option<[u8; 32]>,
    /// The number of signatures required to execute a transaction.
    ///
    /// Borsh size = 2
    pub threshold: u16,
    /// The members of the multisig.
    ///
    /// Borsh size with len 1 = 4 + 33 = 37
    pub members: Vec<Member>,
    /// How many seconds must pass between transaction voting,
    /// settlement, and execution. Borsh size = 4
    pub time_lock: u32,
    /// The address where the rent for the accounts related to
    /// executed, rejected, or cancelled transactions can be
    /// reclaimed. If set to `None`, the rent reclamation feature is
    /// turned off.
    ///
    /// Borsh size when empty = 1
    pub rent_collector: Option<[u8; 32]>,
    /// Memo is used for indexing only.
    ///
    /// Borsh size when empty = 1
    pub memo: Option<String>,
}

impl MultisigCreateArgsV2 {
    pub fn borsh_with(
        authority: &Pubkey,
    ) -> [u8; MULTISIG_CREATE_ARGS_SIZE] {
        let mut data = [0; MULTISIG_CREATE_ARGS_SIZE];

        // Write discriminator sha256(b"global:multisig_create_v2")
        data[0..8]
            .copy_from_slice(&[50, 221, 199, 93, 40, 245, 139, 233]);

        // config_authority: Some(authority)
        data[8] = 1; // is some -> 1
        data[9..41].copy_from_slice(authority.as_ref());

        // threshold: u16
        data[41..43].copy_from_slice(&1u16.to_le_bytes());

        // members: Vec<Member>
        data[43..47].copy_from_slice(&1u32.to_le_bytes()); // len = 1
        data[47..79].copy_from_slice(authority.as_ref());
        data[79] = Permissions::new(&[
            Permission::Execute,
            Permission::Vote,
            Permission::Initiate,
        ])
        .mask;

        // time_lock: u32
        data[80..84].copy_from_slice(&0u32.to_le_bytes());

        // rent_collector: Option<[u8; 32]> = None -> 0
        // (TODO: this can be be set when sold. Decide on this)
        data[84] = 0;

        // memo: Option<String> = None -> 0
        data[85] = 0;

        data
    }
}
