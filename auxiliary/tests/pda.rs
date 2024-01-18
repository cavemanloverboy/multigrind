use auxiliary::{
    instruction::AuxiliaryAccount,
    sdk::{
        auxiliary_pda, multisig_pda_from_auxiliary, InstructionBuilder,
    },
    squads_sdk::SQUADS_V4_PROGRAM,
};
use solana_program::{
    hash::Hash, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey,
    system_program,
};
use solana_program_test::{processor, BanksClient, ProgramTest};
use solana_sdk::{
    signature::Keypair, signer::Signer, system_transaction::transfer,
    transaction::Transaction,
};

pub const SEED_PREFIX: &[u8] = b"multisig";
pub const SEED_MULTISIG: &[u8] = b"multisig";
pub const SEED_TRANSACTION: &[u8] = b"transaction";
pub const SEED_PROPOSAL: &[u8] = b"proposal";
// TODO: change to prod config and treasury
const PROGRAM_CONFIG: Pubkey = solana_program::pubkey!(
    "8z2aG86nbnFBTA5tQin1YzVdhqyf84Xco2sqHy5ZyA2N"
);
const TREASURY: Pubkey = solana_program::pubkey!(
    "HM5y4mz3Bt9JY9mr1hkyhnvqxSH4H2u2451j7Hc2dtvK"
);

