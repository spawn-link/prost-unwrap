use syn::Item;
use syn::ItemEnum;

use crate::macro_args::MacroArgs;
use crate::Traverse;

pub struct Enum;

impl Traverse for Enum {
    type Item = ItemEnum;

    fn traverse(
        _args: &MacroArgs,
        item: &Self::Item,
        _mod_stack: &mut Vec<String>,
    ) -> crate::TraverseCallbackRet {
        let mirror_enum = item.clone();
        Ok(vec![Item::Enum(mirror_enum)])
    }
}
