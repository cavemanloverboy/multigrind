pub struct ProposalCreateArgs {
    /// Index of the multisig transaction this proposal is associated
    /// with.
    pub transaction_index: u64,
    /// Whether the proposal should be initialized with status `Draft`.
    pub draft: bool,
}

/// Includes discriminator and args
const PROPOSAL_CREATE_SIZE: usize = 8 + (8 + 1);

impl ProposalCreateArgs {
    pub fn borsh() -> [u8; PROPOSAL_CREATE_SIZE] {
        let mut data = [0; PROPOSAL_CREATE_SIZE];

        // Write discriminator sha256(b"global:proposal_create")
        data[0..8]
            .copy_from_slice(&[220, 60, 73, 224, 30, 108, 79, 159]);

        // Write index (will always be 1)
        data[8..16].copy_from_slice(&1_u64.to_le_bytes());

        // Write draft (always false)
        data[16] = false as u8;

        data
    }
}

// Borsh size is 1 for None
pub struct ProposalVoteArgs {
    pub memo: Option<String>,
}

/// Includes discriminator and args
const PROPOSAL_APPROVE_SIZE: usize = 8 + 1;

pub struct ProposalApprove;

impl ProposalApprove {
    pub fn borsh() -> [u8; PROPOSAL_APPROVE_SIZE] {
        let mut data = [0; PROPOSAL_APPROVE_SIZE];

        // Write discriminator sha256(b"global:proposal_approve")
        data[0..8]
            .copy_from_slice(&[144, 37, 164, 136, 188, 216, 42, 248]);

        // Write memo = None = 0
        data[8] = 0;

        data
    }
}
