use chrono::{DateTime, Local, TimeZone};
use std::{collections::HashMap, env, fmt::Display, time::SystemTime};

use super::utils::{AWSFile, AWSFileManager};
use crate::sts_client::{aws_sts_request, caller_identity};

pub const KEY_SUFFIX: &str = "awsst";

/// AWS Credentialファイル情報
#[derive(Debug)]
pub struct AWSCredentials {
    // 通常のAWS CLIで利用されるサフィックスついていない情報
    pub bases: Vec<Credential>,
    // アクセスキーを保存するアプリで利用するサッフィクスつきの情報
    pub originals: Vec<Credential>,
}

impl AWSCredentials {
    // キーとする名称からCredential情報を取得
    fn __credential_by_key(&self, vec: &[Credential], key: &str) -> Option<Credential> {
        // filter等を利用するとライフタイム的に問題があるため、
        // forで探索を行う
        for ele in vec {
            if ele.name == *key {
                return Some(ele.clone());
            }
        }
        None
    }

    /// 指定のキー名のCredentialが存在するか確認
    pub fn exists_credential(&self, key: String) -> bool {
        // baseに存在するか確認
        // originalのみ存在する状況は許容しないため、ここではチェックを行わない
        self.__credential_by_key(&self.bases, &key).is_some()
    }

    /// CLIで利用される情報
    pub fn use_credential(&self, key: String) -> Credential {
        // originalのCredentialが存在する場合はそれを利用
        self.__credential_by_key(&self.bases, &key).unwrap()
    }

    /// 本アプリ用の認証情報を保存するキーを取得
    pub fn auth_credential(&mut self, key: String, force: bool) -> Option<Credential> {
        // サフィックスがついていない情報を取得し、期限の確認を行う.
        let cred = self.use_credential(key.clone());
        // 強制更新ではない場合は期限確認を行う
        if !force {
            // 期限内であった場合はそのまま返却
            if !cred.is_expired() {
                return None;
            }
        }

        // 期限切れの場合はoriginalの情報を取得
        let origin_key = format!("{}-{}", key, KEY_SUFFIX);
        let origin = self.__credential_by_key(&self.originals, &origin_key);
        if let Some(_origin) = origin {
            return Some(_origin);
        }
        // // 存在しない場合は新たに生成してMapに追加しておく
        let mut new_cred = cred;
        new_cred.name = format!("{}-{}", key, KEY_SUFFIX);
        self.originals.push(new_cred);

        // // 新たに生成した情報を返却
        self.__credential_by_key(&self.originals, &origin_key)
    }

    /// サフィックスのついた本アプリ用に保存しているCredential情報を優先的に取得
    pub fn suffix_credential(&mut self, key: String) -> Option<&mut Credential> {
        let origin_key = format!("{}-{}", key, KEY_SUFFIX);
        for ele in self.originals.iter_mut() {
            if ele.name == origin_key {
                return Some(ele);
            }
        }

        // オリジナルに存在しない場合はベースから取得
        for ele in self.bases.iter_mut() {
            if ele.name == key {
                return Some(ele);
            }
        }

        None
    }

    /// AWSのクレデンシャル情報を設定
    pub async fn set_credential(
        &mut self,
        config: &super::configs::Config,
        key: String,
        cred: aws_sdk_sts::model::Credentials,
    ) -> Option<Credential> {
        let no_suf_key = key.clone().replace(format!("-{}", KEY_SUFFIX).as_str(), "");

        // サフィックスのデータが存在するかのチェックを行う
        let suf_key = format!("{}-{}", no_suf_key, KEY_SUFFIX);
        let mut is_exists = false;
        for ele in &self.originals {
            if ele.name == suf_key {
                is_exists = true;
                break;
            }
        }

        // キーにサフィックスがついている場合はサフィックスなしのキーを利用する
        let target_key = if key.contains(KEY_SUFFIX) {
            no_suf_key
        } else {
            key
        };
        // AWSCLIで利用するサフィックスがついてないデータを更新する
        for ele in self.bases.iter_mut() {
            // キーが一致する場合に処理を行う
            if ele.name == target_key {
                // originalに存在しない場合は生成する
                if !is_exists {
                    let mut cloned = ele.clone();
                    cloned.name = suf_key;
                    self.originals.push(cloned);
                }
                ele.update_credential(config, cred).await;
                return Some(ele.clone());
            }
        }
        None
    }

