use std::path::{Path, PathBuf};

use eth_trie::DB as EthDB;
use rocksdb::{Options, DB};

use crate::error::{ChainError, Result};

const PATH: &str = "./../.tmp";
const DATABASE_NAME: &str = "db";

// 定义一个调试友好的Storage结构体，用于与RocksDB数据库交互
#[derive(Debug)]
pub(crate) struct Storage {
    db: rocksdb::DB,
}

// 实现EthDB trait，用于以太坊数据库操作
impl EthDB for Storage {
    type Error = ChainError;

    /// 从数据库中获取与key关联的值
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let value = self
            .db
            .get(key)
            .map_err(|_| ChainError::StorageNotFound(Storage::key_string(key)))?;

        Ok(value)
    }

    /// 在数据库中插入键值对
    fn insert(&self, key: &[u8], value: Vec<u8>) -> Result<()> {
        self.db
            .put(key, value)
            .map_err(|_| ChainError::StoragePutError(Storage::key_string(key)))?;

        Ok(())
    }

    /// 从数据库中移除指定的键值对，此处未实现具体逻辑
    fn remove(&self, _key: &[u8]) -> Result<()> {
        Ok(())
    }

    /// 刷新数据库，此处未实现具体逻辑
    fn flush(&self) -> Result<()> {
        Ok(())
    }
}

// 实现Storage结构体的方法
impl Storage {
    /// 创建或打开一个名为database_name的数据库
    pub(crate) fn new(database_name: Option<&str>) -> Result<Self> {
        let database_name = database_name.unwrap_or(DATABASE_NAME);
        let db = DB::open_default(Storage::path(database_name))
            .map_err(|e| ChainError::StorageCannotOpenDb(e.to_string()))?;

        Ok(Self { db })
    }

    /// 获取数据库中所有的键，主要用于调试和特殊操作
    pub(crate) fn _get_all_keys<K: AsRef<[u8]>>(&self) -> Result<Vec<Box<[u8]>>> {
        let value: Vec<Box<[u8]>> = self
            .db
            .iterator(rocksdb::IteratorMode::Start)
            .map(std::result::Result::unwrap)
            .map(|(key, _)| key)
            .collect();

        Ok(value)
    }

    /// 销毁指定的数据库，主要用于测试和特殊操作
    pub(crate) fn _destroy(database_name: Option<&str>) -> Result<()> {
        let database_name = database_name.unwrap_or(DATABASE_NAME);
        DB::destroy(&Options::default(), Storage::path(database_name))
            .map_err(|e| ChainError::StorageDestroyError(e.into()))?;

        Ok(())
    }

    /// 将字节转换为字符串，主要用于错误信息的显示
    pub(crate) fn key_string<K: AsRef<[u8]>>(key: K) -> String {
        String::from_utf8(key.as_ref().to_vec()).unwrap_or_else(|_| "UNKNOWN".into())
    }

    /// 构建数据库的路径
    fn path(database_name: &str) -> PathBuf {
        Path::new(PATH).join(database_name)
    }
}

// 测试模块，用于验证Storage结构体的功能
#[cfg(test)]
mod tests {
    use crate::helpers::{deserialize, serialize, tests::STORAGE};
    use eth_trie::DB;
    use types::account::{Account, AccountData};

    // 测试数据库的创建
    #[test]
    fn it_creates_a_db() {
        let _ = STORAGE;
    }

    // 测试从数据库中获取和插入账户数据
    #[test]
    fn it_gets_and_insert_account_data_from_db() {
        let account = Account::random();
        let account_data = AccountData::new(None);
        STORAGE
            .insert(account.as_ref(), serialize(&account_data).unwrap())
            .unwrap();

        let retrieved = STORAGE.get(account.as_ref()).unwrap().unwrap();

        assert_eq!(account_data, deserialize(&retrieved).unwrap());
    }
}
