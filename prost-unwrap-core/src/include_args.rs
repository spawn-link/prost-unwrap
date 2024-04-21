use std::fmt::Display;
use std::fs;
use std::path::PathBuf;

use derive_builder::Builder;
use proc_macro2::Span;
use proc_macro_error::abort;
use proc_macro_error::abort_call_site;
use quote::quote;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Expr;
use syn::ExprLit;
use syn::File;
use syn::Ident;
use syn::Lit;
use syn::Path;
use syn::Token;

use self::spec_tree::SpecTreeLeaf;
use self::spec_tree::SpecTreeNode;

/// The `include!`` macro arguments.
/// Contains all needed information for AST traversal.
#[derive(Builder, Clone)]
pub struct IncludeArgs {
    pub this_mod_path: Path,
    pub orig_mod_path: Path,
    #[builder(default = "None")]
    pub items_suffix: Option<Ident>,
    pub sources: Vec<SourceFile>,
    pub spec_tree: SpecTreeNode,
}

impl Display for IncludeArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let this_mod_path = &self.this_mod_path;
        let orig_mod_path = &self.orig_mod_path;

        let sources = self
            .sources
            .iter()
            .map(|SourceFile { path_buf, fqn, .. }| {
                format!(
                    "{}<{}>",
                    quote!(#fqn).to_string().replace(' ', ""),
                    path_buf
                        .to_str()
                        .expect("Expected path_buf to render to str")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        write!(
            f,
            "IncludeArgs< this_mod_path = {}, orig_mod_path = {}, items_suffix = {:?}, sources = [ {} ], spec_tree = <{:?}> >",
            quote!(#this_mod_path), quote!(#orig_mod_path), self.items_suffix, sources, self.spec_tree
        )
    }
}

impl Parse for IncludeArgs {
    /// Parse the TokenStream into IncludeArgs struct.
    /// Throws an `proc_macro_error::abort!` error on any inconsistence in macro
    /// parameters.
    /// Utilizes IncludeArgsBuilder and maps its `build()` method errors into
    /// human-readable errors.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut include_args_builder = IncludeArgsBuilder::default();
        include_args_builder.spec_tree(SpecTreeNode::new());
        let mut expr = input
            .parse::<Expr>()
            .map_err(|e| {
                abort_call_site!(format!("Unexpected syntax: {}", e));
            })
            .unwrap();

        Self::parse_call_chain(&mut include_args_builder, &mut expr);

        let args = include_args_builder
            .build()
            .map_err(|e| match e {
                IncludeArgsBuilderError::UninitializedField("this_mod_path") => abort_call_site!(
                    format!("`{}` parameter is required", Self::QUASI_FN_THIS_MOD_PATH),
                ),
                IncludeArgsBuilderError::UninitializedField("orig_mod_path") => abort_call_site!(
                    format!("`{}` parameter is required", Self::QUASI_FN_ORIG_MOD_PATH),
                ),
                IncludeArgsBuilderError::UninitializedField("sources") => {
                    abort_call_site!(format!(
                        "At least one `{}` parameter is required",
                        Self::QUASI_FN_SOURCE
                    ))
                }
                _ => abort_call_site!(format!("Unexpected macro error, please report: {}", e)),
            })
            .unwrap();

        Ok(args)
    }
}

impl IncludeArgs {
    const QUASI_FN_ENUM_SPEC: &'static str = "with_enum";
    const QUASI_FN_ITEMS_SUFFIX: &'static str = "with_suffix";
    const QUASI_FN_ORIG_MOD_PATH: &'static str = "with_original_mod";
    const QUASI_FN_SOURCE: &'static str = "from_source";
    const QUASI_FN_STRUCT_SPEC: &'static str = "with_struct";
    const QUASI_FN_THIS_MOD_PATH: &'static str = "with_this_mod";

    /// Entry point for parsing macro arguments expression.
    /// The arguments expression is a quasi chain of method calls with the
    /// function call as the chain terminator, so the only two expression types
    /// are ExprMethodCall and ExprCall.
    fn parse_call_chain(include_args_builder: &mut IncludeArgsBuilder, call_expr: &mut Expr) {
        match call_expr {
            // chained method-like calls
            Expr::MethodCall(method_call_expr) => {
                Self::build_arguments(
                    include_args_builder,
                    method_call_expr.method.to_string().as_str(),
                    &method_call_expr.method.span(),
                    &mut method_call_expr.args,
                );

                Self::parse_call_chain(include_args_builder, method_call_expr.receiver.as_mut());
            }

            // very first fn-like call
            Expr::Call(fn_call_expr) => {
                if let Expr::Path(fn_path) = fn_call_expr.func.as_ref() {
                    if fn_path.path.segments.len() == 1 {
                        let argument_ident = &fn_path.path.segments.first().unwrap().ident;

                        Self::build_arguments(
                            include_args_builder,
                            format!("{}", quote!(#argument_ident)).as_str(),
                            &fn_call_expr.span(),
                            &mut fn_call_expr.args,
                        );
                    }
                }
            }

            expr_ => abort!(expr_, "Unexpected macro call syntax"),
        }
    }

    /// Function matches on quasi method or function call and calls for mapped
    /// parser function to fill the IncludeArgsBuilder.
    fn build_arguments(
        include_args_builder: &mut IncludeArgsBuilder,
        argument_name: &str,
        expr_span: &Span,
        expr_args: &mut Punctuated<Expr, Token![,]>,
    ) {
        match argument_name {
            Self::QUASI_FN_SOURCE => Self::parse_source(include_args_builder, expr_args, expr_span),
            Self::QUASI_FN_THIS_MOD_PATH => {
                Self::parse_this_mod_path(include_args_builder, expr_args, expr_span)
            }
            Self::QUASI_FN_ORIG_MOD_PATH => {
                Self::parse_orig_mod_path(include_args_builder, expr_args, expr_span)
            }
            Self::QUASI_FN_ITEMS_SUFFIX => {
                Self::parse_items_suffix(include_args_builder, expr_args, expr_span)
            }
            Self::QUASI_FN_STRUCT_SPEC => {
                Self::parse_struct_spec(include_args_builder, expr_args, expr_span)
            }
            Self::QUASI_FN_ENUM_SPEC => {
                Self::parse_enum_spec(include_args_builder, expr_args, expr_span)
            }
            _other => abort!(
                expr_span,
                format!(
                    "Unknown configuration parameter, must be one of: {}",
                    [
                        Self::QUASI_FN_SOURCE,
                        Self::QUASI_FN_THIS_MOD_PATH,
                        Self::QUASI_FN_ORIG_MOD_PATH,
                        Self::QUASI_FN_ITEMS_SUFFIX,
                        Self::QUASI_FN_ENUM_SPEC,
                        Self::QUASI_FN_STRUCT_SPEC,
                    ]
                    .join(", ")
                ),
            ),
        }
    }

    /// Parser for Self::QUASI_FN_SOURCE
    fn parse_source(
        include_args_builder: &mut IncludeArgsBuilder,
        call_args: &mut Punctuated<Expr, Token![,]>,
        expr_span: &Span,
    ) {
        if call_args.len() != 2 {
            abort!(expr_span, "Parameter must have 2 arguments");
        }

        let mut call_args_iter = call_args.iter();

        let fqn = match call_args_iter.next().unwrap() {
            Expr::Path(path_expr) => path_expr.path.clone(),
            expr_ => abort!(
                expr_,
                "Parameter argument must be a module path literal, e.g. `root::submodule`",
            ),
        };

        let (path_buf, ast) = match call_args_iter.next().unwrap()
        {
            Expr::Lit(ExprLit {
                lit: Lit::Str(str_lit),
                ..
            }) => {
                let path_buf = fs::canonicalize(PathBuf::from(str_lit.value()))
                    .map_err(|e| {
                        abort!(
                            str_lit,
                            format!(
                                "Failed to load source code from {:?}: {} (cwd: {:?})",
                                &str_lit.value(),
                                e,
                                std::env::current_dir().expect("Expected current dir to be present")
                            ),
                        )
                    })
                    .unwrap();

                let contents = fs::read_to_string(&path_buf)
                    .map_err(|e| {
                        abort!(
                            str_lit,
                            format!(
                                "Failed to load source code from {:?}: {}",
                                &path_buf.as_path(),
                                e
                            ),
                        )
                    })
                    .unwrap();

                let ast = syn::parse_file(contents.as_str())
                    .map_err(|e| {
                        abort!(
                            str_lit,
                            format!("Failed to parse linked source code as rust file: {}", e),
                        );
                    })
                    .unwrap();


                (path_buf, ast)
            },
            expr_ => abort!(
                expr_,
                "Parameter argument must be a string literal path to the prost-generated rust source code"
            )
        };

        let source = SourceFile { path_buf, ast, fqn };
        match &mut include_args_builder.sources {
            Some(vec) => vec.push(source),
            None => include_args_builder.sources = Some(vec![source]),
        }
    }

    /// Parser for Self::QUASI_FN_THIS_MOD_PATH
    fn parse_this_mod_path(
        include_args_builder: &mut IncludeArgsBuilder,
        call_args: &mut Punctuated<Expr, Token![,]>,
        expr_span: &Span,
    ) {
        if include_args_builder.this_mod_path.is_some() {
            abort!(
                expr_span,
                format!(
                    "Multiple `{}` parameters are not allowed",
                    Self::QUASI_FN_THIS_MOD_PATH
                ),
            )
        }

        if call_args.len() != 1 {
            abort!(expr_span, "Parameter must have 1 argument");
        }

        if let Expr::Path(path_expr) = call_args.first().unwrap() {
            if path_expr.path.segments.first().unwrap().ident == "crate" {
                include_args_builder.this_mod_path(path_expr.path.clone());
                return;
            }
        }

        abort!(
            call_args,
            "Parameter argument must be an absolute module path literal, e.g. `crate::proto`",
        );
    }

    /// Parser for Self::QUASI_FN_ORIG_MOD_PATH
    fn parse_orig_mod_path(
        include_args_builder: &mut IncludeArgsBuilder,
        call_args: &mut Punctuated<Expr, Token![,]>,
        expr_span: &Span,
    ) {
        if include_args_builder.orig_mod_path.is_some() {
            abort!(
                expr_span,
                format!(
                    "Multiple `{}` parameters are not allowed",
                    Self::QUASI_FN_ORIG_MOD_PATH
                ),
            )
        }

        if call_args.len() != 1 {
            abort!(expr_span, "Parameter must have 1 argument");
        }

        if let Expr::Path(path_expr) = call_args.first().unwrap() {
            if path_expr.path.segments.first().unwrap().ident == "crate" {
                include_args_builder.orig_mod_path(path_expr.path.clone());
                return;
            }
        }
        abort!(
            call_args,
            "Parameter argument must be an absolute module path literal, e.g. `crate::proto`",
        )
    }

    /// Parser for Self::QUASI_FN_ITEMS_SUFFIX
    fn parse_items_suffix(
        include_args_builder: &mut IncludeArgsBuilder,
        call_args: &mut Punctuated<Expr, Token![,]>,
        expr_span: &Span,
    ) {
        if include_args_builder.items_suffix.is_some() {
            abort!(
                expr_span,
                format!(
                    "Multiple `{}` parameters are not allowed",
                    Self::QUASI_FN_ITEMS_SUFFIX
                ),
            )
        }

        if call_args.len() != 1 {
            abort!(expr_span, "Parameter must have 1 argument");
        }

        if let Expr::Path(path_expr) = call_args.first().unwrap() {
            include_args_builder
                .items_suffix(Some(path_expr.path.segments.first().unwrap().ident.clone()));
            return;
        }

        abort!(
            call_args,
            "Parameter argument must be an ident literal, e.g. `Suffix`",
        )
    }

    /// Parser for Self::QUASI_FN_STRUCT_SPEC
    fn parse_struct_spec(
        include_args_builder: &mut IncludeArgsBuilder,
        call_args: &mut Punctuated<Expr, Token![,]>,
        expr_span: &Span,
    ) {
        if call_args.len() != 2 {
            abort!(expr_span, "Parameter must have 2 arguments");
        }

        let mut call_args_iter = call_args.iter();

        let fqn = match call_args_iter.next().unwrap() {
            Expr::Path(path_expr) => path_expr.path.clone(),
            expr_ => {
                abort!(
                    expr_,
                    "Argument must be a path literal relative to `with_original_mod` argument, e.g. `root::Something`"
                );
            }
        };

        let fields = match call_args_iter.next().unwrap() {
            Expr::Array(array_expr) => array_expr
                .elems
                .iter()
                .map(|field_expr| {
                    if let Expr::Path(field_path_expr) = field_expr {
                        if field_path_expr.path.segments.len() == 1 {
                            return field_path_expr.path.segments.first().unwrap().ident.clone();
                        }
                    }
                    abort!(
                        field_expr,
                        "Field must be a single ident literal, e.g. `field1`",
                    );
                })
                .collect::<Vec<Ident>>(),
            expr_ => {
                abort!(
                    expr_,
                    "Argument must be an array of fields ident literals, e.g. `[field1, field2]`",
                );
            }
        };

        let struct_spec = SpecTreeLeaf::StructSpec { fqn, fields };

        include_args_builder
            .spec_tree
            .as_mut()
            .expect("Expected spec_tree to be Some")
            .push(struct_spec);
    }

    /// Parser for Self::QUASI_FN_ENUM_SPEC
    fn parse_enum_spec(
        include_args_builder: &mut IncludeArgsBuilder,
        call_args: &mut Punctuated<Expr, Token![,]>,
        expr_span: &Span,
    ) {
        if call_args.len() != 1 {
            abort!(expr_span, "Parameter must have 1 argument");
        }

        let mut call_args_iter = call_args.iter();

        let fqn = match call_args_iter.next().unwrap() {
            Expr::Path(path_expr) => path_expr.path.clone(),
            expr_ => {
                abort!(
                    expr_,
                    "Argument must be a path literal relative to `with_original_mod` argument, e.g. `root::Something`"
                );
            }
        };

        let enum_spec = SpecTreeLeaf::EnumSpec { fqn };

        include_args_builder
            .spec_tree
            .as_mut()
            .expect("Expected spec_tree to be Some")
            .push(enum_spec);
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct SourceFile {
    pub path_buf: PathBuf,
    pub ast: File,
    pub fqn: Path,
}

/// Contains data structures and functions to collect and operate on a tree
/// of specificated structs and enums.
pub(crate) mod spec_tree {
    use std::collections::hash_map::Entry;
    use std::collections::HashMap;
    use std::iter::Peekable;

    use proc_macro2::Span;
    use proc_macro_error::abort;
    use syn::punctuated::Punctuated;
    use syn::Ident;
    use syn::Path;
    use syn::PathSegment;

    #[derive(Clone, Debug)]
    #[allow(dead_code)]
    // Probably, the better way will be to separate SpecTreeNode variants into
    // separate enum, and leave the common `fqn` field present in a parent struct,
    // but this will complicate the consequent processing, as `fqn` is also used
    // when comparing the struct specs with struct AST from the parsed files.
    pub(crate) enum SpecTreeLeaf {
        StructSpec { fqn: Path, fields: Vec<Ident> },
        EnumSpec { fqn: Path },
    }

    impl SpecTreeLeaf {
        pub fn fqn_ref(&self) -> &Path {
            match self {
                SpecTreeLeaf::StructSpec { ref fqn, .. } => fqn,
                SpecTreeLeaf::EnumSpec { ref fqn } => fqn,
            }
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) struct SpecTreeNode {
        pub fqn: Path,
        pub nodes: HashMap<String, SpecTreeNode>,
        pub leafs: HashMap<String, SpecTreeLeaf>,
    }

    impl SpecTreeNode {
        /// Returns a new empty SpecTree
        pub fn new() -> Self {
            SpecTreeNode {
                fqn: Path {
                    leading_colon: None,
                    segments: Punctuated::new(),
                },
                nodes: HashMap::new(),
                leafs: HashMap::new(),
            }
        }

        /// Returns a mutable reference to a child SpecTreeNode with `ident` key.
        /// If no such node if present, it is created.
        pub fn child(&mut self, ident: String) -> &mut SpecTreeNode {
            match self.nodes.entry(ident.clone()) {
                Entry::Occupied(node) => node.into_mut(),
                Entry::Vacant(vacant_node) => {
                    let mut child = SpecTreeNode {
                        fqn: self.fqn.clone(),
                        nodes: HashMap::new(),
                        leafs: HashMap::new(),
                    };
                    child.fqn.segments.push(PathSegment {
                        ident: Ident::new(ident.as_ref(), Span::call_site()),
                        arguments: syn::PathArguments::None,
                    });
                    vacant_node.insert(child)
                }
            }
        }

        /// Pushes the SpecTreeNode into SpecTree, creating necessary module
        /// specs along the path; path is fully-qualified struct or enum name,
        /// specified from the root module (see `with_orig_mod` argument), e.g.
        /// `root::child::StructA`
        pub fn push(&mut self, spec_tree_leaf: SpecTreeLeaf) -> &mut Self {
            let mut path_vec: Vec<_> = spec_tree_leaf
                .fqn_ref()
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            let leaf_name = path_vec.pop().expect("Expected leaf fqn to be non-empty");
            let mut path = path_vec.into_iter().peekable();

            push_recursive(self, &mut path, leaf_name, spec_tree_leaf);

            return self;

            fn push_recursive(
                node: &mut SpecTreeNode,
                path: &mut Peekable<impl Iterator<Item = String>>,
                leaf_name: String,
                spec_tree_leaf: SpecTreeLeaf,
            ) {
                if path.peek().is_none() {
                    if node.leafs.contains_key(&leaf_name) {
                        abort!(spec_tree_leaf.fqn_ref(), "Duplicate specs are not allowed")
                    }
                    node.leafs.insert(leaf_name, spec_tree_leaf);
                } else {
                    let ident = path.next().unwrap();
                    push_recursive(node.child(ident), path, leaf_name, spec_tree_leaf);
                }
            }
        }

        pub fn get_leaf(&self, path: impl IntoIterator<Item = String>) -> Option<&SpecTreeLeaf> {
            let mut path_vec: Vec<String> = path.into_iter().collect();
            let leaf_name = path_vec.pop()?;
            let mut path = path_vec.into_iter().peekable();

            return get_recursive(self, &mut path, leaf_name);

            fn get_recursive<'a>(
                node: &'a SpecTreeNode,
                path_iter: &mut Peekable<impl Iterator<Item = String>>,
                leaf_name: String,
            ) -> Option<&'a SpecTreeLeaf> {
                if let Some(ident) = path_iter.next() {
                    if node.nodes.contains_key(&ident) {
                        get_recursive(node.nodes.get(&ident).unwrap(), path_iter, leaf_name)
                    } else {
                        None
                    }
                } else {
                    node.leafs
                        .contains_key(&leaf_name)
                        .then(|| node.leafs.get(&leaf_name).unwrap())
                }
            }
        }
    }

    #[cfg(test)]
    pub(crate) mod tests {
        use syn::parse_str;

        use super::*;

        #[test]
        fn push_single() {
            let mut tree = SpecTreeNode::new();
            let node = SpecTreeLeaf::StructSpec {
                fqn: parse_str("root::child::StructA").unwrap(),
                fields: Vec::new(),
            };
            tree.push(node);

            assert!(tree
                .get_leaf(vec![
                    "root".to_string(),
                    "child".to_string(),
                    "StructA".to_string()
                ])
                .is_some());
        }

        #[test]
        fn push_multiple() {
            let mut tree = SpecTreeNode::new();
            tree.push(SpecTreeLeaf::StructSpec {
                fqn: parse_str("root::child::StructA").unwrap(),
                fields: Vec::new(),
            })
            .push(SpecTreeLeaf::StructSpec {
                fqn: parse_str("root::child::EnumA").unwrap(),
                fields: Vec::new(),
            });

            assert!(tree
                .get_leaf(vec![
                    "root".to_string(),
                    "child".to_string(),
                    "StructA".to_string()
                ])
                .is_some());
            assert!(tree
                .get_leaf(vec![
                    "root".to_string(),
                    "child".to_string(),
                    "EnumA".to_string()
                ])
                .is_some());
        }

        #[test]
        fn push_conflict() {
            let mut tree = SpecTreeNode::new();
            tree.push(SpecTreeLeaf::StructSpec {
                fqn: parse_str("root::child::StructA").unwrap(),
                fields: Vec::new(),
            })
            .push(SpecTreeLeaf::StructSpec {
                fqn: parse_str("root::child::StructA::Whatever").unwrap(),
                fields: Vec::new(),
            });

            assert!(tree
                .get_leaf(vec![
                    "root".to_string(),
                    "child".to_string(),
                    "StructA".to_string()
                ])
                .is_some());
            assert!(tree
                .get_leaf(vec![
                    "root".to_string(),
                    "child".to_string(),
                    "StructA".to_string(),
                    "Whatever".to_string()
                ])
                .is_some());
        }

        #[test]
        fn get_non_existent() {
            let tree = SpecTreeNode::new();

            assert!(tree
                .get_leaf(vec![
                    "root".to_string(),
                    "child".to_string(),
                    "EnumA".to_string()
                ])
                .is_none());
        }

        #[test]
        fn get_module() {
            let mut tree = SpecTreeNode::new();
            let node = SpecTreeLeaf::StructSpec {
                fqn: parse_str("root::child::StructA").unwrap(),
                fields: Vec::new(),
            };
            tree.push(node);

            assert!(tree
                .get_leaf(vec!["root".to_string(), "child".to_string()])
                .is_none());
        }

        #[test]
        #[should_panic]
        fn push_diplicate_panic() {
            let mut tree = SpecTreeNode::new();
            tree.push(SpecTreeLeaf::StructSpec {
                fqn: parse_str("root::child::StructA").unwrap(),
                fields: Vec::new(),
            })
            .push(SpecTreeLeaf::StructSpec {
                fqn: parse_str("root::child::StructA").unwrap(),
                fields: Vec::new(),
            });
        }
    }
}
