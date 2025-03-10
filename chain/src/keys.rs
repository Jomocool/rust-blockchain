use crate::error::{ChainError, Result};
use ethereum_types::Address;
use lazy_static::lazy_static;
use std::fs::{create_dir, read, write};
use utils::{
    crypto::{keypair, public_key_address},
    PublicKey, SecretKey,
};

// 定义密钥路径常量
const PATH: &str = "./../.keys";
const PRIVATE_KEY_PATH: &str = "./../.keys/private.key";
const PUBLIC_KEY_PATH: &str = "./../.keys/public.key";

// 使用lazy_static宏来初始化静态变量
lazy_static! {
    // 初始化私钥
    pub(crate) static ref PRIVATE_KEY: SecretKey =
        get_private_key().expect("Could not retrieve the private key");
    // 初始化公钥
    pub(crate) static ref PUBLIC_KEY: PublicKey =
        get_public_key().expect("Could not retrieve the public key");
    // 根据公钥初始化地址
    pub(crate) static ref ADDRESS: Address = public_key_address(&PUBLIC_KEY);
}

/// 添加密钥对到指定路径
///
/// 该函数首先尝试创建密钥目录，如果目录已存在或创建失败，将记录错误信息。
/// 如果目录创建成功，将生成新的密钥对，并将其分别保存到私钥路径和公钥路径。
///
/// # Returns
///
/// 返回一个结果，表示操作是否成功。
pub(crate) fn add_keys() -> Result<()> {
    // 尝试创建密钥目录，如果失败则记录错误信息
    if let Err(e) = create_dir(PATH) {
        tracing::info!("Did not create key directory '{}' {}", PATH, e.to_string());
    } else {
        // 生成新的密钥对
        let (private_key, public_key) = keypair();

        // 将私钥和公钥分别写入文件
        write(PRIVATE_KEY_PATH, private_key.as_ref()).unwrap();
        write(PUBLIC_KEY_PATH, public_key.serialize()).unwrap();
    }

    Ok(())
}

/// 读取私钥
///
/// 从私钥路径读取私钥数据，并尝试将其解析为SecretKey对象。
///
/// # Returns
///
/// 返回一个结果，包含解析后的SecretKey对象，如果操作成功。
pub(crate) fn get_private_key() -> Result<SecretKey> {
    // 读取私钥数据
    let key = read(PRIVATE_KEY_PATH).expect("Could not read private key");
    // 将数据解析为SecretKey对象，如果解析失败，返回错误
    SecretKey::from_slice(&key).map_err(|e| ChainError::InternalError(e.to_string()))
}

/// 读取公钥
///
/// 从公钥路径读取公钥数据，并尝试将其解析为PublicKey对象。
///
/// # Returns
///
/// 返回一个结果，包含解析后的PublicKey对象，如果操作成功。
pub(crate) fn get_public_key() -> Result<PublicKey> {
    // 读取公钥数据
    let key = read(PUBLIC_KEY_PATH).expect("Could not read public key");
    // 将数据解析为PublicKey对象，如果解析失败，返回错误
    PublicKey::from_slice(&key).map_err(|e| ChainError::InternalError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_save_keys() {
        add_keys().unwrap();
    }

    #[test]
    fn it_retrieves_the_saved_private_key() {
        add_keys().unwrap();
        let key = get_private_key().unwrap();
        println!("{:?}", key);
    }

    #[test]
    fn it_retrieves_the_saved_public_key() {
        add_keys().unwrap();
        let key = get_public_key().unwrap();
        println!("{:?}", key);
    }
}
