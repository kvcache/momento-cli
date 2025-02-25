use std::{panic, process::exit};

use clap::StructOpt;
#[cfg(feature = "login")]
use commands::login::LoginMode;
use env_logger::Env;
use error::CliError;
use log::{debug, error, LevelFilter};
use utils::console::console_info;
use utils::user::get_creds_and_config;

#[cfg(feature = "login")]
use crate::utils::user::clobber_session_token;

mod commands;
mod config;
mod error;
mod utils;

#[derive(Debug, StructOpt)]
#[clap(version)]
#[structopt(about = "CLI for Momento APIs", name = "momento")]
struct Momento {
    #[structopt(name = "verbose", global = true, long)]
    verbose: bool,

    #[structopt(
        long,
        short,
        default_value = "default",
        global = true,
        help = "User profile"
    )]
    profile: String,

    #[structopt(subcommand)]
    command: Subcommand,
}

#[derive(Debug, StructOpt)]
enum Subcommand {
    #[structopt(about = "Interact with caches")]
    Cache {
        #[structopt(subcommand)]
        operation: CacheCommand,
    },
    #[structopt(about = "Configure credentials")]
    Configure {
        #[structopt(long, short)]
        quick: bool,
    },
    #[structopt(about = "Manage accounts")]
    Account {
        #[structopt(subcommand)]
        operation: AccountCommand,
    },
    #[structopt(about = "Manage signing keys")]
    SigningKey {
        #[structopt(subcommand)]
        operation: SigningKeyCommand,
    },
    #[cfg(feature = "login")]
    #[structopt(
        about = "*Construction Zone* We're working on this! *Construction Zone* Log in to manage your Momento account"
    )]
    Login {
        #[clap(arg_enum, default_value = "browser")]
        via: LoginMode,
    },
}

#[derive(Debug, StructOpt)]
enum SigningKeyCommand {
    #[structopt(about = "Create a signing key")]
    Create {
        #[structopt(
            long = "ttl",
            short = 't',
            default_value = "86400",
            help = "Duration, in minutes, that the signing key will be valid"
        )]
        ttl_minutes: u32,

        #[structopt(long = "endpoint", short = 'e')]
        endpoint: Option<String>,
    },

    #[structopt(about = "Revoke the signing key")]
    Revoke {
        #[structopt(long = "key-id", short, help = "Signing Key ID")]
        key_id: String,

        #[structopt(long = "endpoint", short = 'e')]
        endpoint: Option<String>,
    },

    #[structopt(about = "List all signing keys")]
    List {
        #[structopt(long = "endpoint", short = 'e')]
        endpoint: Option<String>,
    },
}

#[derive(Debug, StructOpt)]
enum AccountCommand {
    #[structopt(about = "Sign up for Momento")]
    Signup {
        #[structopt(subcommand)]
        signup_operation: CloudSignupCommand,
    },
}

#[derive(Debug, StructOpt)]
enum CloudSignupCommand {
    #[structopt(about = "Signup for Momento on GCP")]
    Gcp {
        #[structopt(long, short)]
        email: String,
        #[structopt(long, short, value_name = "us-east1 or asia-northeast1")]
        region: String,
    },
    #[structopt(about = "Signup for Momento on AWS")]
    Aws {
        #[structopt(long, short)]
        email: String,
        #[structopt(long, short, value_name = "us-west-2, us-east-1, or ap-northeast-1")]
        region: String,
    },
}

#[derive(Debug, StructOpt)]
enum CacheCommand {
    #[structopt(about = "Create a cache")]
    Create {
        #[structopt(long = "name", short = 'n')]
        cache_name: String,

        #[structopt(long = "endpoint", short = 'e')]
        endpoint: Option<String>,
    },

    #[structopt(about = "Store a given item in the cache")]
    Set {
        #[structopt(long = "name", short = 'n')]
        cache_name: Option<String>,
        // TODO: Add support for non-string key-value
        #[structopt(long, short)]
        key: String,

        #[structopt(long, short)]
        value: String,

        #[structopt(
            long = "ttl",
            short = 't',
            help = "Max time, in seconds, that the item will be stored in cache"
        )]
        ttl_seconds: Option<u64>,

        #[structopt(long = "endpoint", short = 'e')]
        endpoint: Option<String>,
    },

    #[structopt(about = "Get an item from the cache")]
    Get {
        #[structopt(long = "name", short = 'n')]
        cache_name: Option<String>,
        // TODO: Add support for non-string key-value
        #[structopt(long, short)]
        key: String,

        #[structopt(long = "endpoint", short = 'e')]
        endpoint: Option<String>,
    },

    #[structopt(about = "Delete the cache")]
    Delete {
        #[structopt(long = "name", short = 'n')]
        cache_name: String,

        #[structopt(long = "endpoint", short = 'e')]
        endpoint: Option<String>,
    },

    #[structopt(about = "List all caches")]
    List {
        #[structopt(long = "endpoint", short = 'e')]
        endpoint: Option<String>,
    },
}

