use proc_macro_error::abort;
use quote::quote;
use syn::Fields;
use syn::Item;
use syn::ItemStruct;

use crate::include::spec_tree::SpecTreeLeaf;
use crate::include::Config;
use crate::traverse::Traverse;

pub struct Struct;

impl Traverse for Struct {
    type Item = ItemStruct;

    fn traverse(config: &Config, item: &Self::Item, ident_stack: &mut Vec<String>) -> Vec<Item> {
        let mirror_struct_path = ident_stack
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        let mirror_struct = match config.spec_tree.get_leaf(&mirror_struct_path) {
            None => {
                let mut mirror_struct = item.clone();
                super::drop_prost_derives(&mut mirror_struct.attrs);
                for field in &mut mirror_struct.fields {
                    super::drop_prost_attributes(&mut field.attrs);
                }
                Item::Struct(mirror_struct)
            }
            Some(enum_leaf @ SpecTreeLeaf::Enum { .. }) => abort!(
                enum_leaf.fqn_ref(),
                "Expected specified item to be enum, but struct found"
            ),
            Some(struct_leaf @ SpecTreeLeaf::Struct(struct_spec)) => {
                let mut mirror_struct = item.clone();

                super::drop_prost_derives(&mut mirror_struct.attrs);

                let mut required_fields = struct_spec.fields_map();
                match mirror_struct.fields {
                    Fields::Named(ref mut fields) => {
                        for field in &mut fields.named {
                            super::drop_prost_attributes(&mut field.attrs);

                            let field_name = field
                                .ident
                                .as_ref()
                                .expect("Expected field ident to be Some")
                                .to_string();
                            let is_required_field = required_fields.contains_key(&field_name);
                            let is_std_option_type = super::is_std_option_type(&field.ty);
                            let is_std_vec_type = super::is_std_vec_type(&field.ty);

                            match (is_required_field, is_std_option_type, is_std_vec_type) {
                                (true, true, _) => {
                                    let ty = super::maybe_unwrap_option_type(&field.ty);
                                    field.ty = ty.clone();
                                }
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
                                (_, _, _) => (),
                            }

                            required_fields.remove(&field_name);
                        }

                        if !required_fields.is_empty() {
                            abort!(
                                struct_leaf.fqn_ref(),
                                format!(
                                    "Required fields missing from struct definition: {}",
                                    required_fields.into_keys().collect::<Vec<_>>().join(", ")
                                )
                            )
                        }

                        Item::Struct(mirror_struct)
                    }
                    _ => abort!(
                        struct_leaf.fqn_ref(),
                        "Expected struct to have named fields"
                    ),
                }
            }
        };

        vec![mirror_struct]
    }
}
