use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program::ID as SYSTEM_PROGRAM,
};

use crate::{
    instruction::{AuxiliaryInstruction, InitArgs, SellArgs},
    squads_sdk::SQUADS_V4_PROGRAM,
    utils::USER,
};

const SEED_PREFIX: &[u8] = b"multisig";
const SEED_MULTISIG: &[u8] = b"multisig";
const SEED_TRANSACTION: &[u8] = b"transaction";
const SEED_PROPOSAL: &[u8] = b"proposal";

/// Given the seed for an auxiliary account, computes the auxiliary pda.
/// Returns the `Pubkey` and the `u8` bump
pub fn auxiliary_pda(seed: &[u8; 16]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seed], &crate::ID)
}

/// Given the seed for an auxiliary account, this computes the final
/// multisig pda from the auxiliary pda
///
/// Returns the `Pubkey` and the `u8` bump
pub fn multisig_pda_from_auxiliary(auxiliary: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SEED_PREFIX, SEED_MULTISIG, auxiliary.as_ref()],
        &SQUADS_V4_PROGRAM,
    )
}

/// Given the seed for an auxiliary account, this computes the final
/// multisig pda by first computing the auxiliary pda and then computing
/// the multisig account.
///
/// Returns the `Pubkey` and the `u8` bump
pub fn multisig_pda_from_seed(seed: &[u8; 16]) -> (Pubkey, u8) {
    let (auxiliary_pda, _auxiliary_bump) = auxiliary_pda(seed);
    multisig_pda_from_auxiliary(&auxiliary_pda)
}

pub struct InstructionBuilder;

impl InstructionBuilder {
    pub fn initialize_multisig(
        user: Pubkey,
        auxiliary_seed: [u8; 16],
        target: String,
    ) -> Instruction {
        // Get pdas
        let (auxiliary_pda, auxiliary_bump) =
            auxiliary_pda(&auxiliary_seed);
        let (multisig_pda, _msb) =
            multisig_pda_from_auxiliary(&auxiliary_pda);

        // Get instruction data
        let instruction_data: Vec<u8> = {
            // Build archivable instruction
            let instruction =
                AuxiliaryInstruction::InitMultisig(InitArgs {
                    auxiliary_seed,
                    auxiliary_bump,
                    target,
                });

            // Archive it.
            //
            // PERF TODO: this is pretty sad bc double alloc.
            // next version of rkyv will not use AlignedVec.
            // There's probably a way to do this with a custom
            // serializer.
            rkyv::to_bytes::<_, 512>(&instruction)
                .unwrap()
                .to_vec()
        };

        // Get instruction accounts
        // TODO: this will have to change to prod config, prod treasury
        // let [sqds_config, sqds_treasury, multisig, auxiliary,
        // signer, system_program, squads @ _rem] =
        let instruction_accounts = vec![
            AccountMeta::new_readonly(PROGRAM_CONFIG, false),
            AccountMeta::new(TREASURY, false),
            AccountMeta::new(multisig_pda, false),
            AccountMeta::new(auxiliary_pda, false),
            AccountMeta::new(user, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(SQUADS_V4_PROGRAM, false),
        ];

        Instruction {
            program_id: crate::ID,
            accounts: instruction_accounts,
            data: instruction_data,
        }
    }

    /// Creates an instruction to buy a multisig.
    ///
    /// Note: auxiliary key may be obtained by checking multisig
    /// authority
    pub fn buy_multisig(
        auxiliary: Pubkey,
        multisig: Pubkey,
        buyer: Pubkey,
        recipient: Pubkey,
    ) -> Instruction {
        let transaction_index = 0_u64; // This is the first transaction for this multisig

        // We need the following additional pdas:
        // 1) config_transaction to make a transaction to remove
        //    auxiliary as multisig member
        // 2) proposal to propose and approve the config_transaction to the multisig
        let config_transaction_seeds: &[&[u8]] = &[
            SEED_PREFIX,
            multisig.as_ref(),
            SEED_TRANSACTION,
            &transaction_index
                .checked_add(1)
                .unwrap()
                .to_le_bytes(),
        ];
        let (config_transaction_pda, _config_transaction_bump) =
            Pubkey::find_program_address(
                config_transaction_seeds,
                &SQUADS_V4_PROGRAM,
            );
        let proposal_seeds: &[&[u8]] = &[
            SEED_PREFIX,
            multisig.as_ref(),
            SEED_TRANSACTION,
            &transaction_index
                .checked_add(1)
                .unwrap()
                .to_le_bytes(),
            SEED_PROPOSAL,
        ];
        let (proposal_pda, _proposal_bump) =
            Pubkey::find_program_address(
                proposal_seeds,
                &SQUADS_V4_PROGRAM,
            );

        Instruction {
            program_id: crate::ID,
            accounts: vec![
                AccountMeta::new(multisig, false),
                AccountMeta::new(auxiliary, false),
                AccountMeta::new(buyer, true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
                AccountMeta::new_readonly(SQUADS_V4_PROGRAM, false),
                // TODO: USER is recipient of fees!
                AccountMeta::new(recipient, false),
                AccountMeta::new(config_transaction_pda, false),
                AccountMeta::new(proposal_pda, false),
            ],
            data: rkyv::to_bytes::<_, 64>(
                &AuxiliaryInstruction::SellMultisig(SellArgs {}),
            )
            .unwrap()
            .to_vec(),
        }
    }
}

/// TODO: this will have to change to prod config, prod treasury
const PROGRAM_CONFIG: Pubkey = solana_program::pubkey!(
    "8z2aG86nbnFBTA5tQin1YzVdhqyf84Xco2sqHy5ZyA2N"
);
/// TODO: this will have to change to prod config, prod treasury
const TREASURY: Pubkey = solana_program::pubkey!(
    "HM5y4mz3Bt9JY9mr1hkyhnvqxSH4H2u2451j7Hc2dtvK"
);
