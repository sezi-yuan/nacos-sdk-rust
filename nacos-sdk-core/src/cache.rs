use std::{path::Path, collections::HashMap};

use serde::{Serialize, de::DeserializeOwned};
use tokio::fs;

use crate::error::*;


pub async fn write_file<T: Serialize, D: AsRef<Path>>(content: &T, dir: D, file_name: &str) -> Result<()> {
    let contents = serde_json::to_string(content)?;
    write_file_str(contents.as_str(), dir, file_name).await
}

pub async fn write_file_str<D: AsRef<Path>>(contents: &str, dir: D, file_name: &str) -> Result<()> {
    let dir = dir.as_ref();
    if !dir.exists() {
        fs::create_dir_all(dir).await
            .map_err(|err| Error::Fs("failed to create cache dir".to_owned(), err))?;
    }
    let path = dir.join(file_name);
    fs::write(path, contents).await
        .map_err(|err| Error::Fs("can not write cache file".to_owned(), err))?;
    Ok(())
}

pub async fn read_dir<T: DeserializeOwned>(dir: impl AsRef<Path>) -> Result<HashMap<String, T>> {
    let content = read_dir_str(dir).await?;
    let mut ret = HashMap::new();
    for (key, value) in content.into_iter() {
        let _ = match serde_json::from_str::<T>(value.as_str()) {
            Ok(x) => ret.insert(key, x),
            Err(err) => {
                log::error!("failed to parse cache file[{}]: {}", key, err);
                return Err(Error::Serde(err));
            }
        };
    }
    Ok(ret)
}

pub async fn read_dir_str<D: AsRef<Path>>(dir: D) -> Result<HashMap<String, String>> {
    let mut dir = fs::read_dir(dir.as_ref()).await
        .map_err(|err|Error::Fs("failed to read cache dir".to_owned(), err))?;
    
    let mut ret_val = HashMap::new();
    while let Some(entry) = dir.next_entry().await.expect("") {
        let file_path = entry.path();
        let content = read_file_str(file_path).await?;
        let file_name = entry.file_name().into_string().expect("illegal charactor in filename");
        ret_val.insert(file_name, content);
    }

    Ok(ret_val)
}

pub async fn read_file<T: DeserializeOwned>(file: impl AsRef<Path>) -> Result<T> {
    let content = read_file_str(file).await?;
    let ret = serde_json::from_str(content.as_str())?;
    Ok(ret)
}

pub async fn read_file_str<D: AsRef<Path>>(file: D) -> Result<String> {
    let file = file.as_ref();
    fs::read_to_string(file).await
        .map_err(|err| Error::Fs(format!("failed to read file: {:?}", file), err))
}