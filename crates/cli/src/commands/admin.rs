use alloy::hex;
use clap::Subcommand;
use contender_core::{
    db::DbOps,
    error::ContenderError,
    generator::RandSeed,
};
use crate::util::data_dir;
use tracing::{info};
use alloy::signers::local::PrivateKeySigner;
use alloy::primitives::FixedBytes;

#[derive(Debug, Subcommand)]
pub enum AdminCommand {
    #[command(
        name = "accounts",
        about = "Print addresses generated by RandSeed for a given from_pool"
    )]
    Accounts {
        /// From pool to generate accounts for
        #[arg(short = 'f', long)]
        from_pool: String,

        /// Number of signers to generate
        #[arg(short = 'n', long, default_value = "10")]
        num_signers: usize,

        /// Acknowledge that printing addresses may expose sensitive information
        #[arg(long)]
        confirm: bool,
    },

    #[command(
        name = "latest-run-id",
        about = "Print the max run id in the DB"
    )]
    LatestRunId,

    #[command(
        name = "seed",
        about = "Print the contents of ~/.contender/seed"
    )]
    Seed,
}

/// Reads and validates the seed file
fn read_seed_file() -> Result<Vec<u8>, ContenderError> {
    let data_dir = data_dir().map_err(|e| {
        ContenderError::GenericError(
            "Failed to get data dir",
            e.to_string()
        )
    })?;
    let seed_path = format!("{}/seed", data_dir);
    let seed_hex = std::fs::read_to_string(&seed_path)
        .map_err(|e| ContenderError::AdminError(
            "Failed to read seed file",
            format!("at {}: {}", seed_path, e)
        ))?;
    let decoded = hex::decode(seed_hex.trim())
        .map_err(|_| ContenderError::AdminError(
            "Invalid hex data in seed file",
            format!("at {}", seed_path)
        ))?;
    if decoded.is_empty() {
        return Err(ContenderError::AdminError(
            "Empty seed file",
            format!("at {}", seed_path)
        ));
    }
    Ok(decoded)
}

/// Prompts for confirmation before displaying sensitive information
fn confirm_sensitive_operation(_operation: &str) -> Result<(), ContenderError> {
    println!("WARNING: This command will display sensitive information.");
    println!("This information should not be shared or exposed in CI environments.");
    println!("Press Enter to continue or Ctrl+C to cancel...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)
        .map_err(|e| ContenderError::AdminError(
            "Failed to read input",
            format!("{}", e)
        ))?;
    Ok(())
}

/// Handles the accounts subcommand
async fn handle_accounts(
    from_pool: String,
    num_signers: usize,
    confirm: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if confirm {
        confirm_sensitive_operation("displaying account addresses")?;
    }

    let seed_bytes = read_seed_file()?;
    let seed = RandSeed::seed_from_bytes(&seed_bytes);
    print_accounts_for_pool(&from_pool, num_signers, &seed)?;
    Ok(())
}

/// Prints accounts for a specific pool
fn print_accounts_for_pool(pool: &str, num_signers: usize, seed: &RandSeed) -> Result<(), ContenderError> {
    info!("Generating addresses for pool: {}", pool);
    for i in 0..num_signers {
        let key_bytes = seed.derive_signing_key(pool, i)
            .map_err(|e| ContenderError::AdminError(
                "Failed to derive signing key",
                format!("{}", e)
            ))?;
        let signer = PrivateKeySigner::from_bytes(&FixedBytes::from_slice(&key_bytes))
            .map_err(|e| ContenderError::AdminError(
                "Failed to create signing key",
                format!("{}", e)
            ))?;
        let address = signer.address();
        info!("Signer {}: {}", i, address);
    }
    Ok(())
}

/// Handles the seed subcommand
async fn handle_seed() -> Result<(), Box<dyn std::error::Error>> {
    confirm_sensitive_operation("displaying seed value")?;
    let seed_bytes = read_seed_file()?;
    println!("{}", hex::encode(seed_bytes));
    Ok(())
}

pub async fn handle_admin_command(command: AdminCommand, db: impl DbOps) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        AdminCommand::Accounts {
            from_pool,
            num_signers,
            confirm,
        } => handle_accounts(from_pool, num_signers, confirm).await,
        AdminCommand::LatestRunId => {
            let num_runs = db.num_runs()?;
            println!("Latest run ID: {}", num_runs);
            Ok(())
        }
        AdminCommand::Seed => handle_seed().await,
    }
}