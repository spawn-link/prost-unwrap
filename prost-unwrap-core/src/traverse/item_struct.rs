use syn::Fields;
use syn::Item;
use syn::ItemStruct;

use crate::macro_args::MacroArgs;
use crate::Traverse;

pub struct Struct;

impl Traverse for Struct {
    type Item = ItemStruct;

    fn traverse(
        args: &MacroArgs,
        item: &Self::Item,
        mod_stack: &mut Vec<String>,
    ) -> crate::TraverseCallbackRet {
        let mut mirror_struct = item.clone();
        let mirror_struct_fqn = format!("{}.{}", mod_stack.join("."), mirror_struct.ident);

        let required_fields = args
            .affected_structs
            .get(&mirror_struct_fqn)
            .cloned()
            .unwrap_or(Vec::new());

        match mirror_struct.fields {
            Fields::Named(ref mut fields) => {
                for field in &mut fields.named {
                    if crate::type_path::is_std_option_type(&field.ty)
                        && required_fields.contains(&field.ident.as_ref().unwrap().to_string())
                    {
                        let ty = crate::type_path::maybe_unwrap_option_type(&field.ty);
                        field.ty = ty.clone();
                    }
                }

                Ok(vec![Item::Struct(mirror_struct)])
            }
            _ => Ok(vec![Item::Struct(mirror_struct)]),
        }
    }
}
