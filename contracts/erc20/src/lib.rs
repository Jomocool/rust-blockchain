use std::collections::HashMap;

static mut ERC20: Option<Erc20> = None;

// wit_bindgen 通过自动生成绑定代码，简化了 WebAssembly 模块的开发，支持多语言互操作，提升开发效率和代码安全性。
wit_bindgen::generate!("erc20");

// 定义一个名为Erc20的公共结构体，用于实现ERC20代币的标准。
pub struct Erc20 {
    pub state: State,
}

// 定义一个名为State的公共结构体，用于存储ERC20代币的状态信息。
// 包含代币的名称、符号和各个账户的余额。
pub struct State {
    pub name: String,
    pub symbol: String,
    pub balances: HashMap<String, u64>,
}

// 导出Erc20合约，使其可以在其他模块中使用。
export_contract!(Erc20);

// 实现Erc20代币接口。
impl Contract for Erc20 {
    // 构造函数，用于初始化代币的名称和符号。
    fn construct(name: String, symbol: String) {
        unsafe {
            ERC20 = Some(Erc20 {
                state: State {
                    name,
                    symbol,
                    balances: HashMap::new(),
                },
            });
        }
    }

    // 铸造函数，用于向特定账户发行代币。
    // 参数account是接收代币的账户地址，amount是发行的代币数量。
    fn mint(account: String, amount: u64) {
        unsafe {
            if let Some(erc20) = &mut ERC20 {
                let balance = erc20.state.balances.entry(account).or_insert(0);
                *balance += amount;
            }
        }
    }

    // 转账函数，用于从当前账户向另一个账户转移代币。
    // 参数to是接收代币的账户地址，amount是转移的代币数量。
    fn transfer(to: String, amount: u64) {
        unsafe {
            if let Some(erc20) = &mut ERC20 {
                let from_amount = erc20
                    .state
                    .balances
                    .entry("caller".to_string())
                    .or_insert(0)
                    .to_owned();

                if from_amount >= amount {
                    *erc20.state.balances.get_mut("caller").unwrap() -= amount;
                    *erc20.state.balances.get_mut(&to).unwrap() += amount;
                }
            }
        }
    }
}
