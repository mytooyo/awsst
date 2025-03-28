use dialoguer::{theme::ColorfulTheme, Input};
use regex::Regex;

use super::profile;

pub struct MFAInfo {
    arn: Option<String>,
    code: Option<String>,
}

pub struct AssumeRoleReq {
    arn: Option<String>,
    session_name: String,
}

/// AWSへSTSリクエスト
pub async fn aws_sts_request(
    config: &profile::configs::Config,
    credential: profile::credentials::Credential,
) -> Result<aws_sdk_sts::types::Credentials, aws_sdk_sts::Error> {
    // MFA情報取得
    let mfa = get_mfa_info(credential.mfa_serial.clone());

    // MFAが設定済みだが、コードが入力されていない場合は終了
    if credential.mfa_serial.is_some() && mfa.code.is_none() {
        panic!("No code entered");
    }

    // AssumeRoleが指定されている場合
    if let Some(assule_role) = credential.role_arn {
        let re_session_name = Regex::new(r"arn:aws:iam::[0-9]*:role/(.*)").unwrap();
        let session_name = match re_session_name.captures(assule_role.as_str()) {
            Some(data) => data.get(1).unwrap().as_str().to_string(),
            None => assule_role.clone(),
        };

        let role_req = AssumeRoleReq {
            arn: Some(assule_role),
            session_name,
        };

        return sts_assume_role(config, role_req, mfa).await;
    }

    // 通常のセッショントークンを取得
    sts_session_token(config, mfa).await
}

/// MFA情報を取得
///
fn get_mfa_info(mfa_serial: Option<String>) -> MFAInfo {
    // 指定なしの場合はNoneで生成
    if mfa_serial.is_none() {
        return MFAInfo {
            arn: None,
            code: None,
        };
    }

    // Get the token code at the prompt for entering standard text
    let msg = format!(
        "Enter AWS MFA code for device [{}]",
        mfa_serial.clone().unwrap()
    );
    let input: Option<String> = match Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(msg)
        .interact_text()
    {
        Ok(val) => Some(val),
        Err(_) => None,
    };

    MFAInfo {
        arn: mfa_serial,
        code: input,
    }
}

/// STSクライアント生成
///
async fn __sts_client(config: &profile::configs::Config) -> aws_sdk_sts::Client {
    let config_builder = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(config.region.clone()));

    let aws_config = config_builder.load().await;

    aws_sdk_sts::Client::new(&aws_config)
}

/// セッショントークンを取得
///
pub async fn sts_session_token(
    config: &profile::configs::Config,
    mfa: MFAInfo,
) -> Result<aws_sdk_sts::types::Credentials, aws_sdk_sts::Error> {
    // クライアント
    let client = __sts_client(config).await;

    // リクエスト
    let result = client
        .get_session_token()
        .set_serial_number(mfa.arn)
        .set_token_code(mfa.code)
        .set_duration_seconds(Some(43200))
        .send()
        .await?;

    // Credentialを返却
    Ok(result.credentials.expect("should include credentials"))
}

/// Assume Roleを行う
///
pub async fn sts_assume_role(
    config: &profile::configs::Config,
    role: AssumeRoleReq,
    mfa: MFAInfo,
) -> Result<aws_sdk_sts::types::Credentials, aws_sdk_sts::Error> {
    // クライアント
    let client = __sts_client(config).await;

    // リクエスト
    let result = client
        .assume_role()
        .role_session_name(role.session_name)
        .set_role_arn(role.arn)
        .set_serial_number(mfa.arn)
        .set_token_code(mfa.code)
        .set_duration_seconds(Some(3600))
        .send()
        .await?;

    // Credentialを返却
    Ok(result.credentials.expect("should include credentials"))
}

/// Caller Identityを取得
pub async fn caller_identity(
    config: &profile::configs::Config,
) -> Result<String, aws_sdk_sts::Error> {
    // クライアント
    let client = __sts_client(config).await;
    // リクエスト
    let output = client.get_caller_identity().send().await?;
    Ok(output.account.expect("should include credentials"))
}
