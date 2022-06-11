use self::configs::{AWSConfigs, Config};
use self::credentials::{AWSCredentials, Credential};
use self::select::Selected;
use crate::profile::select::new_selected;
use crate::profile::select::AWSSelecteds;
use crate::utils;
use crate::utils::AWSFileManager;
use prettytable::{cell, format, row, Table};
use setenv::get_shell;
use std::process::exit;
pub mod configs;
pub mod configure;
pub mod credentials;
pub mod select;

pub const CONFIG_FILE_NAME: &str = "config";
pub const CREDENTIAL_FILE_NAME: &str = "credentials";
pub const TOOL_FILE_NAME: &str = "awsst";

/// 初期処理
///
pub fn initialize() -> Result<(), Box<dyn std::error::Error>> {
    let mut prompter = utils::prompt::Prompter::new();
    // 現在のプロファイルを取得
    let selected = read_tool(&mut prompter);
    let profile = selected.items.get("selected");

    if let Some(p) = profile {
        let shell = get_shell();
        // `export`を行う
        shell.setenv("AWS_PROFILE", p.name.clone());
    }

    Ok(())
}

/// セッショントークンを取得する
///
pub async fn session_token(
    profile: Option<String>,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut prompter = utils::prompt::Prompter::new();
    // Ctrl+Cのハンドラーを登録
    prompter.flush();

    // configファイル読み込み
    let configs = read_config(&mut prompter);

    // 対象のConfig名
    let selection = configs.selection_config_name(profile, &mut prompter);
    if selection.is_none() {
        return Ok(());
    }
    let name = selection.unwrap();
    let config = configs.items.get(&name).unwrap();

    // credentialsファイル読み込み
    let mut credentials = read_credential(&mut prompter);
    // 指定された`config`の名称の`credential`が存在するか確認
    if !credentials.exists_credential(name.clone()) {
        prompter.error("Oops... does not exists credential..");
        return Ok(());
    }

    // Credentialを取得
    let opt_cred = credentials.auth_credential(name.clone(), force);
    // Noneが返却された場合は期限内であるため、スキップ
    if opt_cred.is_none() {
        prompter.error("The credential has more than 3 hours remaining to expire.");
        prompter.error("For extensions, please force renewal with the [-f] option");
        return Ok(());
    }
    let mut cred = opt_cred.unwrap();

    // AWS Credentialを取得し、Config情報を更新
    let result = cred.sts_credential(config).await?;
    let new_cred = credentials
        .set_credential(config, cred.name.clone(), result)
        .await;

    // 更新後のCredentialが存在しない場合は失敗した可能性があるため、ここで終了
    if new_cred.is_none() {
        prompter.error("Oops... failed update credential..");
        return Ok(());
    }

    // 期限が設定されていたら最後に出力する
    if let Some(expired) = new_cred.unwrap().expiration {
        prompter.keyvalue("Success! Token expiration is ", expired.as_str());
    }

    // ファイル書き込み
    credentials.write()?;

    // 取得したセッショントークンのプロファイルを選択状態にする
    use_profile(Some(name))?;

    Ok(())
}

/// configureでconfig情報を設定する
pub async fn configure() -> Result<(), Box<dyn std::error::Error>> {
    let mut prompter = utils::prompt::Prompter::new();
    // Ctrl+Cのハンドラーを登録
    prompter.flush();

    // configファイル読み込み
    let mut configs = read_config(&mut prompter);
    // credentialsファイル読み込み
    let mut credentials = read_credential(&mut prompter);

    // 情報の入力をさせるためのダイアログを表示
    let mut aws_configure = configure::AWSConfigure::default();
    aws_configure.dialog_for_user(&mut prompter)?;

    // 同一名のconfigが存在するか確認し、存在した場合はエラーを表示して終了
    if configs.exists_config(aws_configure.profile.clone()) {
        prompter.error("already profile name");
        return Ok(());
    }

    // 新たなconfig情報を生成
    configs.add(Config {
        name: aws_configure.profile.clone(),
        region: aws_configure.region.clone(),
        output: aws_configure.output.clone(),
    });

    // 新たなcredential情報を生成
    credentials.add(Credential::from_configure(
        aws_configure.profile.clone(),
        aws_configure.access_key.clone(),
        aws_configure.secret_access_key.clone(),
        aws_configure.mfa_role.clone(),
    ));

    // ファイル書き込みを行う
    configs.write()?;
    credentials.write()?;

    // 追加したプロファイルを選択状態にする
    use_profile(Some(aws_configure.profile.clone()))?;

    Ok(())
}

/// configureで登録した情報を更新
pub async fn update(profile: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut prompter = utils::prompt::Prompter::new();
    // Ctrl+Cのハンドラーを登録
    prompter.flush();

    // configファイル読み込み
    let mut configs = read_config(&mut prompter);
    // credentialsファイル読み込み
    let mut credentials = read_credential(&mut prompter);

    // 対象のConfig名を取得
    let selection = configs.selection_config_name(profile, &mut prompter);
    if selection.is_none() {
        return Ok(());
    }
    let name = selection.unwrap();
    let config = configs.items.get_mut(&name).unwrap();
    // 指定された`config`の名称の`credential`が存在するか確認
    if !credentials.exists_credential(name.clone()) {
        prompter.error("Oops... does not exists credential..");
        return Ok(());
    }

    // Credentialを取得
    let opt_cred = credentials.suffix_credential(name.clone());
    let mut cred = opt_cred.unwrap();

    // reconfigureの処理を行う
    let mut aws_configure = configure::AWSConfigure::from_conf(config.clone(), Some(cred.clone()));
    aws_configure.dialog_for_user(&mut prompter)?;

    // データを上書き
    config.region = aws_configure.region;
    config.output = aws_configure.output;
    cred.access_key_id = Some(aws_configure.access_key);
    cred.secret_access_key = Some(aws_configure.secret_access_key);
    cred.mfa_serial = aws_configure.mfa_role;

    // ファイル書き込みを行う
    configs.write()?;
    credentials.write()?;

    Ok(())
}

