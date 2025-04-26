use ethereum_types::{Address, H160, H256, U256};
use lazy_static::lazy_static;
use rlp::{Encodable, RlpStream};
pub use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId, Signature as EcdsaSignature},
    generate_keypair, rand, All, Message, PublicKey, Secp256k1, SecretKey,
};
use sha3::{Digest, Keccak256};

use crate::error::{Result, UtilsError};

static ZERO_COUNT: u16 = 1;

// 使用lazy_static宏定义一个全局静态变量CONTEXT
// CONTEXT是一个Secp256k1的实例，使用All配置，这意味着启用所有的验证功能
// Secp256k1是一种椭圆曲线密码学算法，常用于比特币等加密货币中
// 全局静态变量便于在程序的任何地方访问Secp256k1的上下文，避免重复创建，提高性能
lazy_static! {
    pub(crate) static ref CONTEXT: Secp256k1<All> = Secp256k1::new();
}

/// Signature结构体用于表示一个数字签名。
/// 它包含三个字段：v, r, 和 s，这些字段共同构成了一个完整的数字签名。
/// 数字签名在区块链技术中常用于验证交易的完整性和 authenticity。
pub struct Signature {
    /// v是一个64位无符号整数，代表签名的版本信息。
    /// 这个字段帮助在ECDSA签名算法中确定正确的公钥恢复方法。
    pub v: u64,
    /// r是一个H256类型，代表签名的第一个256位组件。
    /// 这个字段是ECDSA签名算法的输出之一，用于验证签名。
    pub r: H256,
    /// s是一个H256类型，代表签名的第二个256位组件。
    /// 和r一样，s也是ECDSA签名算法的输出，对签名的验证至关重要。
    pub s: H256,
}

impl From<RecoverableSignature> for Signature {
    fn from(value: RecoverableSignature) -> Self {
        let (recovery_id, signature) = value.serialize_compact();

        let v = recovery_id.to_i32() as u64;
        let r = H256::from_slice(&signature[..32]);
        let s = H256::from_slice(&signature[32..]);

        Signature { v, r, s }
    }
}

impl TryInto<RecoverableSignature> for Signature {
    type Error = UtilsError;

    fn try_into(self) -> Result<RecoverableSignature> {
        let mut signature = [0u8; 64];
        signature[..32].copy_from_slice(&self.r.as_bytes());
        signature[32..].copy_from_slice(&self.s.as_bytes());

        let recovery_id_32 = i32::try_from(self.v).map_err(|e| {
            UtilsError::ConversionError(format!("could not convert u64 to i32: {}", e))
        })?;

        let recovery_id: RecoveryId = RecoveryId::from_i32(recovery_id_32).map_err(|e| {
            UtilsError::ConversionError(format!("could not convert i32 to RecoveryId {}", e))
        })?;
        let recoverable_signature = RecoverableSignature::from_compact(&signature, recovery_id)
            .map_err(|e| {
                UtilsError::ConversionError(format!(
                    "could not convert a signature to RecoverableSignature: {}",
                    e
                ))
            })?;

        Ok(recoverable_signature)
    }
}

impl TryInto<Vec<u8>> for Signature {
    type Error = UtilsError;

    fn try_into(self) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(65);
        bytes.extend_from_slice(self.r.as_bytes());
        bytes.extend_from_slice(self.s.as_bytes());

        let recovery_id: u8 = <u64 as TryInto<u8>>::try_into(self.v)
            .map_err(|e| UtilsError::ConversionError(e.to_string()))?;

        bytes.push(recovery_id);

        Ok(bytes)
    }
}

pub fn keypair() -> (SecretKey, PublicKey) {
    generate_keypair(&mut rand::thread_rng())
}

pub fn hash(bytes: &[u8]) -> [u8; 32] {
    Keccak256::digest(bytes).into()
}

pub fn to_address(item: &[u8]) -> H160 {
    let hash = hash(&item[1..]);
    Address::from_slice(&hash[12..])
}

