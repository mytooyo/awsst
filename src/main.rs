use clap::{Parser, Subcommand};

mod profile;
mod sts_client;
mod utils;

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Cli {
    // サブコマンド
    // セッショントークンを取得するためのコマンド
    #[clap(subcommand)]
    sub: Option<CliSubCommand>,

    // サブコマンドなしでセッショントークン取得を行うための任意オプション
    /// Name of the profile from which the session token is to be obtained
    #[clap(short, long)]
    profile: Option<String>,

    /// Forces the session token to be updated.
    #[clap(short, long)]
    force: bool,
}

// サブコマンドに対する処理
#[derive(Subcommand, Debug)]
enum CliSubCommand {
    /// Tool initialize
    Init {},
    /// Get session token
    Session {
        /// Profile to be used
        #[clap(short, long)]
        profile: Option<String>,

        /// Forces the session token to be updated.
        #[clap(short, long)]
        force: bool,
    },
    /// Same process as `aws configure`
    Configure {},
    /// Select the profile you want to use
    Use {
        /// Profile to be used
        #[clap(short, long)]
        profile: Option<String>,
    },
    /// Update porfile information
    Update {
        /// Profile name to be updated
        #[clap(short, long)]
        profile: Option<String>,
    },
    /// Remove profile from config
    Remove {
        /// Profile name to be remove
        #[clap(short, long)]
        profile: Option<String>,
    },
    /// List profile from credential
    Ls {},
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 引数を取得
    let args = Cli::parse();

    // サブコマンドが指定されている場合
    if let Some(sub) = args.sub {
        match sub {
            CliSubCommand::Init {} => {
                // 初期処理
                profile::initialize()?;
            }
            CliSubCommand::Session { profile, force } => {
                // セッショントークン取得
                profile::session_token(profile, force).await?;
            }
            CliSubCommand::Configure {} => {
                // configureで新たにプロファイルを生成
                profile::configure().await?
            }
            CliSubCommand::Update { profile } => {
                // セッショントークン取得
                profile::update(profile).await?;
            }
            CliSubCommand::Remove { profile } => {
                // プロファイル情報を削除
                profile::remove(profile).await?;
            }
            CliSubCommand::Use { profile } => {
                // プロファイルを選択
                profile::use_profile(profile)?;
            }
            CliSubCommand::Ls {} => {
                // プロファイル一覧表示
                profile::list()?;
            }
        }
    } else {
        // サブコマンドが指定されていない場合はセッショントーク取得を行う
        profile::session_token(args.profile, args.force).await?;
    }

    Ok(())
}
