use proc_macro_error::abort;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::Attribute;
use syn::File;
use syn::GenericArgument;
use syn::Item;
use syn::Meta;
use syn::PathArguments;
use syn::Token;
use syn::Type;
use syn::TypePath;

use crate::include::Config;

mod item_enum;
mod item_enum_impl;
mod item_mod;
mod item_struct;
mod item_struct_impl;

pub trait Traverse {
    type Item;
    fn traverse(config: &Config, item: &Self::Item, ident_stack: &mut Vec<String>) -> Vec<Item>;
}

pub(crate) fn copy_unwrapped(config: &Config) -> File {
    let ast = &config.source.ast;
    let mut ident_stack = Vec::new();
    let mut items = Vec::new();

    items.extend(items::item_error());
    items.extend(items::item_convert_option_try_from());
    items.extend(items::item_convert_option_into());
    items.extend(items::item_convert_vec_try_from());
    items.extend(items::item_convert_vec_into());
    items.extend(items::item_convert_hashmap_try_from());
    items.extend(items::item_convert_hashmap_into());
    items.extend(copy_unwrapped_items(config, &mut ident_stack, &ast.items));

    File {
        shebang: None,
        attrs: Vec::new(),
        items,
    }
}

fn copy_unwrapped_items<'a, I: IntoIterator<Item = &'a Item>>(
    config: &'a Config,
    ident_stack: &mut Vec<String>,
    items: I,
) -> Vec<Item> {
    let mut ret_items = Vec::new();
    for item in items.into_iter() {
        let copied_items = match item {
            Item::Mod(item_mod) => item_mod::Mod::traverse(config, item_mod, ident_stack),
            Item::Struct(item_struct) => {
                let mut items = Vec::new();
                ident_stack.push(item_struct.ident.to_string());
                items.extend(item_struct::Struct::traverse(
                    config,
                    item_struct,
                    ident_stack,
                ));
                items.extend(item_struct_impl::StructImpl::traverse(
                    config,
                    item_struct,
                    ident_stack,
                ));
                ident_stack.pop();
                items
            }
            Item::Enum(item_enum) => {
                let mut items = Vec::new();
                ident_stack.push(item_enum.ident.to_string());
                items.extend(item_enum::Enum::traverse(config, item_enum, ident_stack));
                items.extend(item_enum_impl::EnumImpl::traverse(
                    config,
                    item_enum,
                    ident_stack,
                ));
                ident_stack.pop();
                items
            }
            _item => Vec::new(),
        };
        ret_items.extend(copied_items);
    }

    ret_items
}

fn drop_prost_derives(attrs: &mut Vec<Attribute>) {
    for attr in attrs {
        if attr.path().is_ident("derive") {
            if let Meta::List(ref mut meta_list) = attr.meta {
                let nested = match meta_list
                    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                {
                    Ok(nested) => nested,
                    Err(e) => {
                        eprintln!("Error parsing derive attributes: {}", e);
                        continue; // Skip to the next attribute if parsing fails
                    }
                };

                let filtered: Punctuated<Meta, Token![,]> = nested
                    .into_iter()
                    .filter(|meta| match meta {
                        Meta::Path(path) => path
                            .segments
                            .first()
                            .map_or(true, |segment| segment.ident != *"prost"),
                        _ => true,
                    })
                    .collect();

                meta_list.tokens = quote!(#filtered);
            }
        }
    }
}

fn drop_prost_attributes(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path().is_ident("prost"));
}

pub(crate) fn maybe_unwrap_option_type(ty: &Type) -> &Type {
    if let Type::Path(ty_path) = ty {
        if let Some(last_segment) = ty_path.path.segments.last() {
            if let PathArguments::AngleBracketed(angle_bracketed_args) = &last_segment.arguments {
                if let Some(syn::GenericArgument::Type(inner_ty)) =
                    angle_bracketed_args.args.first()
                {
                    return inner_ty;
                }
            }
        }
    }
    abort!(ty, "cannot unwrap the type path");
}

