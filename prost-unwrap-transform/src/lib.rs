use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[proc_macro_error]
#[proc_macro]
pub fn include(item: TokenStream) -> TokenStream {
    prost_unwrap_core::include(item.into()).into()
}