pub fn public_key_address(key: &PublicKey) -> H160 {
    to_address(&key.serialize_uncompressed())
}

pub fn private_key_address(key: &SecretKey) -> H160 {
    let public_key = key.public_key(&CONTEXT);

    public_key_address(&public_key)
}

pub fn hash_message(message: &[u8]) -> Result<Message> {
    let hashed = hash(message);
    Message::from_slice(&hashed).map_err(|e| UtilsError::CreateMessage(e.to_string()))
}

pub fn sign(message: &[u8], key: &SecretKey) -> Result<EcdsaSignature> {
    let message = hash_message(message)?;
    Ok(CONTEXT.sign_ecdsa(&message, key))
}

pub fn sign_recovery(message: &[u8], key: &SecretKey) -> Result<RecoverableSignature> {
    let message = hash_message(message)?;
    Ok(CONTEXT.sign_ecdsa_recoverable(&message, key))
}

pub fn verify(message: &[u8], signature: &[u8], key: &PublicKey) -> Result<bool> {
    let message = hash_message(message)?;
    let signature = EcdsaSignature::from_compact(signature)
        .map_err(|e| UtilsError::VerifyError(e.to_string()))?;

    Ok(CONTEXT.verify_ecdsa(&message, &signature, key).is_ok())
}

/// 从给定的消息和签名中恢复出公共钥匙。
///
/// # 参数
/// * `message` - 用于生成签名的原始消息。
/// * `signature` - 消息的紧凑型ECDSA签名。
/// * `recovery_id` - 用于确定具体签名参数的整数ID。
///
/// # 返回值
/// * `Result<PublicKey>` - 恢复成功的公共钥匙，或者在恢复过程中遇到的错误。
///
/// # 错误处理
/// * 如果消息哈希失败，返回 `UtilsError::HashError`。
/// * 如果恢复ID转换失败，返回 `UtilsError::ConversionError`。
/// * 如果签名解析失败，返回 `UtilsError::VerifyError`。
/// * 如果公共钥匙恢复失败，返回 `UtilsError::RecoverError`。
pub fn recover_public_key(message: &[u8], signature: &[u8], recovery_id: i32) -> Result<PublicKey> {
    // 将消息哈希化，以便用于签名验证。
    let message = hash_message(message)?;

    // 将整数类型的恢复ID转换为所需的类型。
    let recovery_id = RecoveryId::from_i32(recovery_id)
        .map_err(|e| UtilsError::ConversionError(e.to_string()))?;

    // 从紧凑格式的签名和恢复ID中创建可恢复的签名对象。
    let signature = RecoverableSignature::from_compact(signature, recovery_id)
        .map_err(|e| UtilsError::VerifyError(e.to_string()))?;

    // 使用ECDSA算法从消息和签名中恢复公共钥匙。
    CONTEXT
        .recover_ecdsa(&message, &signature)
        .map_err(|e| UtilsError::RecoverError(e.to_string()))
}

pub fn recover_address(message: &[u8], signature: &[u8], recovery_id: i32) -> Result<Address> {
    let public_key = recover_public_key(message, signature, recovery_id)?;
    Ok(public_key_address(&public_key))
}

