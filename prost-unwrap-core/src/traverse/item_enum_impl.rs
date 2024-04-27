use quote::quote;
use strfmt::strfmt;
use syn::Fields;
use syn::Item;
use syn::ItemEnum;
use syn::ItemImpl;

use crate::include::Config;
use crate::Traverse;

pub struct EnumImpl;

impl Traverse for EnumImpl {
    type Item = ItemEnum;

    fn traverse(config: &Config, item: &Self::Item, ident_stack: &mut Vec<String>) -> Vec<Item> {
        let mut vec = Vec::with_capacity(2);
        vec.extend(generate_try_from_original(config, item, ident_stack));
        vec.extend(generate_into_original(config, item, ident_stack));
        vec
    }
}

const IMPL_BLOCK_TRY_FROM_ORIGINAL_HEADER: &str = r#"
    impl std::convert::TryFrom<{item_enum_ty_path}> for {struct_name} {{
        type Error = {error_fqn};

        fn try_from(value: {item_enum_ty_path}) -> Result<Self, Self::Error> {{
            Ok(match value {{
"#;
const IMPL_BLOCK_TRY_FROM_VARIANT_NO_CONTENT: &str =
    "{item_enum_ty_path}::{variant_name} => Self::{variant_name},";
const IMPL_BLOCK_TRY_FROM_VARIANT_CONTENT: &str =
    "{item_enum_ty_path}::{variant_name}({fields}) => Self::{variant_name}({fields_into}),";
const IMPL_BLOCK_TRY_FROM_ORIGINAL_FOOTER: &str = "})}}";

fn generate_try_from_original(
    config: &Config,
    item: &ItemEnum,
    ident_stack: &mut [String],
) -> Vec<Item> {
    let orig_item_typepath = config.orig_item_typepath(ident_stack.iter().cloned());
    let error_typepath = config.this_item_typepath([super::items::ERROR_STRUCT_NAME.to_string()]);

    let mut try_from_impl_str = strfmt!(
        IMPL_BLOCK_TRY_FROM_ORIGINAL_HEADER,
        item_enum_ty_path => quote!(#orig_item_typepath).to_string(),
        struct_name => item.ident.to_string(),
        error_fqn => quote!(#error_typepath).to_string()
    )
    .unwrap();

    for variant in &item.variants {
        if variant.fields.is_empty() {
            try_from_impl_str += &strfmt!(
                IMPL_BLOCK_TRY_FROM_VARIANT_NO_CONTENT,
                variant_name => variant.ident.to_string(),
                item_enum_ty_path => quote!(#orig_item_typepath).to_string()
            )
            .unwrap();
        } else {
            try_from_impl_str += &strfmt!(
                IMPL_BLOCK_TRY_FROM_VARIANT_CONTENT,
                variant_name => variant.ident.to_string(),
                item_enum_ty_path => quote!(#orig_item_typepath).to_string(),
                fields => variant_fields_as_string(&variant.fields, ""),
                fields_into => variant_fields_as_string(&variant.fields, ".try_into()?")
            )
            .unwrap();
        }
    }

    try_from_impl_str += IMPL_BLOCK_TRY_FROM_ORIGINAL_FOOTER;
    let try_from_impl_block: ItemImpl = syn::parse_str(&try_from_impl_str).unwrap();

    vec![Item::Impl(try_from_impl_block)]
}

const IMPL_BLOCK_INTO_ORIGINAL_HEADER: &str = r#"
    impl std::convert::Into<{item_enum_ty_path}> for {struct_name} {{
        fn into(self) -> {item_enum_ty_path} {{
            match self {{
"#;
const IMPL_BLOCK_INTO_VARIANT_NO_CONTENT: &str =
    "Self::{variant_name} => {item_enum_ty_path}::{variant_name},";
const IMPL_BLOCK_INTO_VARIANT_CONTENT: &str =
    "Self::{variant_name}({fields}) => {item_enum_ty_path}::{variant_name}({fields_into}),";
const IMPL_BLOCK_INTO_ORIGINAL_FOOTER: &str = "}}}";

fn generate_into_original(
    config: &Config,
    item: &ItemEnum,
    ident_stack: &mut [String],
) -> Vec<Item> {
    let orig_item_typepath = config.orig_item_typepath(ident_stack.iter().cloned());

    let mut try_from_impl_str = strfmt!(
        IMPL_BLOCK_INTO_ORIGINAL_HEADER,
        item_enum_ty_path => quote!(#orig_item_typepath).to_string(),
        struct_name => item.ident.to_string()
    )
    .unwrap();

    for variant in &item.variants {
        if variant.fields.is_empty() {
            try_from_impl_str += &strfmt!(
                IMPL_BLOCK_INTO_VARIANT_NO_CONTENT,
                variant_name => variant.ident.to_string(),
                item_enum_ty_path => quote!(#orig_item_typepath).to_string()
            )
            .unwrap();
        } else {
            try_from_impl_str += &strfmt!(
                IMPL_BLOCK_INTO_VARIANT_CONTENT,
                variant_name => variant.ident.to_string(),
                item_enum_ty_path => quote!(#orig_item_typepath).to_string(),
                fields => variant_fields_as_string(&variant.fields, ""),
                fields_into => variant_fields_as_string(&variant.fields, ".into()")
            )
            .unwrap();
        }
    }

    try_from_impl_str += IMPL_BLOCK_INTO_ORIGINAL_FOOTER;
    let try_from_impl_block: ItemImpl = syn::parse_str(&try_from_impl_str).unwrap();

    vec![Item::Impl(try_from_impl_block)]
}

fn variant_fields_as_string(fields: &Fields, suffix: &str) -> String {
    fields
        .iter()
        .enumerate()
        .map(|(i, _field)| format!("field{i}{suffix}"))
        .collect::<Vec<String>>()
        .join(",")
}
