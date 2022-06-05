
# AWSプロファイル管理ツール

`aws configure`で管理するプロファイルをより簡単に扱えるようにするツールです.  
プロファイルの登録、更新、削除の他にSTSセッショントークンを取得できます.   
セッショントークンを取得することで指定のプロファイルで認証状態を保持し、  
`aws`コマンドを実行することができます.  
また、MFAデバイスによる2ファクタ認証も可能であるため、MFA必須のIAMユーザでも`aws`コマンドを実行できます.  

<br />

本ツールは環境変数の`AWS_PROFILE`に`export`する仕様となっているため、下記の手順に従い設定が必要になります.  
`export`することで`aws`コマンド実行時に`-p`を指定する必要がなくなります.  


## 使い方
```
awsst 0.1.0

USAGE:
    awsst [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -f, --force                Forces the session token to be updated
    -h, --help                 Print help information
    -p, --profile <PROFILE>    Name of the profile from which the session token is to be obtained
    -V, --version              Print version information

SUBCOMMANDS:
    configure    Same process as `aws configure`
    help         Print this message or the help of the given subcommand(s)
    ls           List profile from credential
    remove       Remove profile from config
    session      Get session token
    update       Update porfile information
    use          Select the profile you want to use
```

## インストール
本アプリケーションをビルドし、実行可能ファイルにするための手順について記述する.  
(本リポジトリがダウンロード済とする)  

```shell
// アプリケーションをビルド
$ cargo build --release

// ビルドしたアプリを移動
$ mkdir -p $HOME/tools
$ cp `pwd`/target/release/awsst $HOME/tools/
```

`cd`コマンドと合わせて実行する必要があるため、`.zshrc`または`.bashrc`に下記の通り設定する.  

```
awsst() {
    eval `$HOME/tools/awsst $@`
}
```

## 使い方の例

下記にサブコマンドの使い方の例を示します.  
`configure`を除くサブコマンドは`-p profile`で選択プロンプトを表示せずに直接指定のプロファイルに対して処理を行えます.  
また、登録が1件のみ場合は選択プロンプトは表示されません.

1. プロファイル登録

```shell
$ awsst configure
? Profile Name › profile
? Region › ap-northeast-1
? Output › json
? Access Key ID › xxxxxxxxxxxxxxxxxxx
? Secret Access Key › xxxxxxxxxxxxxxxxxxxxxx
? MFA Device ARN (Optional) › 
+----------------------+------------------------+
| KEY                  | VALUE                  |
+----------------------+------------------------+
| 1. Profile Name      | profile                |
| 2. Region            | ap-northeast-1         |
| 3. Output            | json                   |
| 4. Access Key ID     | xxxxxxxxxxxxxxxxxxx    |
| 5. Secret Access Key | xxxxxxxxxxxxxxxxxxxxxx |
| 6. MFA Device ARN    |                        |
+----------------------+------------------------+
? Is it okay to add with the displayed contents? (y/n) › y
```

2. セッショントークン取得
```shell
$ awsst
? Please select the profile you want to use › profile
Success! Token expiration is : 2022-04-01 00:00:00
```

`awsst session`でサブコマンドを指定しても同一の処理を行えます.  


3. プロファイル情報更新
```shell
$ awsst update
```

表示内容は`configure`サブコマンドと同一となります.

4. プロファイル削除
```shell
$ awsst remove
? Please select the profile you want to use › profile
```

5. 利用プロファイル選択
```shell
$ awsst use
? Please select the profile you want to use › profile
```

## License
MIT License