pub(crate) fn is_std_option_type(ty: &Type) -> bool {
    if let Type::Path(ty_path) = ty {
        return ty_path.path.segments.last().map_or(false, |segment| {
            segment.ident == "Option" && !segment.arguments.is_empty()
        });
    }
    false
}

pub(crate) fn is_std_vec_type(ty: &Type) -> bool {
    if let Type::Path(ty_path) = ty {
        return ty_path.path.segments.last().map_or(false, |segment| {
            segment.ident == "Vec" && !segment.arguments.is_empty()
        });
    }
    false
}

// the functions for detecting non-scalar types are hacky; the proper types
// handling will follow during the partial copying implementation.
// Note: `Vec` is here to handle Vec<Vec<u8>> which corresponds to repeated
// bytes type in protobuf definition. No other protobuf type may be mapped into
// Vec<Vec<T>>.
const SCALAR_TYPES: [&str; 16] = [
    "f64", "f32", "i32", "i64", "u32", "u64", "i32", "i64", "u8", "u32", "u64", "i32", "i64",
    "bool", "String", "Vec",
];

pub(crate) fn is_std_vec_non_scalar(ty: &Type) -> bool {
    if let Type::Path(ty_path) = ty {
        return ty_path.path.segments.last().map_or(false, |segment| {
            if segment.ident == "Vec" && !segment.arguments.is_empty() {
                if let PathArguments::AngleBracketed(ref args) = segment.arguments {
                    if let Some(GenericArgument::Type(Type::Path(ref ty_path))) = args.args.first()
                    {
                        return is_type_non_scalar(ty_path);
                    }
                }
            }
            false
        });
    }
    false
}

pub(crate) fn is_std_hashmap_non_scalar(ty: &Type) -> bool {
    if let Type::Path(ty_path) = ty {
        return ty_path.path.segments.last().map_or(false, |segment| {
            if segment.ident == "HashMap" && !segment.arguments.is_empty() {
                if let PathArguments::AngleBracketed(ref args) = segment.arguments {
                    if let Some(GenericArgument::Type(Type::Path(ref ty_path))) = args.args.last() {
                        return is_type_non_scalar(ty_path);
                    }
                }
            }
            false
        });
    }
    false
}

pub(crate) fn is_type_non_scalar(ty_path: &TypePath) -> bool {
    !SCALAR_TYPES.contains(
        &ty_path
            .path
            .segments
            .last()
            .unwrap()
            .ident
            .to_string()
            .as_str(),
    )
}

pub(crate) mod items {
    use syn::Item;
    use syn::ItemImpl;
    use syn::ItemStruct;

    pub const ERROR_STRUCT_NAME: &str = "Error";
    pub(crate) fn item_error() -> Vec<Item> {
        const DEF_BLOCK: &str = r#"
            #[derive(Debug)]
            pub struct Error {
                pub reason: &'static str,
            }
        "#;
        let item_def_block: ItemStruct =
            syn::parse_str(DEF_BLOCK).expect("Expected Error definition");

        const IMPL_BLOCK_DISPLAY: &str = r#"
            impl std::fmt::Display for Error {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    write!(f, "{}", self.reason)
                }
            }
        "#;
        let item_impl_block_display: ItemImpl =
            syn::parse_str(IMPL_BLOCK_DISPLAY).expect("Expected impl Display for Error");

        const IMPL_BLOCK_STD_ERROR: &str = r#"
            impl std::error::Error for Error {}
        "#;
        let item_impl_block_error: ItemImpl = syn::parse_str(IMPL_BLOCK_STD_ERROR)
            .expect("Expected impl std::error::Error for Error");

