use std::collections::HashMap;

use proc_macro_error::abort;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::Ident;
use syn::LitStr;
use syn::Token;

pub struct MacroArgs {
    pub mod_ident: Ident,
    pub affected_structs: HashMap<String, Vec<String>>,
}

impl Parse for MacroArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mod_ident: Ident = input.parse()?;
        let _: Comma = input.parse()?;

        let paths_content;
        syn::bracketed!(paths_content in input);
        let strings: Punctuated<LitStr, Comma> =
            paths_content.parse_terminated(|input: ParseStream| input.parse(), Token![,])?;

        let mut affected_structs: HashMap<String, Vec<String>> = HashMap::new();
        for (fqn, field) in strings
            .iter()
            .flat_map(Self::from_litstr)
            .collect::<Vec<(String, String)>>()
        {
            affected_structs
                .entry(fqn)
                .and_modify(|fields| fields.push(field.clone()))
                .or_insert(vec![field.clone()]);
        }

        Ok(MacroArgs {
            mod_ident,
            affected_structs,
        })
    }
}

impl MacroArgs {
    /// Takes a single string literal and expands it into a vector of tuples
    /// `(struct_fqn, field_name)`.
    ///
    /// For example,
    /// - the literal `foo.bar.Baz.a` would be expanded into `[("foo.bar.Baz", "a")]`;
    /// - the literal `foo.bar.Baz.{a,b,c}` would be expanded into
    /// `[("foo.bar.Baz", "a"), ("foo.bar.Baz", "b"), ("foo.bar.Baz", "c")]`
    fn from_litstr(lit: &LitStr) -> Vec<(String, String)> {
        let value = lit.value();
        let parts: Vec<&str> = value.split('.').collect();

        if parts.len() < 3 {
            abort!(lit.span(), "Expected format <package>.<message>.<field>");
        }

        let fqn = parts[..parts.len() - 1].join(".");
        Self::parse_fields(parts.last().unwrap().to_string())
            .into_iter()
            .map(|field| (fqn.clone(), field))
            .collect()
    }

    fn parse_fields(field: String) -> Vec<String> {
        if field.starts_with('{') {
            return Self::parse_fields_multiple(field);
        }
        vec![field]
    }

    fn parse_fields_multiple(field: String) -> Vec<String> {
        field
            .strip_prefix('{')
            .unwrap()
            .strip_suffix('}')
            .unwrap()
            .split(',')
            .map(|field| field.trim().to_string())
            .collect()
    }
}
