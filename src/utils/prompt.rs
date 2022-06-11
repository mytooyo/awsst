use dialoguer::{
    console::{style, Style, Term},
    theme::ColorfulTheme,
    Confirm, Input, Select,
};
use std::{
    io::{Error, ErrorKind, Write},
    str,
};

pub struct Prompter {
    pub term: Term,
    height: usize,
}

impl Prompter {
    pub fn new() -> Prompter {
        let _term = Term::stderr();
        Prompter {
            term: _term,
            height: 0,
        }
    }

    pub fn flush(&self) {
        let _term = self.term.clone();
        // Ctrl+Cを検知してターミナルのカーソルを戻す
        let _ = ctrlc::set_handler(move || {
            let _ = _term.show_cursor();
            let _ = _term.flush();
        });
    }

    /// 選択用のプロンプトを設定
    pub fn select_prompt(&self, selections: &[String], msg: &str) -> Option<usize> {
        let result = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(msg)
            .default(0)
            .items(selections)
            .interact_on_opt(&self.term);
        if result.is_err() {
            return None;
        }
        result.unwrap()
    }

    /// テキスト入力用のプロンプトを設定
    pub fn input_prompt(
        &self,
        msg: &str,
        required: bool,
        default_str: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // デフォルト値の設定がある場合
        let text = if let Some(def_val) = default_str {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt(msg)
                .allow_empty(!required)
                .with_initial_text(def_val)
                .interact_text()?
        } else {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt(msg)
                .allow_empty(!required)
                .interact_text()?
        };
        Ok(text)
    }

    /// 確認用のプロンプトを設定
    pub fn confirm_prompt(&self, msg: &str) -> Result<bool, Box<dyn std::error::Error>> {
        // 確認フォーム表示
        let result = match Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(msg)
            .wait_for_newline(true)
            .interact_opt()?
        {
            Some(true) => true,
            Some(false) => false,
            None => false,
        };
        Ok(result)
    }

    /// ターミナルにライン出力
    pub fn write_formatted_line<
        F: FnOnce(&mut Prompter, &mut dyn std::fmt::Write) -> std::fmt::Result,
    >(
        &mut self,
        f: F,
    ) -> std::io::Result<()> {
        let mut buf = String::new();
        f(self, &mut buf).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        self.height += buf.chars().filter(|&x| x == '\n').count() + 1;
        self.term.write_line(&buf)
    }

    /// 通常の文字列を表示
    pub fn standard(&mut self, msg: &str) {
        let _ = self.write_formatted_line(|_, buf| write!(buf, "{}", msg));
    }

    /// キーバリュー形式で表示
    pub fn keyvalue(&mut self, key: &str, value: &str) {
        let _ = self.write_formatted_line(|_, buf| {
            let style = Style::new().for_stderr().blue();
            write!(buf, "{}: {}", key, style.apply_to(value))
        });
    }

    /// エラーの文字列を表示
    pub fn error(&mut self, msg: &str) {
        let _ = self.write_formatted_line(|_, buf| {
            let prefix = style("✘".to_string()).for_stderr().red();
            let style = Style::new().for_stderr().red();
            write!(buf, "{} {}", &prefix, style.apply_to(msg))
        });
    }
}

pub struct StringWriter {
    string: String,
}

impl StringWriter {
    pub fn new() -> StringWriter {
        StringWriter {
            string: String::new(),
        }
    }

    /// 全データを文字列
    pub fn as_string(&self) -> &str {
        &self.string
    }

    /// 行データをリスト形式に
    pub fn as_vec(&self) -> Vec<&str> {
        let c: char = '\n';
        self.as_string().split(c).collect::<Vec<&str>>()
    }
}

impl Write for StringWriter {
    fn write(&mut self, data: &[u8]) -> Result<usize, Error> {
        let string = match str::from_utf8(data) {
            Ok(s) => s,
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("Cannot decode utf8 string : {}", e),
                ))
            }
        };
        self.string.push_str(string);
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<(), Error> {
        // Nothing to do here
        Ok(())
    }
}
