use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    bpf_loader_upgradeable::{self, UpgradeableLoaderState},
    pubkey::Pubkey,
};
use std::{
    borrow::Cow,
    env,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Debug, Parser)]
pub struct Retrieve {
    #[command(subcommand)]
    command: Command,

    /// The url/endpoint to use for any rpc requests.
    #[arg(
        long,
        short = 'u',
        default_value = "http://api.mainnet-beta.solana.com",
        global = true
    )]
    rpc_url: String,
}

#[derive(Debug, Parser)]
pub enum Command {
    #[clap()]
    Program(Program),

    #[clap()]
    Account(Account),
}

#[derive(Debug, Parser)]
pub struct Program {
    #[clap()]
    pub name: String,

    #[clap(value_parser = Pubkey::from_str)]
    pub program: Pubkey,
}

#[derive(Debug, Parser)]
pub struct Account {
    #[clap(value_parser = Pubkey::from_str)]
    pub account: Pubkey,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), &'static str> {
    let retrieve = Retrieve::parse();
    match retrieve.command {
        Command::Program(Program { name, program }) => {
            retrieve_program(&program, name, retrieve.rpc_url).await
        }
        Command::Account(account) => {
            retrieve_account(&account.account, retrieve.rpc_url).await
        }
    }
}

#[allow(deprecated)]
pub async fn retrieve_account(
    account: &Pubkey,
    rpc_endpoint: impl Into<Cow<'_, str>>,
) -> Result<(), &'static str> {
    // Obtain account at specified address from rpc
    let rpc_client = RpcClient::new(rpc_endpoint.into().into());
    let Ok(account_data) = rpc_client
        .get_account_data(&account)
        .await
    else {
        return Err("Failed to fetch account".into());
    };

    println!(
        "account {account} data: {}",
        base64::encode(account_data)
    );

    Ok(())
}

pub async fn retrieve_program(
    program: &Pubkey,
    mut name: String,
    rpc_endpoint: impl Into<Cow<'_, str>>,
) -> Result<(), &'static str> {
    // Obtain account at specified address from rpc
    let rpc_client = RpcClient::new(rpc_endpoint.into().into());
    let program_buffer = Pubkey::find_program_address(
        &[program.as_ref()],
        &bpf_loader_upgradeable::ID,
    )
    .0;

    let Ok(account_data) = rpc_client
        .get_account_data(&program_buffer)
        .await
    else {
        return Err("Failed to fetch program".into());
    };
    println!("Fetched program buffer data");

    // Write program to file
    let deploy_dir = workspace_dir()
        .join("auxiliary")
        .join("tests")
        .join("fixtures");
    std::fs::create_dir_all(&deploy_dir)
        .map_err(|_| "Failed to check or create deploy dir")?;
    name.push_str(".so");
    let filename = deploy_dir.join(name);
    let Ok(mut file) = std::fs::File::create(&filename) else {
        return Err(
            "failed to create file for executable at {filename}",
        );
    };
    let offset = UpgradeableLoaderState::size_of_programdata_metadata();
    file.write_all(&account_data[offset..])
        .map_err(|_| "Failed to write data to file")?;
    println!("Wrote to {}", filename.display());

    Ok(())
}

pub fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(
        std::str::from_utf8(&output)
            .unwrap()
            .trim(),
    );
    cargo_path
        .parent()
        .unwrap()
        .to_path_buf()
}
