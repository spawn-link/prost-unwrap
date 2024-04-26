use std::collections::HashMap;

use proc_macro_error::abort;
use quote::quote;
use strfmt::strfmt;
use syn::Fields;
use syn::Item;
use syn::ItemImpl;
use syn::ItemStruct;

use crate::include::spec_tree::SpecTreeLeaf;
use crate::include::Config;
use crate::traverse::Traverse;

pub struct StructImpl;

impl Traverse for StructImpl {
    type Item = ItemStruct;

    fn traverse(config: &Config, item: &Self::Item, ident_stack: &mut Vec<String>) -> Vec<Item> {
        let mut vec = Vec::with_capacity(2);
        vec.extend(generate_try_from_original(config, item, ident_stack));
        vec.extend(generate_into_original(config, item, ident_stack));
        vec
    }
}

const IMPL_BLOCK_TRY_FROM_ORIGINAL_HEADER: &str = r#"
    impl std::convert::TryFrom<{orig_item_typepath}> for {struct_name} {{
        type Error = {error_typepath};

        fn try_from(value: {orig_item_typepath}) -> Result<Self, Self::Error> {{
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
    config: &Config,
    item: &ItemStruct,
    ident_stack: &mut Vec<String>,
) -> Vec<Item> {
    let mirror_struct_path = ident_stack
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let required_fields = match config.spec_tree.get_leaf(&mirror_struct_path) {
        Some(_struct_leaf @ SpecTreeLeaf::Struct(struct_spec)) => struct_spec.fields_map(),
        Some(enum_leaf @ SpecTreeLeaf::Enum { .. }) => abort!(
            enum_leaf.fqn_ref(),
            "Expected specified item to be enum, but struct found"
        ),
        None => HashMap::new(),
    };

    let ret = match item.fields {
        Fields::Named(ref fields) => {
            let orig_item_typepath = config.orig_item_typepath(ident_stack.iter().cloned());
            let error_typepath = config.error_typepath();

            let mut try_from_impl = vec![strfmt!(
                IMPL_BLOCK_TRY_FROM_ORIGINAL_HEADER,
                struct_name => item.ident.to_string(),
                orig_item_typepath => quote!(#orig_item_typepath).to_string(),
                error_typepath => quote!(#error_typepath).to_string()
            )
            .unwrap()];

            for field in &fields.named {
                let field_name = field
                    .ident
                    .as_ref()
                    .expect("Expected field ident to be Some")
                    .to_string();
                let is_required_field = required_fields.contains_key(&field_name);
                let is_std_option_type = super::is_std_option_type(&field.ty);
                let is_std_vec_type = super::is_std_vec_type(&field.ty);

                match (is_required_field, is_std_option_type, is_std_vec_type) {
                    // field is required, is an Option<T>, unwrap it
                    (true, true, _) => {
                        try_from_impl.push(
                            strfmt!(
                                IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_UNWRAPPED,
                                field_name => field.ident.as_ref().unwrap().to_string(),
                                mirror_struct_fqn => mirror_struct_path.join("::")
                            )
                            .unwrap(),
                        );
                    }
                    // field is required, but is not an Option<T>, throw an error
                    (true, false, _) => {
                        let ty = &field.ty;
                        abort!(
                            required_fields.get(&field_name).unwrap(),
                            format!(
                                "Field has type `{}`, which is not an Option<T> type",
                                quote!(#ty)
                            )
                        );
                    }
                    // field is not required, but is an Option<T>: convert with a function call
                    (_, true, _) => {
                        let convert_fn_typepath = config.this_item_typepath(vec![
                            super::items::FUNCTION_NAME_CONVERT_OPTION_TRY_FROM.to_string(),
                        ]);
                        try_from_impl.push(
                            strfmt!(
                                IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_CONVERTED,
                                field_name => field_name,
                                convert_function_path => quote!(#convert_fn_typepath).to_string()
                            )
                            .unwrap(),
                        );
                    }
                    // field is not required, but is a Vec<T>: convert with a function call
                    (_, _, true) => {
                        let convert_fn_typepath = config.this_item_typepath(vec![
                            super::items::FUNCTION_NAME_CONVERT_VEC_TRY_FROM.to_string(),
                        ]);
                        try_from_impl.push(
                            strfmt!(
                                IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_CONVERTED,
                                field_name => field_name,
                                convert_function_path => quote!(#convert_fn_typepath).to_string()
                            )
                            .unwrap(),
                        );
                    }
                    // field is not required, not an Option<T> nor Vec<T>: pass as is
                    (_, _, _) => {
                        try_from_impl.push(
                            strfmt!(
                                IMPL_BLOCK_TRY_FROM_ORIGINAL_FIELD_AS_IS,
                                field_name => field_name
                            )
                            .unwrap(),
                        );
                    }
                }
            }

            try_from_impl.push(IMPL_BLOCK_TRY_FROM_ORIGINAL_FOOTER.to_string());
            let try_from_impl_block: ItemImpl =
                syn::parse_str(try_from_impl.join("").as_str()).unwrap();

            vec![Item::Impl(try_from_impl_block)]
        }
        _ => vec![],
    };

    ret
}

const IMPL_BLOCK_INTO_ORIGINAL_HEADER: &str = r#"
    impl std::convert::Into<{orig_item_typepath}> for {struct_name} {{
        fn into(self) -> {orig_item_typepath} {{
            {orig_item_typepath} {{
"#;
const IMPL_BLOCK_INTO_ORIGINAL_FIELD_AS_IS: &str = "{field_name}: self.{field_name}.into(),";
const IMPL_BLOCK_INTO_ORIGINAL_FIELD_CONVERTED: &str =
    "{field_name}: {convert_function_path}(self.{field_name}),";
const IMPL_BLOCK_INTO_ORIGINAL_FIELD_WRAPPED: &str =
    "{field_name}: Some(self.{field_name}.into()),";
const IMPL_BLOCK_INTO_ORIGINAL_FOOTER: &str = "}}}";

fn generate_into_original(
    config: &Config,
    item: &ItemStruct,
    ident_stack: &mut Vec<String>,
) -> Vec<Item> {
    let mirror_struct_path = ident_stack
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let required_fields = match config.spec_tree.get_leaf(&mirror_struct_path) {
        Some(_struct_leaf @ SpecTreeLeaf::Struct(struct_spec)) => struct_spec.fields_map(),
        Some(enum_leaf @ SpecTreeLeaf::Enum { .. }) => abort!(
            enum_leaf.fqn_ref(),
            "Expected specified item to be enum, but struct found"
        ),
        None => HashMap::new(),
    };

    let ret = match item.fields {
        Fields::Named(ref fields) => {
            let orig_item_typepath = config.orig_item_typepath(ident_stack.iter().cloned());

            let mut try_from_impl = vec![strfmt!(
                IMPL_BLOCK_INTO_ORIGINAL_HEADER,
                orig_item_typepath => quote!(#orig_item_typepath).to_string(),
                struct_name => item.ident.to_string()
            )
            .unwrap()];

            for field in &fields.named {
                let field_name = field
                    .ident
                    .as_ref()
                    .expect("Expected field ident to be Some")
                    .to_string();
                let is_required_field = required_fields.contains_key(&field_name);
                let is_std_option_type = super::is_std_option_type(&field.ty);
                let is_std_vec_type = super::is_std_vec_type(&field.ty);

                match (is_required_field, is_std_option_type, is_std_vec_type) {
                    // field is required and is an Option<T>, wrap it into Some()
                    (true, true, _) => {
                        try_from_impl.push(
                            strfmt!(
                                IMPL_BLOCK_INTO_ORIGINAL_FIELD_WRAPPED,
                                field_name => field.ident.as_ref().unwrap().to_string()
                            )
                            .unwrap(),
                        );
                    }
                    // field is required but is not an Option<T>, throw an error
                    (true, false, _) => {
                        let ty = &field.ty;
                        abort!(
                            required_fields.get(&field_name).unwrap(),
                            format!(
                                "Field has type `{}`, which is not an Option<T> type",
                                quote!(#ty)
                            )
                        );
                    }
                    // field is not required but is an Option<T>, convert it with a function call
                    (_, true, _) => {
                        let convert_fn_typepath = config.this_item_typepath(vec![
                            super::items::FUNCTION_NAME_CONVERT_OPTION_INTO.to_string(),
                        ]);
                        try_from_impl.push(
                            strfmt!(
                                IMPL_BLOCK_INTO_ORIGINAL_FIELD_CONVERTED,
                                field_name => field_name,
                                convert_function_path => quote!(#convert_fn_typepath).to_string()
                            )
                            .unwrap(),
                        );
                    }
                    // field is not required but is a Vec<T>, convert it with a function call
                    (_, _, true) => {
                        let convert_fn_typepath = config.this_item_typepath(vec![
                            super::items::FUNCTION_NAME_CONVERT_VEC_INTO.to_string(),
                        ]);
                        try_from_impl.push(
                            strfmt!(
                                IMPL_BLOCK_INTO_ORIGINAL_FIELD_CONVERTED,
                                field_name => field_name,
                                convert_function_path => quote!(#convert_fn_typepath).to_string()
                            )
                            .unwrap(),
                        );
                    }
                    // field is not required, not an Option<T> nor Vec<T>, pass as is
                    (_, _, _) => {
                        try_from_impl.push(
                            strfmt!(
                                IMPL_BLOCK_INTO_ORIGINAL_FIELD_AS_IS,
                                field_name => field_name
                            )
                            .unwrap(),
                        );
                    }
                }
            }

            try_from_impl.push(IMPL_BLOCK_INTO_ORIGINAL_FOOTER.to_string());
            let try_from_impl_block: ItemImpl =
                syn::parse_str(try_from_impl.join("").as_str()).unwrap();

            vec![Item::Impl(try_from_impl_block)]
        }
        _ => vec![],
    };

    ret
}
