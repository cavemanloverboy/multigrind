use core::ptr::NonNull;

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult,
    instruction::AccountMeta, pubkey::Pubkey, rent::Rent,
    sysvar::Sysvar,
};

/// TODO: CHANGE THIS IF PERMISSIONED
pub const USER: Pubkey =
    solana_program::pubkey!("11111111111111111111111111111111");

#[repr(C)]
pub(crate) struct StableInstruction {
    pub(crate) accounts: StableView<AccountMeta>,
    pub(crate) data: StableView<u8>,
    pub(crate) program_id: Pubkey,
}

#[repr(C)]
pub(crate) struct StableView<T> {
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
}

impl<T> StableView<T> {
    #[inline(always)]
    pub(crate) fn from_array<const N: usize>(
        array: &mut [T; N],
    ) -> StableView<T> {
        StableView {
            // SAFETY: array implies nonnull
            ptr: unsafe { NonNull::new_unchecked(array.as_mut_ptr()) },
            cap: 0,
            len: N,
        }
    }
}

/// Creates a new pda
#[inline(always)]
pub fn create_pda_funded_by_payer<'a, 'info>(
    target_account: &'a AccountInfo<'info>,
    owner: &Pubkey,
    space: u64,
    pda_seeds: &[&[u8]],
    system_program: &'a AccountInfo<'info>,
    payer: &'a AccountInfo<'info>,
) -> ProgramResult {
    let rent_sysvar = Rent::get()?;
    if target_account.lamports() == 0 {
        // Create account if balance is zero
        let create_account_instruction =
            solana_program::system_instruction::create_account(
                payer.key,
                target_account.key,
                rent_sysvar.minimum_balance(space as usize),
                space,
                owner,
            );
        let create_account_account_infos = [
            payer.clone(),
            target_account.clone(),
            system_program.clone(),
        ];
        #[cfg(target_os = "solana")]
        unsafe {
            use solana_program::stable_layout::stable_instruction::StableInstruction;
            let stable_instruction: StableInstruction =
                create_account_instruction.into();
            let cpi_seeds = &[pda_seeds];
            solana_program::syscalls::sol_invoke_signed_rust(
                (&stable_instruction) as *const StableInstruction
                    as *const u8,
                create_account_account_infos.as_ptr() as *const u8,
                3,
                cpi_seeds.as_ptr() as *const u8,
                1,
            );
        }
        #[cfg(not(target_os = "solana"))]
        core::hint::black_box((
            &create_account_instruction,
            &create_account_account_infos,
            pda_seeds,
        ));
    } else {
        // Otherwise, if the balance is nonzero we need to
        // 1) transfer sufficient lamports for rent exemption -- paid
        //    for by the user
        // 2) system_instruction::allocate enough space for the account
        // 3) assign our program as the owner
        //
        // This is the sad but rare path so I won't bother with direct
        // syscall

        // 1) transfer sufficient lamports for rent exemption
        let rent_exempt_balance = rent_sysvar
            .minimum_balance(space as usize)
            .saturating_sub(target_account.lamports());
        if rent_exempt_balance > 0 {
            // Only call transfer instruction if required
            let transfer_instruction =
                solana_program::system_instruction::transfer(
                    payer.key,
                    target_account.key,
                    rent_exempt_balance,
                );
            let transfer_account_infos = [
                payer.as_ref().clone(),
                target_account.clone(),
                system_program.as_ref().clone(),
            ];
            solana_program::program::invoke(
                &transfer_instruction,
                &transfer_account_infos,
            )?;
        }

        // 2) system_instruction::allocate enough space for the account
        let allocate_instruction =
            solana_program::system_instruction::allocate(
                target_account.key,
                space,
            );
        let allocate_account_infos =
            [target_account.clone(), system_program.as_ref().clone()];
        solana_program::program::invoke_signed(
            &allocate_instruction,
            &allocate_account_infos,
            &[pda_seeds],
        )?;

        // 3) assign our program as the owner
        let assign_owner_instruction =
            solana_program::system_instruction::assign(
                target_account.key,
                owner,
            );
        let assign_owner_accounts =
            [target_account.clone(), system_program.as_ref().clone()];
        solana_program::program::invoke_signed(
            &assign_owner_instruction,
            &assign_owner_accounts,
            &[pda_seeds],
        )?;
    }

    Ok(())
}

// pub fn delete_pda() -> ProgramResult {
//     // solana_program::system_instruction::
//     // let instruction = StableInstruction {

//     };
//     Ok(())
// }
