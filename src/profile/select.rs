use crate::utils::{AWSFile, AWSFileManager};
use std::collections::HashMap;

/// AWS Configファイル情報
pub struct AWSSelecteds {
    pub items: HashMap<String, Selected>,
}

/// 存在しない場合に作成する
pub fn new_selected(name: String, region: String) -> AWSSelecteds {
    let mut items = HashMap::<String, Selected>::new();
    items.insert("selected".to_string(), Selected { name, region });
    AWSSelecteds { items }
}

impl AWSFileManager<Selected> for AWSSelecteds {
    fn new(val: HashMap<String, HashMap<String, String>>) -> AWSSelecteds {
        let mut items = HashMap::<String, Selected>::new();
        for (key, ele) in val {
            items.insert(
                key,
                Selected {
                    name: ele["name"].clone(),
                    region: ele["region"].clone(),
                },
            );
        }
        AWSSelecteds { items }
    }

    /// ファイル出力用にMapを生成
    fn to_file(&self) -> HashMap<String, HashMap<String, String>> {
        let mut list = HashMap::<String, HashMap<String, String>>::new();
        for ele in &self.items {
            list.insert(ele.0.clone(), ele.1.to_file_map());
        }
        list
    }

    /// ツール用ファイル書き込み
    fn write(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 書き込み用に整形
        let data = self.to_file();
        // ファイル書き込み
        super::utils::file::write(super::TOOL_FILE_NAME, data)
    }

    fn add(&mut self, data: Selected) {
        self.items.insert(data.name.clone(), data);
    }

    fn remove(&mut self, name: String) {
        let _ = self.items.remove(&name);
    }
}

/// 選択中のプロファイル情報のアイテム構造体
#[derive(Debug, Clone)]
pub struct Selected {
    pub name: String,
    pub region: String,
}

impl AWSFile for Selected {
    fn to_file_map(&self) -> HashMap<String, String> {
        let mut list = HashMap::<String, String>::new();
        list.insert("name".to_string(), self.name.clone());
        list.insert("region".to_string(), self.region.clone());
        list
    }
}
