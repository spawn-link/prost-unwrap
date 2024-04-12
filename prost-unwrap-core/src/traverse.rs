mod item_enum;
mod item_enum_impl;
mod item_mod;
mod item_struct;
mod item_struct_impl;

pub(crate) use item_enum::Enum;
pub(crate) use item_enum_impl::EnumImpl;
pub(crate) use item_mod::Mod;
pub(crate) use item_struct::Struct;
pub(crate) use item_struct_impl::StructImpl;
use syn::Item;

use crate::macro_args::MacroArgs;

pub type TraverseCallbackRetOk = Vec<Item>;
pub type TraverseCallbackRetErr = syn::Error;
pub type TraverseCallbackRet = Result<TraverseCallbackRetOk, TraverseCallbackRetErr>;

pub trait Traverse {
    type Item;
    fn traverse(
        args: &MacroArgs,
        item: &Self::Item,
        mod_stack: &mut Vec<String>,
    ) -> super::TraverseCallbackRet;
}
