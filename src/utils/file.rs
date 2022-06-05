use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, BufWriter, Error, ErrorKind, Write},
};

use super::AWSFileManager;

/// ファイル読み込み
pub fn read<T, S>(file_name: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T: AWSFileManager<S>,
{
    // ホームディレクトリを取得
    let opt_home = dirs::home_dir();
    // ホームディレクトリが存在しない場合は作成するかの確認メッセージを表示
    if opt_home.is_none() {
        return Err(Box::new(Error::new(
            ErrorKind::Other,
            "Oops... does not home directory..orz",
        )));
    }
    // awsディレクトリを生成
    let mut aws_dir = opt_home.unwrap();
    aws_dir.push(".aws");

    // 存在確認を行い、存在しない場合は作成するかの確認メッセージを表示
    if !aws_dir.exists() {
        return Err(Box::new(Error::new(
            ErrorKind::Other,
            "Oops... does not [~/.aws] directory..orz",
        )));
    }

    // ファイルのパスを生成
    let mut config_path = aws_dir;
    config_path.push(file_name);
    // ファイルの存在確認し、存在しない場合は空で作成しておく
    if !config_path.exists() {
        let mut f = BufWriter::new(fs::File::create(config_path.clone())?);
        f.write("".as_bytes())?;
    }

    // ファイルを読み込み
    let file = fs::File::open(config_path)?;
    let reader = BufReader::new(file);
    // 読み込んだファイルをHashMap形式に整形
    let result = shape_aws_toml(reader)?;
    Ok(T::new(result))
}

/// AWS関連のファイルデータをMap形式に整形する
fn shape_aws_toml(
    reader: BufReader<fs::File>,
) -> std::io::Result<HashMap<String, HashMap<String, String>>> {
    // 読み込んだデータ用のマップ
    let mut tomls = HashMap::new();

    // プロファイルの名称を取得するための正規表現
    let re = regex::Regex::new(r"\[(.*)\]").unwrap();

    // 現在処理中のキー名と格納用マップ
    let mut key_name: Option<String> = None;

    // バッファから1行ずつ読み込んで処理する
    for line in reader.lines() {
        // ライン読み込み
        let l = line?;

        // トリムした結果、空行であった場合は無視
        if l.as_str().trim().is_empty() {
            continue;
        }

        // 名称の行と一致する場合
        if let Some(name) = re.captures(l.as_str()) {
            let key = name.get(1).unwrap().as_str().to_string();
            key_name = Some(key.clone());
            tomls.insert(key, "".to_string());
            continue;
        }

        // `=`で分割する
        let data = l.split("=").collect::<Vec<&str>>();
        // 分割できなかった場合は無視
        if data.len() < 2 {
            continue;
        }
        let key = key_name.clone().unwrap();
        if let Some(x) = tomls.get_mut(&key) {
            // 未設定状態の場合はそのまま設定
            *x = if x.is_empty() {
                data.join("=")
            } else {
                let v = vec![x.clone(), data.join("=")];
                v.join(";")
            }
        }
    }

    // 返却用のMapを生成
    let mut ret_map = HashMap::new();
    // 読み込んだデータのキー分実施
    for (key, val) in tomls.iter() {
        // キー内の要素を格納するMap
        let mut data_map = HashMap::new();
        // セミコロンで分割することでキー要素内の個々のデータを取得する
        val.split(";").for_each(|x| {
            // 個々のデータをKey, Value形式に変換してMapに追加
            let v = x
                .split("=")
                .map(|s| s.trim().to_string())
                .collect::<Vec<String>>();
            // Base64変換された認証情報は値に`=`が含まれるため、すべてをjoinするようにする
            let s = v[1..(v.len())].to_vec().join("=");
            data_map.insert(v[0].clone(), s.clone());
        });
        ret_map.insert(key.clone(), data_map);
    }

    Ok(ret_map)
}

/// ファイル書き込み
pub fn write(
    file_name: &str,
    data: HashMap<String, HashMap<String, String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 書き込み用のデータ
    let mut write_data = String::new();
    for (key, ele) in data {
        // キーを設定
        write_data += format!("[{}]\n", key).as_str();
        // 要素をそれぞれ追加していく
        for (ele_key, val) in ele {
            write_data += format!("{} = {}\n", ele_key, val).as_str();
        }
        write_data += "\n";
    }

    // バイトデータを生成
    let bytes = write_data.as_bytes();

    // ホームディレクトリを取得
    let opt_home = dirs::home_dir();
    // ホームディレクトリが存在しない場合は作成するかの確認メッセージを表示
    if opt_home.is_none() {
        return Err(Box::new(Error::new(ErrorKind::Other, "oh no!")));
    }
    // awsディレクトリを生成
    let mut aws_dir = opt_home.unwrap();
    aws_dir.push(".aws");
    // ファイルのパスを生成
    let mut fullpath = aws_dir;
    fullpath.push(file_name);

    let mut f = BufWriter::new(fs::File::create(fullpath)?);
    f.write(bytes)?;

    Ok(())
}