async fn entrypoint() -> Result<(), CliError> {
    let args = Momento::parse();

    let log_level = if args.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Off
    }
    .as_str();

    env_logger::Builder::from_env(
        Env::default()
            .default_filter_or(log_level)
            .default_write_style_or("always"),
    )
    .init();

    match args.command {
        Subcommand::Cache { operation } => match operation {
            CacheCommand::Create {
                cache_name,
                endpoint,
            } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::cache::cache_cli::create_cache(cache_name.clone(), creds.token, endpoint)
                    .await?;
                debug!("created cache {cache_name}")
            }
            CacheCommand::Set {
                cache_name,
                key,
                value,
                ttl_seconds,
                endpoint,
            } => {
                let (creds, config) = get_creds_and_config(&args.profile).await?;
                commands::cache::cache_cli::set(
                    cache_name.unwrap_or(config.cache),
                    creds.token,
                    key,
                    value,
                    ttl_seconds.unwrap_or(config.ttl),
                    endpoint,
                )
                .await?
            }
            CacheCommand::Get {
                cache_name,
                key,
                endpoint,
            } => {
                let (creds, config) = get_creds_and_config(&args.profile).await?;
                commands::cache::cache_cli::get(
                    cache_name.unwrap_or(config.cache),
                    creds.token,
                    key,
                    endpoint,
                )
                .await?;
            }
            CacheCommand::Delete {
                cache_name,
                endpoint,
            } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::cache::cache_cli::delete_cache(cache_name.clone(), creds.token, endpoint)
                    .await?;
                debug!("deleted cache {}", cache_name)
            }
            CacheCommand::List { endpoint } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::cache::cache_cli::list_caches(creds.token, endpoint).await?
            }
        },
        Subcommand::Configure { quick } => {
            commands::configure::configure_cli::configure_momento(quick, &args.profile).await?
        }
        Subcommand::Account { operation } => match operation {
            AccountCommand::Signup { signup_operation } => match signup_operation {
                CloudSignupCommand::Gcp { email, region } => {
                    commands::account::signup_user(email, "gcp".to_string(), region).await?
                }
                CloudSignupCommand::Aws { email, region } => {
                    commands::account::signup_user(email, "aws".to_string(), region).await?
                }
            },
        },
        Subcommand::SigningKey { operation } => match operation {
            SigningKeyCommand::Create {
                ttl_minutes,
                endpoint,
            } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::signingkey::signingkey_cli::create_signing_key(
                    ttl_minutes,
                    creds.token,
                    endpoint,
                )
                .await?;
            }
            SigningKeyCommand::Revoke { key_id, endpoint } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::signingkey::signingkey_cli::revoke_signing_key(
                    key_id.clone(),
                    creds.token,
                    endpoint,
                )
                .await?;
                debug!("revoked signing key {}", key_id)
            }
            SigningKeyCommand::List { endpoint } => {
                let (creds, _config) = get_creds_and_config(&args.profile).await?;
                commands::signingkey::signingkey_cli::list_signing_keys(creds.token, endpoint)
                    .await?
            }
        },
        #[cfg(feature = "login")]
        Subcommand::Login { via } => match commands::login::login(via).await {
            momento::momento::auth::LoginResult::LoggedIn(logged_in) => {
                debug!("{}", logged_in.session_token);
                clobber_session_token(
                    Some(logged_in.session_token.to_string()),
                    logged_in.valid_for_seconds,
                )
                .await?;
                console_info!("Login valid for {}m", logged_in.valid_for_seconds / 60);
            }
            momento::momento::auth::LoginResult::NotLoggedIn(not_logged_in) => {
                return Err(CliError {
                    msg: not_logged_in.error_message,
                })
            }
        },
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    panic::set_hook(Box::new(move |info| {
        error!("{}", info);
    }));

    if let Err(e) = entrypoint().await {
        console_info!("{}", e);
        exit(1)
    }
}
