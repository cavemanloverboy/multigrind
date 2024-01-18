use rkyv::{Archive, Deserialize, Serialize};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult,
    instruction::AccountMeta, program_error::ProgramError,
    pubkey::Pubkey, rent::Rent, system_program::ID as SYSTEM_PROGRAM,
    sysvar::Sysvar,
};

use crate::{
    instruction::AuxiliaryAccount,
    squads_sdk::{
        ConfigTransactionCreateArgs, ConfigTransactionExecute,
        MultisigAddMemberArgs, MultisigSetConfigAuthorityArgs,
        ProposalApprove, ProposalCreateArgs, SQUADS_V4_PROGRAM,
    },
    utils::{StableInstruction, StableView},
    MAX_TARGET_LEN, MIN_TARGET_LEN,
};

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[archive(compare(PartialEq), check_bytes)]
#[archive_attr(derive(Debug, PartialEq))]
pub struct SellArgs {
    // pub config_transaction_bump: u8,
}

#[inline(always)]
pub fn sell_multisig(
    accounts: &[AccountInfo],
    _args: &ArchivedSellArgs, // unused but scaffolded
) -> ProgramResult {
    // Extract expected accounts.
    // We must validate that auxiliary is an pda owned and initialized
    // by us. We must validate the auxiliary <-> multisig
    // correspondence We must validate user == USER
    // Other accounts will be passed onto and validated by squads
    let [multisig, auxiliary, buyer, system_program, squads, user, _config_transaction, _proposal @ _rem] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    assert_eq!(*squads.key, SQUADS_V4_PROGRAM);

    // TODO if permissioned
    // assert_eq!(*user.key, USER);
    // TODO: if not permissioned then auxiliary account must store
    // recipient pubkey or verify auxiliary pda seeds w/ user pubkey

    // Auxiliary must be initialized pda owned by us
    assert_eq!(*auxiliary.owner, crate::ID);

    // Auxiliary must correspond to the multisig in question
    // This is done in a scope to drop the Ref before invoke
    // In addition to checks we get seed, bump, target len
    let (target_len, auxiliary_seed, auxiliary_bump) = {
        let auxiliary_data: _ = auxiliary.try_borrow_data()?;
        let auxiliary_account = bytemuck::try_from_bytes::<
            AuxiliaryAccount,
        >(&auxiliary_data)
        .expect("invalid auxiliary account");
        assert_eq!(auxiliary_account.multisig, multisig.key.to_bytes());

        (
            auxiliary_account.len as u32,
            auxiliary_account.seed,
            auxiliary_account.bump,
        )
    };

    // Invoke squads to transfer ownership of multisig
    let auxiliary_seeds: &[&[u8]] =
        &[&auxiliary_seed, &[auxiliary_bump]];
    let rent =
        invoke_squads_add_update_remove(accounts, auxiliary_seeds);

    // Send fee to USER
    assert!(
        (MIN_TARGET_LEN as u32 <= target_len) & (target_len <= MAX_TARGET_LEN as u32),
        "MIN_TARGET_LEN <= target len <= MAX_TARGET_LEN sanity check failed: {} <= {} <= {}",
        MIN_TARGET_LEN, target_len, MAX_TARGET_LEN
    );
    let fee = compute_fee_lookup(target_len);
    send_fee_to_user(rent + fee, buyer, user, system_program);

    // Delete auxiliary account
    // TODO: is this all we have to do?
    solana_program::msg!("deleting auxiliary account");
    let mut auxiliary_lamports = auxiliary.try_borrow_mut_lamports()?;
    **user.try_borrow_mut_lamports()? += **auxiliary_lamports;
    **auxiliary_lamports = 0;
    auxiliary.realloc(0, false)?;
    auxiliary.assign(&SYSTEM_PROGRAM);

    Ok(())
}

