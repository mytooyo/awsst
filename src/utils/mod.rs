use std::collections::HashMap;

pub mod file;
pub mod prompt;

/// AWSの`config`や`credential`ファイル用のトレイト
pub trait AWSFileManager<T> {
    fn new(val: HashMap<String, HashMap<String, String>>) -> Self;

    fn to_file(&self) -> HashMap<String, HashMap<String, String>>;

    fn write(&self) -> Result<(), Box<dyn std::error::Error>>;

    fn add(&mut self, data: T);

    fn remove(&mut self, name: String);
}

pub trait AWSFile {
    fn to_file_map(&self) -> HashMap<String, String>;
}
