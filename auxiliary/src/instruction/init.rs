use rkyv::{Archive, Deserialize, Serialize};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult,
    instruction::AccountMeta, msg, program_error::ProgramError,
    system_program::ID as SYSTEM_PROGRAM,
};

use crate::{
    instruction::{AuxiliaryAccount, AUXILIARY_DATA_LEN},
    squads_sdk::{MultisigCreateArgsV2, SQUADS_V4_PROGRAM},
    utils::{
        create_pda_funded_by_payer, StableInstruction, StableView,
    },
    MAX_TARGET_LEN, MIN_TARGET_LEN,
};

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug, PartialEq))]
pub struct InitArgs {
    pub auxiliary_seed: [u8; 16],
    pub auxiliary_bump: u8,
    pub target: String,
}

#[inline(always)]
pub fn initialize_multisig(
    accounts: &[AccountInfo],
    args: &ArchivedInitArgs,
) -> ProgramResult {
    // Extract expected accounts.
    let [sqds_config, sqds_treasury, multisig, auxiliary, signer, system_program, squads @ _rem] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // All accounts are validated by squads v4 (immutable) so we need
    // only validate the squads v4 program account
    assert_eq!(*squads.key, SQUADS_V4_PROGRAM);

    // Additionally, only one person can use this program...
    // TODO if permissioned
    // assert_eq!(*signer.key, USER);
    // TODO: If permissionless, may want to change auxiliary pda
    // to also include user's pubkey to avoid front-running attack
    // This will require changes to grinder!
    // (I'd recommend changing preseed to include pubkey)

    // Limit target size
    if args.target.len() > MAX_TARGET_LEN
        || args.target.len() < MIN_TARGET_LEN
    {
        msg!(
            "target length too small/large. max = {}; min = {}; received {}",
            MAX_TARGET_LEN,
            MIN_TARGET_LEN,
            args.target.len()
        );
        return Err(ProgramError::InvalidInstructionData);
    }

    // Initialize auxiliary and write data
    let auxiliary_seeds =
        &[args.auxiliary_seed.as_ref(), &[args.auxiliary_bump]];
    msg!("creating pda");
    // Either branch internally uses signer seeds so the pda seeds are
    // verified within this functino
    create_pda_funded_by_payer(
        auxiliary,
        &crate::ID,
        AUXILIARY_DATA_LEN as u64,
        auxiliary_seeds,
        system_program,
        signer,
    )?;
    AuxiliaryAccount::verified_write(
        auxiliary,
        args.auxiliary_seed,
        args.auxiliary_bump,
        &args.target,
        multisig.key,
    )?;

    // Invoke squads
    invoke_squads_init_multisig(
        sqds_config,
        sqds_treasury,
        multisig,
        auxiliary,
        signer,
        accounts,
        auxiliary_seeds,
    );

    Ok(())
}

#[inline(always)]
/// Invokes Squads V4 InitializeMultisigV2
fn invoke_squads_init_multisig(
    sqds_config: &AccountInfo<'_>,
    sqds_treasury: &AccountInfo<'_>,
    multisig: &AccountInfo<'_>,
    auxiliary: &AccountInfo<'_>,
    signer: &AccountInfo<'_>,
    accounts: &[AccountInfo<'_>],
    auxiliary_seeds: &[&[u8]; 2],
) {
    // InitializeMultisigV2 instruction
    // Accounts required by InitializeMultisigV2
    let mut instruction_accounts: [AccountMeta; 6] = [
        AccountMeta::new_readonly(sqds_config.key.clone(), false),
        AccountMeta::new(sqds_treasury.key.clone(), false),
        AccountMeta::new(multisig.key.clone(), false),
        AccountMeta::new_readonly(auxiliary.key.clone(), true),
        AccountMeta::new(signer.key.clone(), true),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    // Data is discriminator + borsh ser of MultisigCreateArgsV2
    let mut instruction_data =
        MultisigCreateArgsV2::borsh_with(auxiliary.key);
    let instruction = StableInstruction {
        accounts: StableView::from_array(&mut instruction_accounts),
        data: StableView::from_array(&mut instruction_data),
        program_id: SQUADS_V4_PROGRAM,
    };

    // Our init ix intentionally uses same account order for easy slice
    let cpi_infos = &accounts[..7];

    // Only PDA signer is the `create_key` aka our auxiliary pda
    let cpi_seeds: &[&[&[u8]]] = &[auxiliary_seeds];

    // Cross-program invocation
    // SAFETY: we never hold a RefMut for more than one line
    msg!("invoking squads");
    #[cfg(target_os = "solana")]
    unsafe {
        solana_program::syscalls::sol_invoke_signed_rust(
            &instruction as *const StableInstruction as *const u8,
            cpi_infos.as_ptr() as *const u8,
            7,
            cpi_seeds.as_ptr() as *const u8,
            1,
        );
    }

    #[cfg(not(target_os = "solana"))]
    core::hint::black_box((&instruction, cpi_infos, cpi_seeds));
}
