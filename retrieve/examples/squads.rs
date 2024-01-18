use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

#[tokio::main(flavor = "current_thread")]
#[allow(deprecated)]
async fn main() {
    // TODO: prod
    // const SQUADS_V4_PROGRAM: Pubkey = solana_sdk::pubkey!(
    //     "SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf"
    // );

    const SQUADS_V4_PROGRAM: Pubkey = solana_sdk::pubkey!(
        "STAG3xkFMyVK3sRtQhipsKuLpRGbgospDpVdNyJqDpS"
    );

    // #[account(
    //     mut,
    //     seeds = [SEED_PREFIX, SEED_PROGRAM_CONFIG],
    //     bump,
    // )]
    // pub program_config: Account<'info, crate::state::ProgramConfig>,
    pub const SEED_PREFIX: &[u8] = b"multisig";
    pub const SEED_PROGRAM_CONFIG: &[u8] = b"program_config";
    let (program_config, _bump) = Pubkey::find_program_address(
        &[SEED_PREFIX, SEED_PROGRAM_CONFIG],
        &SQUADS_V4_PROGRAM,
    );
    println!("program config address = {program_config}");

    // Obtain data
    let client =
        RpcClient::new("https://api.mainnet-beta.solana.com".into());
    let program_config_data = client
        .get_account_data(&program_config)
        .await
        .unwrap();

    println!(
        "program config data = {}",
        base64::encode(&program_config_data)
    );

    // /// Global program configuration account.
    // #[account]
    // #[derive(InitSpace)]
    // pub struct ProgramConfig {
    //     /// The authority which can update the config.
    //     pub authority: Pubkey,
    //     /// The lamports amount charged for creating a new multisig
    // account.     /// This fee is sent to the `treasury` account.
    //     pub multisig_creation_fee: u64,
    //     /// The treasury account to send charged fees to.
    //     pub treasury: Pubkey,
    //     /// Reserved for future use.
    //     pub _reserved: [u8; 64],
    // }

    let offset = 8 // discriminator
        + 32  // authority 
        + 8; // fee
    let treasury_bytes: [u8; 32] = program_config_data
        [offset..offset + 32]
        .try_into()
        .unwrap();
    let treasury = Pubkey::new_from_array(treasury_bytes);
    println!("\ntreasury = {treasury}");

    let client =
        RpcClient::new("https://api.mainnet-beta.solana.com".into());
    let treasury_data = client
        .get_account_data(&treasury)
        .await
        .unwrap();

    println!("treasury data = {}", base64::encode(&treasury_data));
}
