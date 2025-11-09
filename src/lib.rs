use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn get(_attr: TokenStream, _item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(_item as DeriveInput);
    quote::quote! { #input }.into()
}
