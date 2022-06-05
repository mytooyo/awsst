use std::{collections::BTreeMap, error::Error};

use prettytable::{cell, format, row, Table};

use super::{configs::Config, credentials::Credential};

pub struct AWSConfigure {
    pub profile: String,
    pub region: String,
    pub output: String,
    pub access_key: String,
    pub secret_access_key: String,
    pub mfa_role: Option<String>,
}

impl Default for AWSConfigure {
    fn default() -> Self {
        AWSConfigure {
            profile: String::from(""),
            region: String::from(""),
            output: String::from(""),
            access_key: String::from(""),
            secret_access_key: String::from(""),
            mfa_role: None,
        }
    }
}

impl AWSConfigure {
    pub fn from_conf(config: Config, credential: Option<Credential>) -> Self {
        let _credential = credential.unwrap_or_default();
        AWSConfigure {
            profile: config.name,
            region: config.region,
            output: config.output,
            access_key: if let Some(ak) = _credential.access_key_id {
                ak
            } else {
                String::from("")
            },
            secret_access_key: if let Some(sak) = _credential.secret_access_key {
                sak
            } else {
                String::from("")
            },
            mfa_role: _credential.mfa_serial,
        }
    }

    /// `configure`実行時に表示するダイアログを生成、設定を行う
    pub fn dialog_for_user(
        &mut self,
        prompter: &mut super::utils::prompt::Prompter,
    ) -> Result<(), Box<dyn Error>> {
        // 更新の場合はプロファイル名の更新はさせない
        let profile = if self.profile.is_empty() {
            prompter.input_prompt("Profile Name", true, None)?
        } else {
            self.profile.clone()
        };
        let region = prompter.input_prompt(
            "Region",
            true,
            if self.region.is_empty() {
                Some("ap-northeast-1".to_string())
            } else {
                Some(self.region.clone())
            },
        )?;
        let output = prompter.input_prompt(
            "Output",
            true,
            if self.output.is_empty() {
                Some("json".to_string())
            } else {
                Some(self.output.clone())
            },
        )?;
        let access_key = prompter.input_prompt(
            "Access Key ID",
            true,
            if self.access_key.is_empty() {
                None
            } else {
                Some(self.access_key.clone())
            },
        )?;
        let secret_access_key = prompter.input_prompt(
            "Secret Access Key",
            true,
            if self.secret_access_key.is_empty() {
                None
            } else {
                Some(self.secret_access_key.clone())
            },
        )?;
        let mfa_role =
            prompter.input_prompt("MFA Device ARN (Optional)", false, self.mfa_role.clone())?;

        // 入力された内容の確認フォームを表示させる
        let mut map: BTreeMap<String, &String> = BTreeMap::new();
        map.insert("1. Profile Name".to_string(), &profile);
        map.insert("2. Region".to_string(), &region);
        map.insert("3. Output".to_string(), &output);
        map.insert("4. Access Key ID".to_string(), &access_key);
        map.insert("5. Secret Access Key".to_string(), &secret_access_key);
        map.insert("6. MFA Device ARN".to_string(), &mfa_role);

        // 確認するためのテーブルに表示
        let mut table = Table::new();
        table.set_titles(row![cell!("KEY"), cell!("VALUE")]);
        table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        for (k, v) in map {
            // Add a row to the table
            table.add_row(row![cell!(k), cell!(v)]);
        }

        let mut writer = super::utils::prompt::StringWriter::new();
        table.print(&mut writer)?;
        let data = writer
            .as_vec()
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        let out = data[..(data.len() - 1)].to_vec();
        prompter.standard(out.join("\n").as_str());

        // 確認フォーム表示
        let result = prompter.confirm_prompt("Is it okay to add with the displayed contents?")?;

        // 登録する場合の処理
        if !result {
            return Ok(());
        }

        // 各情報を設定
        self.profile = profile;
        self.region = region;
        self.output = output;
        self.access_key = access_key;
        self.secret_access_key = secret_access_key;
        if !mfa_role.is_empty() {
            self.mfa_role = Some(mfa_role);
        } else {
            self.mfa_role = None;
        }

        Ok(())
    }
}