    /// HashMapから設定
    fn set_from_map(key: String, ele: &HashMap<String, String>) -> Credential {
        let role = ele.contains_key("assumed_role") && ele.get("assumed_role").unwrap() == "true";

        Credential {
            name: key,
            access_key_id: Self::get_value_from_map(ele, "aws_access_key_id"),
            secret_access_key: Self::get_value_from_map(ele, "aws_secret_access_key"),
            session_token: Self::get_value_from_map(ele, "aws_session_token"),
            security_token: Self::get_value_from_map(ele, "aws_security_token"),
            expiration: Self::get_value_from_map(ele, "expiration"),
            mfa_serial: Self::get_value_from_map(ele, "mfa_serial"),
            role_arn: Self::get_value_from_map(ele, "role_arn"),
            account: Self::get_value_from_map(ele, "account"),
            source_profile: Self::get_value_from_map(ele, "source_profile"),
            assumed_role: role,
        }
    }

    /// HashMapからOption型で取り出す
    fn get_value_from_map(map: &HashMap<String, String>, key: &str) -> Option<String> {
        map.get(key).cloned()
    }
}

/// AWSFileの実装を行い, AWSFileとして扱えるようにする
impl AWSFileManager<Credential> for AWSCredentials {
    /// AWSCredentialsを生成
    fn new(val: HashMap<String, HashMap<String, String>>) -> AWSCredentials {
        let mut bases = Vec::<Credential>::new();
        let mut originals = Vec::<Credential>::new();
        for (key, ele) in val {
            if key.contains(KEY_SUFFIX) {
                originals.push(Self::set_from_map(key, &ele));
            } else {
                bases.push(Self::set_from_map(key, &ele));
            }
        }

        AWSCredentials { bases, originals }
    }

    /// ファイル出力用にMapを生成
    fn to_file(&self) -> HashMap<String, HashMap<String, String>> {
        let mut list = HashMap::<String, HashMap<String, String>>::new();

        // original分のデータを生成
        for ele in &self.originals {
            list.insert(ele.name.clone(), ele.to_file_map());
        }
        // base分のデータを生成
        for ele in &self.bases {
            list.insert(ele.name.clone(), ele.to_file_map());
        }
        list
    }

    /// `credentials`ファイル書き込み
    fn write(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 書き込み用に整形
        let data = self.to_file();
        // ファイル書き込み
        super::utils::file::write(super::CREDENTIAL_FILE_NAME, data, false)
    }

    fn add(&mut self, data: Credential) {
        self.bases.push(data);
    }

