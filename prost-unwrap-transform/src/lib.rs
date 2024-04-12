use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[proc_macro_error]
#[proc_macro_attribute]
pub fn required(attrs: TokenStream, item: TokenStream) -> TokenStream {
    prost_unwrap_core::required(attrs.into(), item.into()).into()
}
