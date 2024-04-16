mod include_args;
mod macro_args;
mod traverse;
mod type_path;

use include_args::IncludeArgs;
use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::parse2;
use syn::Item;
use syn::ItemImpl;
use syn::ItemStruct;
use syn::Token;
use syn::Visibility;

use crate::macro_args::MacroArgs;
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
    let _args = parse2::<IncludeArgs>(_item).unwrap();
    // println!("{args}");
    TokenStream::new()
}

pub fn required(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match parse2::<MacroArgs>(attr) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error(),
    };
    let item_mod = match parse2::<Item>(item) {
        Ok(Item::Mod(item_mod)) => item_mod,
        Ok(item) => abort!(item, "`required` attribute must be set for a module"),
        Err(e) => return e.to_compile_error(),
    };
    let orig_item_mod = Item::Mod(item_mod);

    match copy_modified(&args, &orig_item_mod, Vec::new().as_mut()) {
        Ok(mirror_mod_vec) => {
            let mut items_content = Vec::new();
            items_content.extend(item_error());
            items_content.extend(item_convert_option_try_from());
            items_content.extend(item_convert_option_into());
            items_content.extend(item_convert_vec_try_from());
            items_content.extend(item_convert_vec_into());
            items_content.extend(mirror_mod_vec);
            let mirror_item_mod = syn::ItemMod {
                attrs: vec![],
                vis: Visibility::Public(<Token![pub]>::default()),
                unsafety: None,
                mod_token: Default::default(),
                ident: args.mod_ident.clone(),
                content: Some((syn::token::Brace::default(), items_content)),
                semi: None,
            };

            quote!(#orig_item_mod #mirror_item_mod)
        }
        Err(e) => e.to_compile_error(),
    }
}

type TraverseCallbackRetOk = Vec<Item>;
type TraverseCallbackRetErr = syn::Error;
type TraverseCallbackRet = Result<TraverseCallbackRetOk, TraverseCallbackRetErr>;

fn copy_modified(
    args: &MacroArgs,
    item: &Item,
    mod_stack: &mut Vec<String>,
) -> TraverseCallbackRet {
    match item {
        Item::Mod(item_mod) => Mod::traverse(args, item_mod, mod_stack),
        Item::Struct(item_struct) => {
            let mut items = Vec::new();
            items.extend(Struct::traverse(args, item_struct, mod_stack)?);
            items.extend(StructImpl::traverse(args, item_struct, mod_stack)?);
            Ok(items)
        }
        Item::Enum(item_enum) => {
            let mut items = Vec::new();
            items.extend(Enum::traverse(args, item_enum, mod_stack)?);
            items.extend(EnumImpl::traverse(args, item_enum, mod_stack)?);
            Ok(items)
        }
        item => {
            println!("{}", quote!(#item));
            Ok(vec![item.clone()])
        }
    }
}

fn item_error() -> Vec<Item> {
    const DEF_BLOCK: &str = r#"
        #[derive(Debug)]
        pub struct Error {
            pub reason: &'static str,
        }
    "#;
    let item_def_block: ItemStruct = syn::parse_str(DEF_BLOCK).unwrap();

    const IMPL_BLOCK_DISPLAY: &str = r#"
        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}", self.reason)
            }
        }
    "#;
    let item_impl_block_display: ItemImpl = syn::parse_str(IMPL_BLOCK_DISPLAY).unwrap();

    const IMPL_BLOCK_STD_ERROR: &str = r#"
        impl std::error::Error for Error {}
    "#;
    let item_impl_block_error: ItemImpl = syn::parse_str(IMPL_BLOCK_STD_ERROR).unwrap();

    vec![
        Item::Struct(item_def_block),
        Item::Impl(item_impl_block_display),
        Item::Impl(item_impl_block_error),
    ]
}

const FUNCTION_NAME_CONVERT_OPTION_TRY_FROM: &str = "convert_option_try_from";
fn item_convert_option_try_from() -> Vec<Item> {
    const DEF_BLOCK: &str = r#"
        fn convert_option_try_from<U, T>(option: Option<U>) -> Result<Option<T>, T::Error>
        where
            T: TryFrom<U>,
        {
            match option {
                Some(u) => T::try_from(u).map(Some),
                None => Ok(None),
            }
        }
    "#;
    let item_def_block: Item = syn::parse_str(DEF_BLOCK).unwrap();
    vec![item_def_block]
}

const FUNCTION_NAME_CONVERT_OPTION_INTO: &str = "convert_option_into";
fn item_convert_option_into() -> Vec<Item> {
    const DEF_BLOCK: &str = r#"
        fn convert_option_into<T, U>(option: Option<T>) -> Option<U>
        where
            T: Into<U>,
        {
            option.map(Into::into)
        }
    "#;
    let item_def_block: Item = syn::parse_str(DEF_BLOCK).unwrap();
    vec![item_def_block]
}

const FUNCTION_NAME_CONVERT_VEC_TRY_FROM: &str = "convert_vec_try_from";
fn item_convert_vec_try_from() -> Vec<Item> {
    const DEF_BLOCK: &str = r#"
        fn convert_vec_try_from<T, U>(input_vec: Vec<T>) -> Result<Vec<U>, U::Error>
        where
            U: TryFrom<T>,
        {
            let mut output_vec = Vec::with_capacity(input_vec.len());

            for item in input_vec {
                let converted_item = U::try_from(item)?;
                output_vec.push(converted_item);
            }

            Ok(output_vec)
        }
    "#;
    let item_def_block: Item = syn::parse_str(DEF_BLOCK).unwrap();
    vec![item_def_block]
}

const FUNCTION_NAME_CONVERT_VEC_INTO: &str = "convert_vec_into";
fn item_convert_vec_into() -> Vec<Item> {
    const DEF_BLOCK: &str = r#"
        fn convert_vec_into<T, U>(input_vec: Vec<T>) -> Vec<U>
        where
            T: Into<U>,
        {
            input_vec.into_iter().map(Into::into).collect()
        }
    "#;
    let item_def_block: Item = syn::parse_str(DEF_BLOCK).unwrap();
    vec![item_def_block]
}
