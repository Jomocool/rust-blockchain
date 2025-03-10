use crate::error::{Result, RuntimeError};
use wasmtime::{
    self,
    component::{Component, Instance, Linker, Val},
    Config, Engine, Store,
};
use wit_component::ComponentEncoder;

/// 加载WebAssembly合约
/// 
/// 该函数接受一个字节切片作为输入，尝试将这些字节作为WebAssembly模块进行解析和加载。
/// 它首先配置WebAssembly引擎，然后创建一个存储和链接器，最后实例化WebAssembly模块。
/// 
/// # 参数
/// 
/// * `bytes`: &[u8] - WebAssembly模块的字节表示。
/// 
/// # 返回
/// 
/// * `Result<(Store<i32>, Instance)>` - 返回一个结果类型，包含WebAssembly存储和实例。
fn load_contract(bytes: &[u8]) -> Result<(Store<i32>, Instance)> {
    // 创建并配置WebAssembly配置对象
    let mut config = Config::new();

    // 启用WebAssembly组件模型
    Config::wasm_component_model(&mut config, true);

    // 根据配置创建WebAssembly引擎
    let engine = Engine::new(&config)?;
    // 创建WebAssembly存储，初始值为0
    let mut store = Store::new(&engine, 0);
    // 创建WebAssembly链接器
    let linker = Linker::new(&engine);

    // 将字节编码为WebAssembly组件
    let component_bytes = ComponentEncoder::default()
        .module(bytes)?
        .validate(true)
        .encode()?;
    // 从二进制创建WebAssembly组件
    let component = Component::from_binary(&engine, &component_bytes)?;
    // 实例化WebAssembly组件
    let instance = linker.instantiate(&mut store, &component)?;

    // 返回WebAssembly存储和实例
    Ok((store, instance))
}

/// 解析参数字符串并将其转换为指定类型的值
///
/// 此函数根据提供的字符串切片确定预期的类型和值
/// 它支持将参数解析为字符串或无符号64位整数类型
/// 如果类型不匹配已知类型，则返回错误
///
/// 参数:
/// - `chunk`: 一个包含两个元素的字符串切片，第一个元素是类型名称，第二个元素是类型的值
///
/// 返回:
/// - `Result<Val>`: 如果解析成功，则返回包含解析值的 `Ok`，
///   否则返回一个包含错误信息的 `Err`
fn parse_params(chunk: &[&str]) -> Result<Val> {
    match chunk[0] {
        // 当第一个元素是 "String" 时，将第二个元素解析为 `Val::String` 类型
        "String" => Ok(Val::String(chunk[1].into())),
        // 当第一个元素是 "U64" 时，尝试将第二个元素解析为 `Val::U64` 类型
        // 如果解析失败，`unwrap` 会触发程序崩溃
        "U64" => Ok(Val::U64(chunk[1].parse::<u64>().unwrap())),
        // 如果提供的类型不是已知类型，则返回错误
        _ => Err(RuntimeError::InvalidParamType(chunk[0].into())),
    }
}
/// 调用Wasm合约中的指定函数
///
/// 此函数负责加载Wasm合约，解析参数，并调用指定的函数
/// 它使用`load_contract`函数来加载合约，然后解析参数并调用指定的函数
///
/// # Parameters
///
/// - `bytes`: &[u8]类型，Wasm合约的字节码
/// - `function`: &str类型，要调用的函数名
/// - `params`: &[&str]类型，函数调用参数列表，每两个元素表示一个键值对
///
/// # Returns
///
/// - `Result<()>`: 表示函数调用是否成功如果成功，返回Ok(())；如果失败，返回错误类型
pub fn call_function(bytes: &[u8], function: &str, params: &[&str]) -> Result<()> {
    // 加载Wasm合约
    let (mut store, instance) = load_contract(bytes)?;

    // 解析参数，每两个元素表示一个键值对，并将它们转换为函数所需的格式
    let parsed: Result<Vec<Val>> = params.chunks_exact(2).map(parse_params).collect();

    // 记录函数名和解析后的参数
    tracing::info!("{} params {:?}", function, parsed);

    // 获取指定名称的函数导出
    let function = instance
        .get_func(&mut store, function)
        .ok_or_else(|| RuntimeError::ExportFunctionError(function.into()))?;

    // 调用函数，并处理可能的错误
    function
        .call(&mut store, &parsed?, &mut [])
        .map_err(|e| RuntimeError::CallFunctionError(e.to_string()))
}

/// 从给定的WASM字节码中提取导出的函数名
///
/// # 参数
/// * `bytes`: &[u8] - WASM模块的字节码表示
///
/// # 返回
/// * `Vec<String>` - 导出的函数名集合
///
/// # 功能描述
/// 该函数使用wasmtime引擎解析WASM字节码，并提取出所有导出的函数名
/// 首先，它创建一个新的配置对象并启用WASM组件模型
/// 然后，尝试创建一个引擎实例
/// 如果引擎创建成功，它将从字节码中创建一个模块实例，并收集所有导出的函数名
fn _contract_functions(bytes: &[u8]) -> Vec<String> {
    // 创建一个新的配置对象
    let mut config = Config::new();
    // 初始化导出的函数名集合
    let mut exports = vec![];

    // 启用WASM组件模型
    Config::wasm_component_model(&mut config, true);

    // 尝试创建一个引擎实例
    if let Ok(engine) = Engine::new(&config) {
        // 从字节码创建模块实例并收集导出的函数名
        exports = wasmtime::Module::from_binary(&engine, bytes)
            .unwrap()
            .exports()
            .map(|export| export.name().to_string())
            .collect();
    }

    // 返回导出的函数名集合
    exports
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use types::account::Account;

    const PARAMS_1: &[&str] = &["String", "Rust Coin", "String", "RustCoin"];

    fn params_2<'a>(address: &'a String) -> [&'a str; 4] {
        ["String", &address, "U64", "10"]
    }

    #[test]
    fn it_loads_a_contract() {
        let bytes = include_bytes!("./../../target/wasm32-unknown-unknown/release/erc20.wasm");
        let _loaded = load_contract(bytes).unwrap();
    }

    #[test]
    fn it_calls_contract_functions() {
        let bytes = include_bytes!("./../../target/wasm32-unknown-unknown/release/erc20.wasm");
        let address = Account::random().to_string();

        call_function(bytes, "construct", PARAMS_1).unwrap();
        call_function(bytes, "mint", &params_2(&address)).unwrap();
    }

    #[test]
    fn it_parses_string_params() {
        let parsed = parse_params(&[PARAMS_1[0], PARAMS_1[1]]).unwrap();
        assert_eq!(parsed, Val::String("Rust Coin".into()));
    }

    #[test]
    fn it_parses_u64_params() {
        let address = Account::random().to_string();
        let params = params_2(&address);
        let parsed = parse_params(&[params[2], params[3]]).unwrap();
        assert_eq!(parsed, Val::U64(10));
    }

    #[test_log::test]
    fn it_retrieves_contract_function_names() {
        let bytes = include_bytes!("./../../target/wasm32-unknown-unknown/release/erc20.wasm");
        let functions = _contract_functions(bytes);
        let expected = [
            "memory",
            "construct",
            "mint",
            "transfer",
            "cabi_realloc",
            "__data_end",
            "__heap_base",
        ];

        assert_eq!(functions, expected);
    }
}
