use instruction::{
    initialize_multisig, sell_multisig,
    ArchivedAuxiliaryInstruction as Ix, AuxiliaryInstruction,
};
use rkyv::check_archived_root;
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult,
    program_error::ProgramError, pubkey::Pubkey,
};

pub mod error;
pub mod instruction;
pub mod sdk;
pub mod squads_sdk;
pub mod utils;

pub const MAX_TARGET_LEN: usize = 13;
pub const MIN_TARGET_LEN: usize = 3;

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

declare_id!("AuxokT3REMom8yP5TvuJQaUQUjtkHooQ48hTSQZiYd7W");

pub fn process_instruction(
    program_id: &Pubkey,
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Check program_id
    if *program_id != ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Deserialize instruction
    let instruction =
        check_archived_root::<AuxiliaryInstruction>(instruction_data)
            .or(Err(ProgramError::InvalidInstructionData))?;

    // Process instruction
    match instruction {
        Ix::InitMultisig(args) => {
            initialize_multisig(account_infos, args)
        }

        Ix::SellMultisig(args) => sell_multisig(account_infos, args),
    }?;

    Ok(())
}
