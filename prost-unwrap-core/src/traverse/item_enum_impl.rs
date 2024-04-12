use proc_macro2::Span;
use quote::quote;
use strfmt::strfmt;
use syn::punctuated::Punctuated;
use syn::Fields;
use syn::Ident;
use syn::Item;
use syn::ItemEnum;
use syn::ItemImpl;
use syn::Path;
use syn::PathArguments;
use syn::PathSegment;
use syn::Token;
use syn::TypePath;

use crate::macro_args::MacroArgs;
use crate::Traverse;

pub struct EnumImpl;

impl Traverse for EnumImpl {
    type Item = ItemEnum;

    fn traverse(
        args: &MacroArgs,
        item: &Self::Item,
        mod_stack: &mut Vec<String>,
    ) -> crate::TraverseCallbackRet {
        let mut vec = Vec::with_capacity(2);
        vec.extend(generate_try_from_original(args, item, mod_stack)?);
        vec.extend(generate_into_original(args, item, mod_stack)?);
        Ok(vec)
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
    _args: &MacroArgs,
    item_enum: &ItemEnum,
    mod_stack: &mut [String],
) -> super::TraverseCallbackRet {
    let item_ty_path = item_enum_typepath(mod_stack, item_enum);
    let error_ty_path = crate::type_path::error_typepath(mod_stack);

    let mut try_from_impl_str = strfmt!(
        IMPL_BLOCK_TRY_FROM_ORIGINAL_HEADER,
        item_enum_ty_path => quote!(#item_ty_path).to_string(),
        struct_name => item_enum.ident.to_string(),
        error_fqn => quote!(#error_ty_path).to_string()
    )
    .unwrap();

    for variant in &item_enum.variants {
        if variant.fields.is_empty() {
            try_from_impl_str += &strfmt!(
                IMPL_BLOCK_TRY_FROM_VARIANT_NO_CONTENT,
                variant_name => variant.ident.to_string(),
                item_enum_ty_path => quote!(#item_ty_path).to_string()
            )
            .unwrap();
        } else {
            try_from_impl_str += &strfmt!(
                IMPL_BLOCK_TRY_FROM_VARIANT_CONTENT,
                variant_name => variant.ident.to_string(),
                item_enum_ty_path => quote!(#item_ty_path).to_string(),
                fields => variant_fields_as_string(&variant.fields, ""),
                fields_into => variant_fields_as_string(&variant.fields, ".try_into()?")
            )
            .unwrap();
        }
    }

    try_from_impl_str += IMPL_BLOCK_TRY_FROM_ORIGINAL_FOOTER;
    let try_from_impl_block: ItemImpl = syn::parse_str(&try_from_impl_str).unwrap();

    Ok(vec![Item::Impl(try_from_impl_block)])
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
    _args: &MacroArgs,
    item_enum: &ItemEnum,
    mod_stack: &mut [String],
) -> super::TraverseCallbackRet {
    let item_ty_path = item_enum_typepath(mod_stack, item_enum);
    let error_ty_path = crate::type_path::error_typepath(mod_stack);

    let mut try_from_impl_str = strfmt!(
        IMPL_BLOCK_INTO_ORIGINAL_HEADER,
        item_enum_ty_path => quote!(#item_ty_path).to_string(),
        struct_name => item_enum.ident.to_string(),
        error_fqn => quote!(#error_ty_path).to_string()
    )
    .unwrap();

    for variant in &item_enum.variants {
        if variant.fields.is_empty() {
            try_from_impl_str += &strfmt!(
                IMPL_BLOCK_INTO_VARIANT_NO_CONTENT,
                variant_name => variant.ident.to_string(),
                item_enum_ty_path => quote!(#item_ty_path).to_string()
            )
            .unwrap();
        } else {
            try_from_impl_str += &strfmt!(
                IMPL_BLOCK_INTO_VARIANT_CONTENT,
                variant_name => variant.ident.to_string(),
                item_enum_ty_path => quote!(#item_ty_path).to_string(),
                fields => variant_fields_as_string(&variant.fields, ""),
                fields_into => variant_fields_as_string(&variant.fields, ".into()")
            )
            .unwrap();
        }
    }

    try_from_impl_str += IMPL_BLOCK_INTO_ORIGINAL_FOOTER;
    let try_from_impl_block: ItemImpl = syn::parse_str(&try_from_impl_str).unwrap();

    Ok(vec![Item::Impl(try_from_impl_block)])
}

fn variant_fields_as_string(fields: &Fields, suffix: &str) -> String {
    fields
        .iter()
        .enumerate()
        .map(|(i, _field)| format!("field{i}{suffix}"))
        .collect::<Vec<String>>()
        .join(",")
}

fn item_enum_typepath(mod_stack: &mut [String], item_struct: &ItemEnum) -> TypePath {
    let super_segments = std::iter::repeat(PathSegment {
        ident: Ident::new("super", Span::call_site()),
        arguments: PathArguments::None,
    })
    .take(mod_stack.len() + 1); // +1 for the extra "super" needed, as mirror is wrapped

    let mod_segments = mod_stack.iter().map(|mod_name| PathSegment {
        ident: Ident::new(mod_name, Span::call_site()),
        arguments: PathArguments::None,
    });

    let mut segments: Punctuated<PathSegment, Token![::]> = Punctuated::new();
    segments.extend(super_segments);
    segments.extend(mod_segments);
    segments.push(PathSegment {
        ident: item_struct.ident.clone(),
        arguments: PathArguments::None,
    });

    // Construct the TypePath
    TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    }
}
