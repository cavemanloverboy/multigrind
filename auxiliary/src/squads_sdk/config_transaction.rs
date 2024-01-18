use solana_program::pubkey::Pubkey;

use super::Member;

pub struct ConfigTransactionCreateArgs {
    // // borsh size w/ 1 remove action = 4 + 1 + 32 = 37
    // pub actions: Vec<ConfigAction>,
    // // borsh size when none = 1
    // pub memo: Option<String>,
}

// Includes discriminator + args
const TRANSFER_OWNERSHIP_CONFIG_TRANSACTION_SIZE: usize = 8 + (37 + 1);

impl ConfigTransactionCreateArgs {
    pub fn remove_member_borsh_with(
        auxiliary: &Pubkey,
    ) -> [u8; TRANSFER_OWNERSHIP_CONFIG_TRANSACTION_SIZE] {
        let mut data = [0; TRANSFER_OWNERSHIP_CONFIG_TRANSACTION_SIZE];

        // Write discriminator
        // sha256(b"global:config_transaction_create")
        data[0..8]
            .copy_from_slice(&[155, 236, 87, 228, 137, 75, 81, 39]);

        // Write one action [u32 le length, 1_u8 enum disc, pubkey]
        data[8..12].copy_from_slice(&1_u32.to_le_bytes());
        data[12] = 1;
        data[13..45].copy_from_slice(&auxiliary.as_ref());

        // Write memo = None = 0
        data[45] = 0;

        data
    }
}

#[non_exhaustive]
pub enum ConfigAction {
    /// Add a new member to the multisig.
    AddMember { new_member: Member },
    /// Remove a member from the multisig.
    RemoveMember { old_member: Pubkey },
    /// Change the `threshold` of the multisig.
    ChangeThreshold { new_threshold: u16 },
    /// Change the `time_lock` of the multisig.
    SetTimeLock { new_time_lock: u32 },
    /// Change the `time_lock` of the multisig.
    AddSpendingLimit {
        /// Key that is used to seed the SpendingLimit PDA.
        create_key: Pubkey,
        /// The index of the vault that the spending limit is for.
        vault_index: u8,
        /// The token mint the spending limit is for.
        mint: Pubkey,
        /// The amount of tokens that can be spent in a period.
        /// This amount is in decimals of the mint,
        /// so 1 SOL would be `1_000_000_000` and 1 USDC would be
        /// `1_000_000`.
        amount: u64,
        /// The reset period of the spending limit.
        /// When it passes, the remaining amount is reset, unless it's
        /// `Period::OneTime`.
        period: Period,
        /// Members of the multisig that can use the spending limit.
        /// In case a member is removed from the multisig, the spending
        /// limit will remain existent (until explicitly
        /// deleted), but the removed member will not be able to use it
        /// anymore.
        members: Vec<Pubkey>,
        /// The destination addresses the spending limit is allowed to
        /// sent funds to. If empty, funds can be sent to any
        /// address.
        destinations: Vec<Pubkey>,
    },
    /// Remove a spending limit from the multisig.
    RemoveSpendingLimit { spending_limit: Pubkey },
    /// Set the `rent_collector` config parameter of the multisig.
    SetRentCollector { new_rent_collector: Option<Pubkey> },
}

pub enum Period {
    /// The spending limit can only be used once.
    OneTime,
    /// The spending limit is reset every day.
    Day,
    /// The spending limit is reset every week (7 days).
    Week,
    /// The spending limit is reset every month (30 days).
    Month,
}

/// No args
pub struct ConfigTransactionExecute;

impl ConfigTransactionExecute {
    pub fn borsh() -> [u8; CONFIG_TRANSACTION_EXECUTE_SIZE] {
        // only discriminator
        //sha256(b"global:config_transaction_execute")
        [114, 146, 244, 189, 252, 140, 36, 40]
    }
}

/// Only discriminator
const CONFIG_TRANSACTION_EXECUTE_SIZE: usize = 8;