    fn remove(&mut self, name: String) {
        for (i, ele) in self.bases.iter_mut().enumerate() {
            if ele.name == name {
                self.bases.remove(i);
                break;
            }
        }
        // サフィックス付きのデータも合わせて削除する
        let suf_key = format!("{}-{}", name, KEY_SUFFIX);
        for (i, ele) in self.originals.iter_mut().enumerate() {
            if ele.name == suf_key {
                self.bases.remove(i);
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Credential {
    pub name: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub security_token: Option<String>,
    pub expiration: Option<String>,
    pub mfa_serial: Option<String>,
    pub role_arn: Option<String>,
    pub account: Option<String>,
    pub source_profile: Option<String>,
    pub assumed_role: bool,
}

impl Default for Credential {
    fn default() -> Self {
        Credential {
            name: String::from(""),
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
            security_token: None,
            expiration: None,
            mfa_serial: None,
            role_arn: None,
            account: None,
            source_profile: None,
            assumed_role: false,
        }
    }
}

impl Credential {
    pub fn from_configure(
        profile_name: String,
        access_key_id: String,
        secret_access_key: String,
        mfa: Option<String>,
    ) -> Self {
        Credential {
            name: profile_name,
            access_key_id: Some(access_key_id),
            secret_access_key: Some(secret_access_key),
            session_token: None,
            security_token: None,
            expiration: None,
            mfa_serial: mfa,
            role_arn: None,
            account: None,
            source_profile: None,
            assumed_role: false,
        }
    }

    /// 期限切れかチェック
    /// 期限が切れている場合は`true`, 期限内の場合は`false`
    pub fn is_expired(&self) -> bool {
        // 期限が設定されている場合
        if let Some(expiration) = &self.expiration {
            let date = Local
                .datetime_from_str(expiration.as_str(), "%Y-%m-%d %H:%M:%S")
                .unwrap();
            let now = Local::now();
            // 比較して、期限が切れていない場合は`false`
            // if date.cmp(&now) == Ordering::Greater {
            //     return false;
            // }
            let duration = date - now;
            return duration.num_hours() < 3;
        }
        true
    }

    /// 環境変数に利用するAWSプロファイル情報を設定する
    /// 環境変数に設定することで, 本ツール内でAWSリクエストを行う際に利用できるようにする
    pub fn set_environment(&self, config: &super::configs::Config) {
        // リージョンを設定
        env::set_var("AWS_DEFAULT_REGION", &config.region);
        // アクセスキーを設定
        if let Some(access_key) = &self.access_key_id {
            env::set_var("AWS_ACCESS_KEY_ID", access_key);
        }
        // シークレットアクセスキーを設定
        if let Some(secret_access_key) = &self.secret_access_key {
            env::set_var("AWS_SECRET_ACCESS_KEY", secret_access_key);
        }
    }

    /// AWS STSで認証情報を取得
    pub async fn sts_credential(
        &mut self,
        config: &super::configs::Config,
    ) -> Result<aws_sdk_sts::model::Credentials, aws_sdk_sts::Error> {
        // 環境情報を設定
        self.set_environment(config);

        // 認証情報取得リクエスト
        aws_sts_request(config, self.clone()).await
    }

    /// クレデンシャル情報更新
    pub async fn update_credential(
        &mut self,
        config: &super::configs::Config,
        aws_cred: aws_sdk_sts::model::Credentials,
    ) {
        self.access_key_id = aws_cred.access_key_id;
        self.secret_access_key = aws_cred.secret_access_key;
        self.session_token = aws_cred.session_token.clone();
        self.security_token = aws_cred.session_token;

        if let Some(system_time) = aws_cred.expiration {
            match SystemTime::try_from(system_time) {
                Ok(dtime) => {
                    // When writing the deadline, do it in local time
                    let datetime: DateTime<Local> = dtime.into();
                    self.expiration = Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string());
                }
                Err(_) => return,
            }
        }

        // アカウント情報を取得して設定しておく
        let result = caller_identity(config).await;
        if let Ok(account) = result {
            self.account = Some(account);
        }
    }

    /// リストに追加するかの判定を行う
    fn __to_file_list_push(
        &self,
        list: &mut HashMap<String, String>,
        key: &str,
        data: &Option<String>,
    ) {
        if let Some(val) = data {
            list.insert(key.to_string(), val.clone());
        }
    }
}

impl AWSFile for Credential {
    /// ファイルに書き込むための形式に変換
    fn to_file_map(&self) -> HashMap<String, String> {
        let mut list = HashMap::<String, String>::new();
        self.__to_file_list_push(&mut list, "aws_access_key_id", &self.access_key_id);
        self.__to_file_list_push(&mut list, "aws_secret_access_key", &self.secret_access_key);
        self.__to_file_list_push(&mut list, "aws_session_token", &self.session_token);
        self.__to_file_list_push(&mut list, "aws_security_token", &self.security_token);
        self.__to_file_list_push(&mut list, "expiration", &self.expiration);
        self.__to_file_list_push(&mut list, "mfa_serial", &self.mfa_serial);
        self.__to_file_list_push(&mut list, "role_arn", &self.role_arn);
        self.__to_file_list_push(&mut list, "account", &self.account);
        self.__to_file_list_push(&mut list, "source_profile", &self.source_profile);
        if self.assumed_role {
            list.insert("assumed_role".to_string(), self.assumed_role.to_string());
        }
        list
    }
}

impl Display for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, access_key_id: {:?}, secret_access_key: {:?}",
            self.name, self.access_key_id, self.secret_access_key,
        )
    }
}