        vec![
            Item::Struct(item_def_block),
            Item::Impl(item_impl_block_display),
            Item::Impl(item_impl_block_error),
        ]
    }

    pub const FUNCTION_NAME_CONVERT_OPTION_TRY_FROM: &str = "convert_option_try_from";
    pub(crate) fn item_convert_option_try_from() -> Vec<Item> {
        const DEF_BLOCK: &str = r#"
            pub fn convert_option_try_from<U, T>(option: Option<U>) -> Result<Option<T>, T::Error>
            where
                T: TryFrom<U>,
            {
                match option {
                    Some(u) => T::try_from(u).map(Some),
                    None => Ok(None),
                }
            }
        "#;
        let item_def_block: Item =
            syn::parse_str(DEF_BLOCK).expect("Expected item_convert_option_try_from definition");
        vec![item_def_block]
    }

    pub const FUNCTION_NAME_CONVERT_OPTION_INTO: &str = "convert_option_into";
    pub(crate) fn item_convert_option_into() -> Vec<Item> {
        const DEF_BLOCK: &str = r#"
            pub fn convert_option_into<T, U>(option: Option<T>) -> Option<U>
            where
                T: Into<U>,
            {
                option.map(Into::into)
            }
        "#;
        let item_def_block: Item =
            syn::parse_str(DEF_BLOCK).expect("Expected item_convert_option_into definition");
        vec![item_def_block]
    }

    pub const FUNCTION_NAME_CONVERT_VEC_TRY_FROM: &str = "convert_vec_try_from";
    pub(crate) fn item_convert_vec_try_from() -> Vec<Item> {
        const DEF_BLOCK: &str = r#"
            pub fn convert_vec_try_from<T, U>(input_vec: Vec<T>) -> Result<Vec<U>, U::Error>
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
        let item_def_block: Item =
            syn::parse_str(DEF_BLOCK).expect("Expected item_convert_vec_try_from definition");
        vec![item_def_block]
    }

    pub const FUNCTION_NAME_CONVERT_VEC_INTO: &str = "convert_vec_into";
    pub(crate) fn item_convert_vec_into() -> Vec<Item> {
        const DEF_BLOCK: &str = r#"
            pub fn convert_vec_into<T, U>(input_vec: Vec<T>) -> Vec<U>
            where
                T: Into<U>,
            {
                input_vec.into_iter().map(Into::into).collect()
            }
        "#;
        let item_def_block: Item =
            syn::parse_str(DEF_BLOCK).expect("Expected item_convert_vec_into definition");
        vec![item_def_block]
    }

    pub const FUNCTION_NAME_CONVERT_HASHMAP_TRY_FROM: &str = "convert_hashmap_try_from";
    pub(crate) fn item_convert_hashmap_try_from() -> Vec<Item> {
        const DEF_BLOCK: &str = r#"
            pub fn convert_hashmap_try_from<K, T, U>(
                input_map: std::collections::HashMap<K, T>,
            ) -> Result<std::collections::HashMap<K, U>, U::Error>
            where
                U: TryFrom<T>,
                K: std::cmp::Eq + std::hash::Hash,
            {
                let mut output_map = std::collections::HashMap::with_capacity(input_map.len());
                for (key, value) in input_map {
                    let converted_value = U::try_from(value)?;
                    output_map.insert(key, converted_value);
                }
                Ok(output_map)
            }
        "#;
        let item_def_block: Item =
            syn::parse_str(DEF_BLOCK).expect("Expected item_convert_hashmap_try_from definition");
        vec![item_def_block]
    }

    pub const FUNCTION_NAME_CONVERT_HASHMAP_INTO: &str = "convert_hashmap_into";
    pub(crate) fn item_convert_hashmap_into() -> Vec<Item> {
        const DEF_BLOCK: &str = r#"
            pub fn convert_hashmap_into<K, T, U>(
                input_map: std::collections::HashMap<K, T>,
            ) -> std::collections::HashMap<K, U>
            where
                T: Into<U>,
                K: std::cmp::Eq + std::hash::Hash,
            {
                let mut output_map = std::collections::HashMap::with_capacity(input_map.len());
                for (key, value) in input_map {
                    let converted_value = value.into();
                    output_map.insert(key, converted_value);
                }
                output_map
            }
        "#;
        let item_def_block: Item =
            syn::parse_str(DEF_BLOCK).expect("Expected item_convert_hashmap_into definition");
        vec![item_def_block]
    }
}
