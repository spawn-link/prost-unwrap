use syn::Item;
use syn::ItemMod;

use crate::include::Config;
use crate::Traverse;

pub struct Mod;

impl Traverse for Mod {
    type Item = ItemMod;

    fn traverse(config: &Config, item: &Self::Item, ident_stack: &mut Vec<String>) -> Vec<Item> {
        ident_stack.push(item.ident.to_string());

        let ret = if let Some((brace, ref sub_items)) = item.content {
            let copied_content = super::copy_unwrapped_items(config, ident_stack, sub_items);
            if !copied_content.is_empty() {
                let mut copied_mod = item.clone();
                copied_mod.content = Some((brace, copied_content));
                vec![Item::Mod(copied_mod)]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        ident_stack.pop();

        ret
    }
}