/// 使用RLP编码给定的项和可选的签名
///
/// RLP编码是一种用于编码任意数据的方案，主要用于以太坊网络
/// 本函数接受一个可编码项的向量和一个可选的签名，然后将它们编码为一个RLP流
///
/// # 参数
/// - `items`: 一个实现了Encodable trait的类型向量，表示要编码的项
/// - `signature`: 一个可选的签名引用，如果存在，将与项一起编码
///
/// # 返回值
/// 返回一个RLPStream实例，它包含了编码后的数据
pub fn rlp_encode<T: Encodable>(items: Vec<T>, signature: Option<&Signature>) -> RlpStream {
    // 初始化RLP流
    let mut stream = RlpStream::new();
    // 计算列表大小，如果存在签名，则增加3个元素
    let mut list_size = items.len();

    // 如果有签名，列表大小增加3，因为签名由v、r和s三个部分组成
    if signature.is_some() {
        list_size += 3
    }

    // 开始列表，指定列表大小
    stream.begin_list(list_size);

    // 遍历项并添加到流中
    items.iter().for_each(|item| {
        stream.append(item);
    });

    // 如果签名存在，将其v、r和s部分添加到流中
    if let Some(signature) = signature {
        // 添加签名的v值
        stream.append(&signature.v);
        // 添加签名的r值，转换为U256类型
        stream.append(&U256::from_big_endian(signature.r.as_bytes()));
        // 添加签名的s值，转换为U256类型
        stream.append(&U256::from_big_endian(signature.s.as_bytes()));
    }

    // 返回构建好的RLP流
    stream
}

/// 检查给定的哈希值是否有效
///
/// 有效性是指哈希值的前`ZERO_COUNT`个字节是否全部为0
/// 这个函数用于验证哈希值是否满足特定的难度条件
///
/// # 参数
///
/// * `hash` - 一个`H256`类型的哈希值，表示待验证的哈希
///
/// # 返回值
///
/// 返回一个布尔值，如果哈希值的前`ZERO_COUNT`个字节都为0，则返回`true`，否则返回`false`
pub fn is_valid_hash(hash: H256) -> bool {
    // 迭代哈希值的前`ZERO_COUNT`个字节，检查它们是否都为0
    // `iter`用于遍历哈希值的每个字节
    // `take`限制遍历的字节数为`ZERO_COUNT`
    // `all`确保选取的这些字节都满足条件（即都为0）
    hash.0.iter().take(ZERO_COUNT as usize).all(|&x| x == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_and_public_key_addresses_match() {
        let (secret_key, public_key) = keypair();
        let private_key_address = private_key_address(&secret_key);
        let public_key_address = public_key_address(&public_key);
        assert_eq!(private_key_address, public_key_address);
    }

    #[test]
    fn it_hashes() {
        let message = b"The message";
        let hashed = hash(message);
        assert_eq!(
            hashed,
            [
                174, 253, 38, 204, 75, 207, 36, 167, 252, 109, 46, 248, 163, 40, 95, 14, 14, 198,
                197, 2, 119, 153, 141, 102, 195, 214, 250, 111, 247, 123, 45, 64
            ]
        );
    }

    #[test]
    fn it_recovers() {
        let (secret_key, public_key) = keypair();
        let message = b"The message";
        let signature = sign_recovery(message, &secret_key).unwrap();
        let (recovery_id, serialized_signature) = signature.serialize_compact();
        let recovered_public_key =
            recover_public_key(message, &serialized_signature, recovery_id.to_i32()).unwrap();

        assert_eq!(recovered_public_key, public_key);

        let recovered_address =
            recover_address(message, &serialized_signature, recovery_id.to_i32()).unwrap();
        assert_eq!(recovered_address, public_key_address(&public_key));
    }

    #[test]
    fn it_verifies() {
        let (secret_key, public_key) = keypair();
        let message = b"The message";

        let signature = sign(message, &secret_key).unwrap();
        let serialized_signature = signature.serialize_compact();
        let verified = verify(message, &serialized_signature, &public_key).unwrap();
        assert!(verified);

        let signature = sign_recovery(message, &secret_key).unwrap();
        let (_, serialized_signature) = signature.serialize_compact();
        let verified = verify(message, &serialized_signature, &public_key).unwrap();
        assert!(verified);
    }

    #[test]
    fn it_rlp_encodes() {
        let items = vec!["a", "b", "c", "d", "e", "f"];
        let stream = rlp_encode(items, None);

        assert_eq!(stream.out().to_vec(), b"\xc6abcdef".to_vec());
    }
}
