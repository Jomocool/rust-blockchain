use core::panic;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse2, DeriveInput, FieldsUnnamed};

/**
 * 实现一个过程宏，用于生成新类型结构体的Deref、DerefMut和Into trait的实现。
 *
 * # 参数
 *
 * - `input`: 一个`TokenStream2`，代表输入的Rust代码流，其中包含了一个derive输入结构体的定义。
 *
 * # 返回值
 *
 * - 返回一个`TokenStream2`，其中包含了生成的Rust代码流，即Deref、DerefMut和Into trait的实现。
 *
 * # 功能描述
 *
 * 此函数旨在为新类型结构体（例如`struct Block(SimpleBlock)`）生成常见的trait实现。
 * 它首先解析输入的结构体定义，然后检查该结构体是否为新类型结构体（即只有一个未命名字段的结构体）。
 * 如果是，它将为该结构体生成Deref、DerefMut和Into trait的实现，这些实现都指向内部的未命名字段。
 * 如果输入的结构体不是新类型结构体，函数将触发一个panic，指出错误。
 */
pub fn append(input: TokenStream2) -> TokenStream2 {
    // 解析输入的TokenStream2为DeriveInput结构体，以便获取结构体的标识符和数据结构。
    let DeriveInput { ident, data, .. } = parse2(input).unwrap();
    // 构造一个错误消息，用于在结构体不符合新类型结构体要求时显示。
    let error = format!(
        "{} is not a new type struct (e.g. struct Block(SimpleBlock))",
        ident
    );

    // 尝试从数据结构中提取未命名字段的标识符，如果结构体不是新类型结构体，则触发panic。
    let inner_ident = match data {
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => unnamed,
            _ => panic!("{}", error),
        },
        _ => panic!("{}", error),
    };

    // 使用`quote` crate生成实现Deref、DerefMut和Into trait的代码。
    let output = quote! {
        // 实现Deref trait，允许通过新类型结构体访问其内部的未命名字段。
        impl std::ops::Deref for #ident {
            type Target  = #inner_ident;

            fn deref(&self) -> &#inner_ident {
                &self.0
            }
        }

        // 实现DerefMut trait，允许通过新类型结构体修改其内部的未命名字段。
        impl std::ops::DerefMut for #ident {
            fn deref_mut(&mut self) -> &mut #inner_ident {
                &mut self.0
            }
        }

        // 实现Into trait，允许将新类型结构体转换为其内部的未命名字段。
        impl Into<#inner_ident> for #ident {
            fn into(self) -> #inner_ident {
                self.0
            }
        }
    };

    // 返回生成的代码作为TokenStream2。
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_syntax() {
        let input: TokenStream2 = quote! { pub (crate) struct Block(SimpleBlock); };
        let output = append(input.into());
        let expected: TokenStream2 = quote! {
            impl std::ops::Deref for Block {
                type Target = SimpleBlock;

                fn deref(&self) -> &SimpleBlock {
                    &self.0
                }
            }

            impl std::ops::DerefMut for Block {
                fn deref_mut(&mut self) -> &mut SimpleBlock {
                    &mut self.0
                }
            }

            impl Into<SimpleBlock> for Block {
                fn into(self) -> SimpleBlock {
                    self.0
                }
            }
        };

        assert_eq!(output.to_string(), expected.to_string());
    }
}
