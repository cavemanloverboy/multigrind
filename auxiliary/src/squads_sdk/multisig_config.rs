use solana_program::pubkey::Pubkey;

use crate::squads_sdk::{Member, Permission, Permissions};

pub struct MultisigAddMemberArgs {
    // Borsh size 33
    pub new_member: Member,
    // Borsh size when empty = 1
    pub memo: Option<String>,
}

impl MultisigAddMemberArgs {
    pub fn borsh_with(member: &Pubkey) -> [u8; 42] {
        let mut data = [0; 42];

        // Write discriminator sha256(b"global:multisig_add_member")
        data[..8]
            .copy_from_slice(&[1, 219, 215, 108, 184, 229, 214, 8]);

        // Write member
        data[8..40].copy_from_slice(&member.to_bytes());
        data[40] = Permissions::new(&[
            Permission::Execute,
            Permission::Initiate,
            Permission::Vote,
        ])
        .mask;

        // None -> 0
        data[41] = 0;

        data
    }
}

pub struct MultisigSetConfigAuthorityArgs {
    // Borsh size 32
    pub config_authority: Pubkey,
    // Borsh size when empty = 1
    pub memo: Option<String>,
}

impl MultisigSetConfigAuthorityArgs {
    pub fn borsh_with(new_authority: &Pubkey) -> [u8; 41] {
        let mut data = [0; 41];

        // Write discriminator
        // sha256(b"global:multisig_set_config_authority")
        data[..8]
            .copy_from_slice(&[143, 93, 199, 143, 92, 169, 193, 232]);

        // Write member
        data[8..40].copy_from_slice(&new_authority.to_bytes());

        // None -> 0
        data[40] = 0;

        data
    }
}
