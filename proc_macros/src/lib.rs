mod newtype;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// 新类型派生宏
///
/// 该函数允许用户通过派生宏的方式定义新类型
/// 它接受一个结构体或枚举类型，并为其生成额外的实现代码
/// 主要用于简化新类型的创建过程，并自动实现一些常见的trait
#[proc_macro_derive(NewType)]
pub fn newtype(item: TokenStream) -> TokenStream {
    // 解析输入的token流，将其转换为可以操作的数据结构
    let input = parse_macro_input!(item);
    // 调用newtype::append函数处理输入，并将结果转换回token流
    newtype::append(input).into()
}