#[inline(always)]
/// Invokes Squads V4 with multiple instructions:
///
/// 1) Add buyer as member to controlled multisig
/// 2) Remove auxiliary as config authority
/// 3) Remove auxiliary as member. a) Create config transaction to
///    remove auxiliary as member b) Propose auxiliary member removal
///    (activate proposal) c) Approve auxiliary member removal (vote on
///    proposal) d) Execute auxiliary member removal
///
/// Returns rent cost for multisig in SOL lamports
#[must_use]
fn invoke_squads_add_update_remove(
    accounts: &[AccountInfo<'_>],
    auxiliary_seeds: &[&[u8]],
) -> u64 {
    // Unpack accounts
    let [multisig, auxiliary, buyer, system_program, squads, _user, config_transaction, proposal @ _rem] =
        accounts
    else {
        unreachable!("sell ix already checked length");
    };

    // 1) Add buyer as member to controlled multisig
    // Accounts required by MultisigConfig
    // These can be reused for the next invoke
    let mut instruction_accounts: [AccountMeta; 4] = [
        AccountMeta::new(multisig.key.clone(), false),
        AccountMeta::new_readonly(auxiliary.key.clone(), true),
        AccountMeta::new(buyer.key.clone(), true), // rent payer
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];

    // Data is discriminator + borsh ser of add_member(buyer)
    let mut instruction_data =
        MultisigAddMemberArgs::borsh_with(buyer.key);
    let mut instruction = StableInstruction {
        accounts: StableView::from_array(&mut instruction_accounts),
        data: StableView::from_array(&mut instruction_data),
        program_id: SQUADS_V4_PROGRAM,
    };

    // Our sell ix intentionally uses same account order for easy slice
    let cpi_infos = &accounts[..5];

    // Only PDA signer is the `create_key` aka our auxiliary pda
    let cpi_seeds: &[&[&[u8]]] = &[auxiliary_seeds];

    // Prior to invoking squads, record multisig data length and rent
    // cost TODO... we should think about future rent changes a la
    // Solana SIMD-0101.
    let multisig_data_length = multisig.data_len();
    let multisig_rent_cost = Rent::get()
        .unwrap()
        .minimum_balance(multisig_data_length);

    // Cross-program invocation
    // SAFETY: we never hold a RefMut for more than one line
    solana_program::msg!("adding buyer as multisig member");
    #[cfg(target_os = "solana")]
    unsafe {
        solana_program::syscalls::sol_invoke_signed_rust(
            &instruction as *const StableInstruction as *const u8,
            cpi_infos.as_ptr() as *const u8,
            5,
            cpi_seeds.as_ptr() as *const u8,
            1,
        );
    }

    #[cfg(not(target_os = "solana"))]
    core::hint::black_box((&instruction, &cpi_infos, cpi_seeds));

    // 2) Remove auxiliary as config authority (set to default)
    // We reuse the same accounts and seeds, so only need to update data
    const NEW_AUTHORITY: Pubkey = Pubkey::new_from_array([0; 32]);
    let mut instruction_data =
        MultisigSetConfigAuthorityArgs::borsh_with(&NEW_AUTHORITY);
    instruction.data = StableView::from_array(&mut instruction_data);
    // Cross-program invocation
    // SAFETY: we never hold a RefMut for more than one line
    solana_program::msg!("removing auxiliary as config authority");
    #[cfg(target_os = "solana")]
    unsafe {
        solana_program::syscalls::sol_invoke_signed_rust(
            &instruction as *const StableInstruction as *const u8,
            cpi_infos.as_ptr() as *const u8,
            5,
            cpi_seeds.as_ptr() as *const u8,
            1,
        );
    }

    // 3a) Now we must create a config transaction
    let mut instruction_accounts: [AccountMeta; 5] = [
        AccountMeta::new(multisig.key.clone(), false),
        AccountMeta::new(config_transaction.key.clone(), false),
        AccountMeta::new_readonly(auxiliary.key.clone(), true), /* transaction creator */
        AccountMeta::new(buyer.key.clone(), true), // rent payer
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    instruction.accounts =
        StableView::from_array(&mut instruction_accounts);
    let mut instruction_data =
        ConfigTransactionCreateArgs::remove_member_borsh_with(
            auxiliary.key,
        );
    instruction.data = StableView::from_array(&mut instruction_data);
    solana_program::msg!(
        "creating config transcation to remove auxiliary pda"
    );

    // We will need to update cpi infos
    // (we will reuse this for proposals by swapping out
    // config_transaction)
    let mut cpi_infos: [AccountInfo; 6] = [
        multisig.clone(),
        config_transaction.clone(),
        auxiliary.clone(),
        buyer.clone(),
        system_program.clone(),
        squads.clone(),
    ];

    // Only PDA signer is the `creator` aka our auxiliary pda
    let cpi_seeds: &[&[&[u8]]] = &[auxiliary_seeds];

    #[cfg(target_os = "solana")]
    unsafe {
        solana_program::syscalls::sol_invoke_signed_rust(
            &instruction as *const StableInstruction as *const u8,
            cpi_infos.as_ptr() as *const u8,
            6,
            cpi_seeds.as_ptr() as *const u8,
            1,
        );
    }
    #[cfg(not(target_os = "solana"))]
    core::hint::black_box((&instruction, &cpi_infos, cpi_seeds));

    // 3b) Create proposal using this config transaction
    let mut instruction_accounts: [AccountMeta; 5] = [
        AccountMeta::new(multisig.key.clone(), false),
        AccountMeta::new(proposal.key.clone(), false),
        AccountMeta::new_readonly(auxiliary.key.clone(), true), /* proposal creator */
        AccountMeta::new(buyer.key.clone(), true), // rent payer
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    instruction.accounts =
        StableView::from_array(&mut instruction_accounts);
    let mut instruction_data = ProposalCreateArgs::borsh();
    instruction.data = StableView::from_array(&mut instruction_data);
    // Need to swap out config transaction for proposal
    // (seeds stay the same because auxiliary is still signer)
    cpi_infos[1] = proposal.clone();
    #[cfg(target_os = "solana")]
    unsafe {
        solana_program::syscalls::sol_invoke_signed_rust(
            &instruction as *const StableInstruction as *const u8,
            cpi_infos.as_ptr() as *const u8,
            6,
            cpi_seeds.as_ptr() as *const u8,
            1,
        );
    }
    #[cfg(not(target_os = "solana"))]
    core::hint::black_box((&instruction, &cpi_infos, cpi_seeds));

    // 3c) Vote on proposal using this config transaction and proposal
    let mut instruction_accounts: [AccountMeta; 3] = [
        AccountMeta::new_readonly(multisig.key.clone(), false),
        AccountMeta::new(auxiliary.key.clone(), true), // voter
        AccountMeta::new(proposal.key.clone(), false),
    ];
    instruction.accounts =
        StableView::from_array(&mut instruction_accounts);
    let mut instruction_data = ProposalApprove::borsh();
    instruction.data = StableView::from_array(&mut instruction_data);
    // Need new infos again
    // (seeds stay the same because auxiliary is still signer)
    let cpi_infos: [AccountInfo; 4] = [
        multisig.clone(),
        auxiliary.clone(),
        proposal.clone(),
        squads.clone(),
    ];
    #[cfg(target_os = "solana")]
    unsafe {
        solana_program::syscalls::sol_invoke_signed_rust(
            &instruction as *const StableInstruction as *const u8,
            cpi_infos.as_ptr() as *const u8,
            4,
            cpi_seeds.as_ptr() as *const u8,
            1,
        );
    }
    #[cfg(not(target_os = "solana"))]
    core::hint::black_box((&instruction, &cpi_infos, cpi_seeds));

    // 3d) Execute config transaction to remove auxiliar pda
    // After this, buyer is 1/1 multisig authority
    let mut instruction_accounts: [AccountMeta; 6] = [
        AccountMeta::new(multisig.key.clone(), false),
        AccountMeta::new_readonly(auxiliary.key.clone(), true), /* executing member */
        AccountMeta::new(proposal.key.clone(), false),
        AccountMeta::new_readonly(
            config_transaction.key.clone(),
            false,
        ),
        AccountMeta::new(buyer.key.clone(), true), // rent payer
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    instruction.accounts =
        StableView::from_array(&mut instruction_accounts);
    let mut instruction_data = ConfigTransactionExecute::borsh();
    instruction.data = StableView::from_array(&mut instruction_data);
    // Need new infos again
    // (seeds stay the same because auxiliary is still signer)
    let cpi_infos: [AccountInfo; 7] = [
        multisig.clone(),
        auxiliary.clone(),
        proposal.clone(),
        config_transaction.clone(),
        buyer.clone(),
        system_program.clone(),
        squads.clone(),
    ];
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
    core::hint::black_box((&instruction, &cpi_infos, cpi_seeds));

    multisig_rent_cost
}

fn send_fee_to_user<'info>(
    fee: u64,
    buyer: &AccountInfo<'info>,
    user: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
) {
    // Transfer accounts [from, to]
    let mut accounts = [
        AccountMeta::new(buyer.key.clone(), true),
        AccountMeta::new(user.key.clone(), false),
    ];
    // Data is u32 discriminator + lamports
    let mut data = [0; 4 + 8];
    data[0..4].copy_from_slice(&2_u32.to_le_bytes());
    data[4..].copy_from_slice(&fee.to_le_bytes());

    // Build transfer instruction
    let instruction = StableInstruction {
        accounts: StableView::from_array(&mut accounts),
        data: StableView::from_array(&mut data),
        program_id: SYSTEM_PROGRAM,
    };

    // Account infos and seeds
    let cpi_infos =
        [buyer.clone(), user.clone(), system_program.clone()];
    let cpi_seeds: &[&[&[u8]]] = &[];

    solana_program::msg!("charging user for multisig");
    #[cfg(target_os = "solana")]
    unsafe {
        solana_program::syscalls::sol_invoke_signed_rust(
            &instruction as *const StableInstruction as *const u8,
            cpi_infos.as_ptr() as *const u8,
            3,
            cpi_seeds.as_ptr() as *const u8,
            0,
        );
    }

    #[cfg(not(target_os = "solana"))]
    core::hint::black_box((&instruction, &cpi_infos, cpi_seeds));
}

fn compute_fee_lookup(target_len: u32) -> u64 {
    // This is kind of ad hoc. Should scale exponentially in length
    // with a base between 29 and 58 but that's kind of fast
    match target_len {
        // 13 is insane so 1000 SOL seems fair
        13 => 1000___000_000_000,
        12 => 100___000_000_000,
        11 => 50___000_000_000,
        10 => 20___000_000_000,
        9 => 6___000_000_000,
        8 => 2___000_000_000,
        // More common range here is ≈1 SOL
        7 => 1___000_000_000,
        6 => 500_000_000,
        5 => 200_000_000,
        4 => 100_000_000,
        // Super cheap 0.01 SOL
        3 => 10_000_000,
        _ => unreachable!("compute fee"),
    }
}

// An old simple 1/2^n scaling
// Letters | Fee
// 13      | 1000
// 12      | 500
// 11      | 250
// 10      | 125
// 9       | 62.5
// 8       | 31.25
// 7       | 15.625
// 6       | 7.8125
// 5       | 3.90625
// const MAX_FEE: u64 = 1000 * LAMPORTS_PER_SOL;
// MAX_FEE / 2_u64.pow(MAX_TARGET_LEN as u32 - target_len)

// old fee
// #[test]
// fn test_compute_fee() {
//     // serves to test but also visualize down to 1
//     assert_eq!(compute_fee_lookup(13), 1000___000_000_000);
//     assert_eq!(compute_fee_lookup(12), 500___000_000_000);
//     assert_eq!(compute_fee_lookup(11), 250___000_000_000);
//     assert_eq!(compute_fee_lookup(10), 125___000_000_000);
//     assert_eq!(compute_fee_lookup(9), 62___500_000_000);
//     assert_eq!(compute_fee_lookup(8), 31___250_000_000);
//     assert_eq!(compute_fee_lookup(7), 15___625_000_000);
//     assert_eq!(compute_fee_lookup(6), 7___812_500_000);
//     assert_eq!(compute_fee_lookup(5), 3___906_250_000);
// }
