use std::{collections::HashMap, fmt::Display};

use crate::utils;

use super::utils::{AWSFile, AWSFileManager};

/// AWS Configファイル情報
pub struct AWSConfigs {
    pub items: HashMap<String, Config>,
}

/// AWSConfigsの実装
impl AWSConfigs {
    /// 選択用のリストを生成
    pub fn shape_for_selectable(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<String>>()
    }

    /// 指定の名称のconfigが存在するか確認
    pub fn exists_config(&self, name: String) -> bool {
        self.items.get(&name).is_some()
    }

    /// Configのリストから選択または、指定の名称のConfigが存在するかのチェックを行う
    pub fn selection_config_name(
        &self,
        profile: Option<String>,
        prompter: &mut utils::prompt::Prompter,
    ) -> Option<String> {
        let selections = &self.shape_for_selectable();

        // 0件の場合はメッセージを表示して終了
        if selections.is_empty() {
            prompter.error("No profile is registered.\nPlease register with the `awsst configure` command before use.");
            return None;
        }

        // Configが1件のみの場合はそのまま返却する
        if selections.len() == 1 {
            return Some(selections[0].clone());
        }

        // プロファイルが指定されている場合
        let name: String = if let Some(_profile) = profile {
            // 指定のプロファイルが存在するか確認
            if !self.exists_config(_profile.clone()) {
                // 存在しない場合はエラー
                prompter.error("Oops.. profile does not exists profile...");
                return None;
            }
            _profile
        }
        // 対象が指定されていない場合は選択プロンプトを表示して対象を選択してもらう
        else {
            // コンソールに選択プロンプトを表示
            let opt_selection =
                prompter.select_prompt(selections, "Please select the profile you want to use")?;

            // 選択されたインデックスからConfigの名前を取得
            selections[opt_selection].clone()
        };
        Some(name)
    }
}

/// AWSFileの実装を行い, AWSFileとして扱えるようにする
impl AWSFileManager<Config> for AWSConfigs {
    /// AWSConfigを生成
    fn new(val: HashMap<String, HashMap<String, String>>) -> AWSConfigs {
        let mut items = HashMap::<String, Config>::new();
        for (key, ele) in val {
            items.insert(
                key.clone().replace("profile ", ""),
                Config {
                    name: key,
                    region: ele["region"].clone(),
                    output: ele["output"].clone(),
                },
            );
        }

        AWSConfigs { items }
    }

    /// ファイル出力用にMapを生成
    fn to_file(&self) -> HashMap<String, HashMap<String, String>> {
        let mut list = HashMap::<String, HashMap<String, String>>::new();
        for ele in &self.items {
            list.insert(ele.0.clone(), ele.1.to_file_map());
        }
        list
    }

    /// `config`ファイル書き込み
    fn write(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 書き込み用に整形
        let data = self.to_file();
        // ファイル書き込み
        super::utils::file::write(super::CONFIG_FILE_NAME, data, true)
    }

    fn add(&mut self, data: Config) {
        self.items.insert(data.name.clone(), data);
    }

    fn remove(&mut self, name: String) {
        let _ = self.items.remove(&name);
    }
}

/// AWS　Configファイル情報のアイテムの構造体
#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub region: String,
    pub output: String,
}

impl AWSFile for Config {
    /// ファイルに書き込むための形式に変換
    fn to_file_map(&self) -> HashMap<String, String> {
        let mut list = HashMap::<String, String>::new();
        list.insert("region".to_string(), self.region.clone());
        list.insert("output".to_string(), self.output.clone());
        list
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, region: {}, output: {}",
            self.name, self.region, self.output
        )
    }
}
