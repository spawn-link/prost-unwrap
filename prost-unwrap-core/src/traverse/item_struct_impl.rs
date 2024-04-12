use proc_macro2::Span;
use quote::quote;
use strfmt::strfmt;
use syn::punctuated::Punctuated;
use syn::Fields;
use syn::Ident;
use syn::Item;
use syn::ItemImpl;
use syn::ItemStruct;
use syn::Path;
use syn::PathArguments;
use syn::PathSegment;
use syn::Token;
use syn::TypePath;

use crate::macro_args::MacroArgs;
use crate::Traverse;

pub struct StructImpl;

impl Traverse for StructImpl {
    type Item = ItemStruct;

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
    impl std::convert::TryFrom<{item_struct_ty_path}> for {struct_name} {{
        type Error = {error_fqn};

        fn try_from(value: {item_struct_ty_path}) -> Result<Self, Self::Error> {{
            Ok(Self {{
"#;
const IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_AS_IS: &str = "{field_name}: value.{field_name},";
const IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_CONVERTED: &str =
    "{field_name}: {convert_function_path}(value.{field_name})?,";
const IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_UNWRAPPED: &str = r#"
    {field_name}: value
        .{field_name}.ok_or(Self::Error {{ reason: "{mirror_struct_fqn}.{field_name} is required" }})?
        .try_into()?,
"#;
const IMPL_BLOCK_TRY_FROM_ORIGINAL_FOOTER: &str = "})}}";

fn generate_try_from_original(
    args: &MacroArgs,
    item_struct: &ItemStruct,
    mod_stack: &mut [String],
) -> super::TraverseCallbackRet {
    let mirror_struct_fqn = format!("{}.{}", mod_stack.join("."), item_struct.ident);

    let required_fields = args
        .affected_structs
        .get(&mirror_struct_fqn)
        .cloned()
        .unwrap_or(Vec::new());

    match item_struct.fields {
        Fields::Named(ref fields) => {
            let item_ty_path = item_struct_typepath(mod_stack, item_struct);
            let error_ty_path = crate::type_path::error_typepath(mod_stack);

            let mut try_from_impl_str = strfmt!(
                IMPL_BLOCK_TRY_FROM_ORIGINAL_HEADER,
                item_struct_ty_path => quote!(#item_ty_path).to_string(),
                struct_name => item_struct.ident.to_string(),
                error_fqn => quote!(#error_ty_path).to_string()
            )
            .unwrap();

            for field in &fields.named {
                if required_fields.contains(&field.ident.as_ref().unwrap().to_string()) {
                    try_from_impl_str += &strfmt!(
                        IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_UNWRAPPED,
                        field_name => field.ident.as_ref().unwrap().to_string(),
                        mirror_struct_fqn => mirror_struct_fqn.clone()
                    )
                    .unwrap();
                } else if crate::type_path::is_std_option_type(&field.ty) {
                    try_from_impl_str += &strfmt!(
                            IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_CONVERTED,
                            field_name => field.ident.as_ref().unwrap().to_string(),
                            convert_function_path => item_function_path_string(mod_stack, crate::FUNCTION_NAME_CONVERT_OPTION_TRY_FROM)
                        )
                        .unwrap();
                } else if crate::type_path::is_std_vec_type(&field.ty) {
                    try_from_impl_str += &strfmt!(
                            IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_CONVERTED,
                            field_name => field.ident.as_ref().unwrap().to_string(),
                            convert_function_path => item_function_path_string(mod_stack, crate::FUNCTION_NAME_CONVERT_VEC_TRY_FROM)
                        )
                        .unwrap();
                } else {
                    try_from_impl_str += &strfmt!(
                        IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_AS_IS,
                        field_name => field.ident.as_ref().unwrap().to_string()
                    )
                    .unwrap();
                }
            }

            try_from_impl_str += IMPL_BLOCK_TRY_FROM_ORIGINAL_FOOTER;
            let try_from_impl_block: ItemImpl = syn::parse_str(&try_from_impl_str).unwrap();

            Ok(vec![Item::Impl(try_from_impl_block)])
        }
        _ => Ok(vec![]),
    }
}

const IMPL_BLOCK_INTO_ORIGINAL_HEADER: &str = r#"
    impl std::convert::Into<{item_struct_ty_path}> for {struct_name} {{
        fn into(self) -> {item_struct_ty_path} {{
            {item_struct_ty_path} {{
"#;
const IMPL_BLOCK_INTO_ORIGINAL_FIELD_AS_IS: &str = "{field_name}: self.{field_name}.into(),";
const IMPL_BLOCK_INTO_ORIGINAL_FIELD_CONVERTED: &str =
    "{field_name}: {convert_function_path}(self.{field_name}),";
const IMPL_BLOCK_INTO_ORIGINAL_FIELD_WRAPPED: &str =
    "{field_name}: Some(self.{field_name}.into()),";
const IMPL_BLOCK_INTO_ORIGINAL_FOOTER: &str = "}}}";

fn generate_into_original(
    args: &MacroArgs,
    item_struct: &ItemStruct,
    mod_stack: &mut [String],
) -> super::TraverseCallbackRet {
    let mirror_struct_fqn = format!("{}.{}", mod_stack.join("."), item_struct.ident);

    let required_fields = args
        .affected_structs
        .get(&mirror_struct_fqn)
        .cloned()
        .unwrap_or(Vec::new());

    match item_struct.fields {
        Fields::Named(ref fields) => {
            let item_ty_path = item_struct_typepath(mod_stack, item_struct);

            let mut try_from_impl_str = strfmt!(
                IMPL_BLOCK_INTO_ORIGINAL_HEADER,
                item_struct_ty_path => quote!(#item_ty_path).to_string(),
                struct_name => item_struct.ident.to_string()
            )
            .unwrap();

            for field in &fields.named {
                if required_fields.contains(&field.ident.as_ref().unwrap().to_string()) {
                    try_from_impl_str += &strfmt!(
                        IMPL_BLOCK_INTO_ORIGINAL_FIELD_WRAPPED,
                        field_name => field.ident.as_ref().unwrap().to_string()
                    )
                    .unwrap();
                } else if crate::type_path::is_std_option_type(&field.ty) {
                    try_from_impl_str += &strfmt!(
                            IMPL_BLOCK_INTO_ORIGINAL_FIELD_CONVERTED,
                            field_name => field.ident.as_ref().unwrap().to_string(),
                            convert_function_path => item_function_path_string(mod_stack, crate::FUNCTION_NAME_CONVERT_OPTION_INTO)
                        )
                        .unwrap();
                } else if crate::type_path::is_std_vec_type(&field.ty) {
                    try_from_impl_str += &strfmt!(
                            IMPL_BLOCK_INTO_ORIGINAL_FIELD_CONVERTED,
                            field_name => field.ident.as_ref().unwrap().to_string(),
                            convert_function_path => item_function_path_string(mod_stack, crate::FUNCTION_NAME_CONVERT_VEC_INTO)
                        )
                        .unwrap();
                } else {
                    try_from_impl_str += &strfmt!(
                        IMPL_BLOCK_INTO_ORIGINAL_FIELD_AS_IS,
                        field_name => field.ident.as_ref().unwrap().to_string()
                    )
                    .unwrap();
                }
            }

            try_from_impl_str += IMPL_BLOCK_INTO_ORIGINAL_FOOTER;
            let try_from_impl_block: ItemImpl = syn::parse_str(&try_from_impl_str).unwrap();

            Ok(vec![Item::Impl(try_from_impl_block)])
        }
        _ => Ok(vec![]),
    }
}

fn item_struct_typepath(mod_stack: &mut [String], item_struct: &ItemStruct) -> TypePath {
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

fn item_function_path_string(mod_stack: &mut [String], function_name: &str) -> String {
    let super_segments = std::iter::repeat(PathSegment {
        ident: Ident::new("super", Span::call_site()),
        arguments: PathArguments::None,
    })
    .take(mod_stack.len());

    let mut segments: Punctuated<PathSegment, Token![::]> = Punctuated::new();
    segments.extend(super_segments);
    segments.push(PathSegment {
        ident: Ident::new(function_name, Span::call_site()),
        arguments: PathArguments::None,
    });

    let path = Path {
        leading_colon: None,
        segments,
    };
    quote!(#path).to_string()
}
