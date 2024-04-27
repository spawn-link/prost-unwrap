mod include;
mod traverse;

use include::Config;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse2;

use crate::traverse::*;

/// The idea for this macro
/// - Collect the config, that contains
///     - this module path
///     - original prost-generated structs module path
///     - maybe structs and enums postfix
///     - a prost-generated file that has a name of proto package (or specify)
///     - a number of structs specs
///         - struct fqn
///         - array of required fields
///         - array of additional attributes
///     - a number of enums
///         - enum fqn
///         - array of additional attributes
/// - For each linked file
///     - Read and load the file into token streams
///     - Traverse the AST, collecting
///         - for each struct
///             - struct fqn, AST, array of fields with field type
///             - enum fqn, AST, array of variants with variant inner type if any
/// - Traverse the collected data
///     - validate that all configured structs and enums do exist
///     - either
///         - related types (option, hashmap, vec) are also configured
///         - related types (option, hashmap, vec) are also added to the config
///     - if conditions are not met, throw an error
///         - unknown struct
///         - unknwon field
///         - related type is not configured
/// - Assemble the module AST, including
///     - error type
///     - conversion functions (option, hashmap, vec) (TryFrom<O>, Into<O>)
pub fn include(_item: TokenStream) -> TokenStream {
    let config = parse2::<Config>(_item).unwrap();
    let ast = traverse::copy_unwrapped(&config);
    quote!(#ast)
}
