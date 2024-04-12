use syn::token::Brace;
use syn::Error;
use syn::Item;
use syn::ItemMod;

use crate::macro_args::MacroArgs;
use crate::Traverse;

pub struct Mod;

impl Traverse for Mod {
    type Item = ItemMod;

    fn traverse(
        args: &MacroArgs,
        item: &Self::Item,
        mod_stack: &mut Vec<String>,
    ) -> super::TraverseCallbackRet {
        let mut mirror_mod = item.clone();
        mod_stack.push(mirror_mod.ident.to_string());

        mirror_mod.content = item
            .content
            .as_ref()
            .map(|(brace, sub_items)| fold_sub_items(args, mod_stack, brace, sub_items))
            .transpose()?;

        mod_stack.pop();

        Ok(vec![Item::Mod(mirror_mod)])
    }
}

fn fold_sub_items(
    args: &MacroArgs,
    mod_stack: &mut Vec<String>,
    brace: &Brace,
    sub_items: &[Item],
) -> Result<(Brace, Vec<Item>), Error> {
    sub_items
        .iter()
        .try_fold((brace.to_owned(), Vec::new()), |acc, sub_item| {
            process_sub_item(args, mod_stack, acc, sub_item)
        })
}

fn process_sub_item(
    args: &MacroArgs,
    mod_stack: &mut Vec<String>,
    acc: (Brace, Vec<Item>),
    sub_item: &Item,
) -> Result<(Brace, Vec<Item>), Error> {
    let items = crate::copy_modified(args, sub_item, mod_stack)?;
    let (brace, mut acc_items) = acc;
    acc_items.extend(items);
    Ok((brace, acc_items))
}
