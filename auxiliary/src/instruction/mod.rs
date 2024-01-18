use bytemuck::{Pod, Zeroable};
use rkyv::{Archive, Deserialize, Serialize};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

pub mod init;
pub use init::*;
pub mod sell;
pub use sell::*;

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug, PartialEq))]
pub enum AuxiliaryInstruction {
    InitMultisig(InitArgs),
    SellMultisig(SellArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
#[repr(C, align(1))]
pub struct AuxiliaryAccount {
    /// Multisig associated with this auxiliary PDA
    pub multisig: [u8; 32],

    /// Seed used to derive this address
    pub seed: [u8; 16],

    /// Bump used to derive this address
    pub bump: u8,

    /// Length of bs58 pubkey substring (target)
    pub len: u8,
}

pub const AUXILIARY_DATA_LEN: usize =
    core::mem::size_of::<AuxiliaryAccount>();

impl AuxiliaryAccount {
    pub fn verified_write(
        account: &AccountInfo,
        auxiliary_seed: [u8; 16],
        auxiliary_bump: u8,
        target: &str,
        multisig_key: &Pubkey,
    ) -> ProgramResult {
        // We must verify (for user sake i.e. pricing, not really for
        // program accuracy) that the multisig contains the
        // target string
        const MAX_PUBKEY_LEN: usize = 44;
        const STR_BYTES: [u8; 44] = [b'a'; MAX_PUBKEY_LEN];
        let mut str_bytes = STR_BYTES;
        let temp_str =
            core::str::from_utf8_mut(&mut str_bytes).unwrap();
        let str_len = bs58::encode(multisig_key.as_ref())
            .onto(&mut *temp_str)
            .unwrap();
        if !temp_str[..str_len].starts_with(target) {
            msg!("multisig pubkey does not start with target");
            return Err(ProgramError::InvalidInstructionData);
        }

        // Write after verification
        let mut account_data = account.try_borrow_mut_data()?;
        // This should happen after initialization,
        // so we are guaranteed the correct length
        let AuxiliaryAccount {
            ref mut multisig,
            ref mut seed,
            ref mut bump,
            ref mut len,
        } = bytemuck::try_from_bytes_mut(&mut account_data).unwrap();
        *multisig = multisig_key.to_bytes();
        *seed = auxiliary_seed;
        *bump = auxiliary_bump;
        *len = target.len() as u8;

        Ok(())
    }
}