/// Profileから削除
pub async fn remove(profile: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut prompter = utils::prompt::Prompter::new();
    // Ctrl+Cのハンドラーを登録
    prompter.flush();

    // configファイル読み込み
    let mut configs = read_config(&mut prompter);
    // credentialsファイル読み込み
    let mut credentials = read_credential(&mut prompter);

    // 対象のConfig名
    let selection = configs.selection_config_name(profile, &mut prompter);
    if selection.is_none() {
        return Ok(());
    }
    let name = selection.unwrap();

    // ConfigとCredentialを削除
    configs.remove(name.clone());
    credentials.remove(name);

    // ファイル書き込みを行う
    configs.write()?;
    credentials.write()?;

    prompter.standard("complete! deleted profile.");

    Ok(())
}

/// 利用するプロファイルを選択
pub fn use_profile(profile: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut prompter = utils::prompt::Prompter::new();
    // Ctrl+Cのハンドラーを登録
    prompter.flush();

    // configファイル読み込み
    let configs = read_config(&mut prompter);

    // 対象のConfig名
    let selection = configs.selection_config_name(profile, &mut prompter);
    if selection.is_none() {
        return Ok(());
    }
    let name = selection.unwrap();

    // プロファイル情報を設定
    let config = configs.items.get(&name).unwrap();
    _set_tool_file(config)?;

    // Set the information of the selected profile
    // in the environment variable at the end of execution
    let shell = get_shell();
    // `export`を行う
    shell.setenv("AWS_PROFILE", name);

    Ok(())
}

/// 登録されているCredentialファイルからリストを表示する
pub fn list() -> Result<(), Box<dyn std::error::Error>> {
    let mut prompter = utils::prompt::Prompter::new();
    // Ctrl+Cのハンドラーを登録
    prompter.flush();

    // credentialsファイル読み込み
    let credentials = read_credential(&mut prompter);

    // ベースとなるcredentialのみ取得
    let bases = credentials.bases;

    // 現在のプロファイルを取得
    let selected = read_tool(&mut prompter);
    let profile = selected.items.get("selected");

    // 表示するためのテーブル
    let mut table = Table::new();
    table.set_titles(row![
        cell!(""),
        cell!("NAME"),
        cell!("ACCOUNT"),
        cell!("MFA"),
        cell!("ROLE ARN"),
        cell!("EXPIRATION")
    ]);
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);

    // データを追加
    for cred in bases {
        let _profile = profile;
        let used = if _profile.is_some() && _profile.unwrap().name == cred.name {
            "*".to_string()
        } else {
            "".to_string()
        };
        // アカウント情報
        let account = cred.account.unwrap_or_else(|| "".to_string());
        // MFA
        let mfa = cred.mfa_serial.unwrap_or_else(|| "".to_string());
        // Role arn
        let role = cred.role_arn.unwrap_or_else(|| "".to_string());
        // 期限
        let expiration = cred.expiration.unwrap_or_else(|| "".to_string());

        // テーブルに追加
        table.add_row(row![
            cell!(used),
            cell!(cred.name),
            cell!(account),
            cell!(mfa),
            cell!(role),
            cell!(expiration),
        ]);
    }

    // コンソールに出力
    let mut writer = super::utils::prompt::StringWriter::new();
    table.print(&mut writer)?;
    let data = writer
        .as_vec()
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>();
    let out = data[..(data.len() - 1)].to_vec();
    prompter.standard(out.join("\n").as_str());

    Ok(())
}

/// `config`ファイル読み込み
pub fn read_config(prompter: &mut utils::prompt::Prompter) -> AWSConfigs {
    // ファイル読み込み
    let result = utils::file::read::<AWSConfigs, Config>(CONFIG_FILE_NAME);
    // 読み込みに失敗した場合は`aws configure`を行うかどうかを確認
    if result.is_err() {
        prompter.error(format!("{}", result.err().unwrap()).as_str());
        exit(1);
    }

    // AWSConfigs返却
    result.unwrap()
}

/// `credentials`ファイル読み込み
pub fn read_credential(prompter: &mut utils::prompt::Prompter) -> AWSCredentials {
    // ファイル読み込み
    let result = utils::file::read::<AWSCredentials, Credential>(CREDENTIAL_FILE_NAME);
    // 読み込みに失敗した場合は`aws configure`を行うかどうかを確認
    if result.is_err() {
        prompter.error(format!("{}", result.err().unwrap()).as_str());
        exit(1);
    }

    // AWSCredentials返却
    result.unwrap()
}

/// ツール用のファイル読み込み
pub fn read_tool(prompter: &mut utils::prompt::Prompter) -> AWSSelecteds {
    // ファイル読み込み
    let result = utils::file::read::<AWSSelecteds, Selected>(TOOL_FILE_NAME);
    // 読み込みに失敗した場合は`aws configure`を行うかどうかを確認
    if result.is_err() {
        prompter.error(format!("{}", result.err().unwrap()).as_str());
        exit(1);
    }

    // AWSCredentials返却
    result.unwrap()
}

/// ツール用のファイルを設定
fn _set_tool_file(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let selecteds = new_selected(config.name.clone(), config.region.clone());
    // ファイル書き込みを行う
    selecteds.write()
}
