extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemStruct};

#[proc_macro_attribute]
pub fn dirty_module(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        #input

        impl #name {
            pub fn new_boxed(sample_rate: f32) -> Box<dyn dirtyrack_sdk::RackDspNode> {
                Box::new(Self::new(sample_rate))
            }

            pub fn as_any_mut_impl(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };

    TokenStream::from(expanded)
}

/// 1ボイス分のロジックを16ボイスのループに展開するマクロ
#[proc_macro_attribute]
pub fn voice_process(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let vis = &input.vis;
    let block = &input.block;
    let sig = &input.sig;

    // TODO: 引数の解析を行い、どの入力をどこにマッピングするかを自動化する
    // 現状は単純な関数定義を維持しつつ、ラッパーを生成するイメージ
    
    let expanded = quote! {
        #vis #sig #block

        // このマクロは将来的に、RackDspNode::processを自動生成するために使用される
    };

    TokenStream::from(expanded)
}
