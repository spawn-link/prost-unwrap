use syn::Item;
use syn::ItemEnum;

use crate::include::Config;
use crate::Traverse;

pub struct Enum;

impl Traverse for Enum {
    type Item = ItemEnum;

    fn traverse(_config: &Config, item: &Self::Item, _mod_stack: &mut Vec<String>) -> Vec<Item> {
        let mut mirror_enum = item.clone();
        super::drop_prost_derives(&mut mirror_enum.attrs);
        for variant in &mut mirror_enum.variants {
            super::drop_prost_attributes(&mut variant.attrs);
        }
        vec![Item::Enum(mirror_enum)]
    }
}