#[tokio::test]
async fn test_pda_init() {
    // "Bank..." (for the staging program)
    // Will probably be a random thing for the real SQDS program
    pub const AUXILIARY_SEED: [u8; 16] = [
        86, 164, 144, 192, 246, 19, 69, 111, 187, 210, 211, 104, 252,
        63, 154, 181,
    ];
    let target = "Bank".to_string();
    let target_len = target.len();

    // // "Jito..." (for the prod program)
    // pub const AUXILIARY_SEED: [u8; 16] = [
    //     29, 201, 66, 65, 120, 69, 185, 62, 133, 185, 198, 91, 203,
    // 30,     110, 209,
    // ];
    // let target = "Jito".to_string();
    // let target_len = target.len();

    // Initialize program test environment
    let (mut banks, payer, hash) = setup_program_test_env().await;

    // Get auxiliary and multisig pdas
    let (auxiliary_pda, auxiliary_bump) =
        auxiliary_pda(&AUXILIARY_SEED);
    let (multisig_pda, _multisig_bump) =
        multisig_pda_from_auxiliary(&auxiliary_pda);
    println!("using auxiliary {auxiliary_pda}");
    println!("target multisig {multisig_pda}");

    // STEP ONE: Seller initializes multisig
    let instruction = InstructionBuilder::initialize_multisig(
        payer.pubkey(),
        AUXILIARY_SEED,
        target,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        hash,
    );
    banks
        .process_transaction(transaction)
        .await
        .unwrap();
    // POST STEP ONE: Check auxiliary content
    let auxiliary_account_data = banks
        .get_account(auxiliary_pda)
        .await
        .unwrap()
        .unwrap();
    let &AuxiliaryAccount {
        multisig: on_chain_multisig,
        seed: on_chain_seed,
        bump: on_chain_bump,
        len: on_chain_len,
    } = bytemuck::from_bytes(&auxiliary_account_data.data);
    assert_eq!(on_chain_bump, auxiliary_bump);
    assert_eq!(on_chain_seed, AUXILIARY_SEED);
    assert_eq!(on_chain_len, target_len as u8);
    assert_eq!(on_chain_multisig, multisig_pda.as_ref());

    // Wild buyer appears
    let buyer = Keypair::new();
    let fund_buyer =
        transfer(&payer, &buyer.pubkey(), 100 * LAMPORTS_PER_SOL, hash);
    banks
        .process_transaction(fund_buyer)
        .await
        .unwrap();

    // STEP TWO: Buyer buys multisig
    let instruction = InstructionBuilder::buy_multisig(
        auxiliary_pda,
        multisig_pda,
        buyer.pubkey(),
        // TODO: recipient
        payer.pubkey(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&buyer.pubkey()),
        &[&buyer],
        hash,
    );
    banks
        .process_transaction(transaction)
        .await
        .unwrap();
    // POST STEP TWO:
    // 1) Retrieve multisig and validate contents
    //    a) Config Authority is unset
    //    b) Num members = 1
    //    c) Only member = buyer
    // 2) Auxiliary account no longer exists
    // 3) TODO check user/buyer balances

    // 1) Retrieve multisig and validate contents
    let multisig_account = banks
        .get_account(multisig_pda)
        .await
        .unwrap()
        .unwrap();
    println!("Final status of multisig");
    let authority_key = Pubkey::new_from_array(
        multisig_account.data[MULTISIG_ACCOUNT_AUTHORITY_OFFSET
            ..MULTISIG_ACCOUNT_AUTHORITY_OFFSET + 32]
            .try_into()
            .unwrap(),
    );
    let num_members = u32::from_le_bytes(
        multisig_account.data[MULTISIG_ACCOUNT_MEMBER_OFFSET
            ..MULTISIG_ACCOUNT_MEMBER_OFFSET + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    println!("Number of members: {num_members}");
    assert_eq!(num_members, 1, "member set not equal to only buyer");
    println!("Multisig config_authority: {authority_key}");
    assert_eq!(
        authority_key,
        Pubkey::default(),
        "config authority not unset"
    );
    for (i, chunk) in multisig_account.data
        [MULTISIG_ACCOUNT_MEMBER_OFFSET + 4..]
        .chunks(33)
        .take(num_members)
        .enumerate()
    {
        let key =
            Pubkey::new_from_array(chunk[..32].try_into().unwrap());
        let expected = if i == 0 {
            buyer.pubkey()
        } else {
            Pubkey::default()
        };
        println!("Member {i}: {key} vs expected {expected}");
        assert_eq!(key, expected, "member {i} {key} != {expected}")
    }

    // 2) Auxiliary account no longer exists
    assert!(banks
        .get_account(auxiliary_pda)
        .await
        .unwrap()
        .is_none())

    // 3) TODO: Check user/buyer balances
}

async fn setup_program_test_env() -> (BanksClient, Keypair, Hash) {
    // Initialize program test
    let mut program_test = ProgramTest::new(
        "auxiliary",
        auxiliary::ID,
        processor!(auxiliary::process_instruction),
    );
    program_test.prefer_bpf(true);
    // Add squads v4 program
    program_test.add_program("squadsv4", SQUADS_V4_PROGRAM, None);

    // Add squads program config and treasury accounts

    // First add program config account
    program_test.add_account_with_base64_data(
        PROGRAM_CONFIG,
        1 * LAMPORTS_PER_SOL,
        SQUADS_V4_PROGRAM,
        "xNJa55CVjD/y4DkJcttfEtTBz1IxzaY5194JtJU/4aL4ooeXbGx+0AAAAAAAAAAA8uA5CXLbXxLUwc9SMc2mOdfeCbSVP+Gi+KKHl2xsftAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    );
    // Then add treasury account
    program_test.add_account_with_base64_data(
        TREASURY,
        1 * LAMPORTS_PER_SOL,
        system_program::ID,
        "",
    );

    program_test.start().await
}

const MULTISIG_ACCOUNT_AUTHORITY_OFFSET: usize = 8  + // anchor account discriminator
32; // create_key

const MULTISIG_ACCOUNT_MEMBER_OFFSET: usize = 8  + // anchor account discriminator
32 + // create_key
32 + // config_authority
2  + // threshold
4  + // time_lock
8  + // transaction_index
8  + // stale_transaction_index
1  + // rent_collector Option discriminator
// MAGNETAR FIELDS: this offset is removed since we set it to none
// 32 + // rent_collector (always 32 bytes, even if None, just to keep the realloc logic simpler)
1; // bump
   // 4; // members vector length
