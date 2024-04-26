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

use self::spec_tree::SpecTree;
use self::spec_tree::SpecTreeLeaf;

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct SourceFile {
    pub path_span: Span,
    pub path_buf: PathBuf,
    pub fqn: Path,
    pub ast: File,
}

/// The `include!` macro arguments.
/// Contains all needed information for AST traversal.
#[derive(Builder, Clone)]
pub struct Config {
    pub this_mod_path: Path,
    pub orig_mod_path: Path,
    #[builder(default = "None")]
    pub items_suffix: Option<Ident>,
    pub source: SourceFile,
    pub spec_tree: SpecTree,
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let this_mod_path = &self.this_mod_path;
        let orig_mod_path = &self.orig_mod_path;

        write!(
            f,
            "IncludeArgs< this_mod_path = {}, orig_mod_path = {}, items_suffix = {:?}, spec_tree = <{:?}> >",
            quote!(#this_mod_path), quote!(#orig_mod_path), self.items_suffix, self.spec_tree
        )
    }
}

impl Parse for Config {
    /// Parse the TokenStream into IncludeArgs struct.
    /// Throws an `proc_macro_error::abort!` error on any inconsistence in macro
    /// parameters.
    /// Utilizes ConfigBuilder and maps its `build()` method errors into
    /// human-readable errors.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut config_builder = ConfigBuilder::default();
        config_builder.spec_tree(SpecTree::new());
        let mut expr = input
            .parse::<Expr>()
            .map_err(|e| {
                abort_call_site!(format!("Unexpected syntax: {}", e));
            })
            .unwrap();

        Self::parse_call_chain(&mut config_builder, &mut expr);

        let config = config_builder
            .build()
            .map_err(|e| match e {
                ConfigBuilderError::UninitializedField("this_mod_path") => abort_call_site!(
                    format!("`{}` parameter is required", Self::QUASI_FN_THIS_MOD_PATH),
                ),
                ConfigBuilderError::UninitializedField("orig_mod_path") => abort_call_site!(
                    format!("`{}` parameter is required", Self::QUASI_FN_ORIG_MOD_PATH),
                ),
                ConfigBuilderError::UninitializedField("sources") => {
                    abort_call_site!(format!(
                        "At least one `{}` parameter is required",
                        Self::QUASI_FN_SOURCE
                    ))
                }
                _ => abort_call_site!(format!("Unexpected macro error, please report: {}", e)),
            })
            .unwrap();

        Ok(config)
    }
}

impl Config {
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
    fn parse_call_chain(config_builder: &mut ConfigBuilder, call_expr: &mut Expr) {
        match call_expr {
            // chained method-like calls
            Expr::MethodCall(method_call_expr) => {
                Self::build_arguments(
                    config_builder,
                    method_call_expr.method.to_string().as_str(),
                    &method_call_expr.method.span(),
                    &mut method_call_expr.args,
                );

                Self::parse_call_chain(config_builder, method_call_expr.receiver.as_mut());
            }

            // very first fn-like call
            Expr::Call(fn_call_expr) => {
                if let Expr::Path(fn_path) = fn_call_expr.func.as_ref() {
                    if fn_path.path.segments.len() == 1 {
                        let argument_ident = &fn_path.path.segments.first().unwrap().ident;

                        Self::build_arguments(
                            config_builder,
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

    /// Function matches on quasi-method or -function call and calls for mapped
    /// parser function to fill the ConfigBuilder.
    fn build_arguments(
        config_builder: &mut ConfigBuilder,
        argument_name: &str,
        expr_span: &Span,
        expr_args: &mut Punctuated<Expr, Token![,]>,
    ) {
        match argument_name {
            Self::QUASI_FN_SOURCE => Self::parse_source(config_builder, expr_args, expr_span),
            Self::QUASI_FN_THIS_MOD_PATH => {
                Self::parse_this_mod_path(config_builder, expr_args, expr_span)
            }
            Self::QUASI_FN_ORIG_MOD_PATH => {
                Self::parse_orig_mod_path(config_builder, expr_args, expr_span)
            }
            Self::QUASI_FN_ITEMS_SUFFIX => {
                Self::parse_items_suffix(config_builder, expr_args, expr_span)
            }
            Self::QUASI_FN_STRUCT_SPEC => {
                Self::parse_struct_spec(config_builder, expr_args, expr_span)
            }
            Self::QUASI_FN_ENUM_SPEC => Self::parse_enum_spec(config_builder, expr_args, expr_span),
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
        config_builder: &mut ConfigBuilder,
        call_args: &mut Punctuated<Expr, Token![,]>,
        expr_span: &Span,
    ) {
        if config_builder.source.is_some() {
            abort!(
                expr_span,
                format!(
                    "Multiple `{}` parameters are not allowed",
                    Self::QUASI_FN_SOURCE
                ),
            )
        }

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

        let (path_span, path_buf, ast) = match call_args_iter.next().unwrap()
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


                (str_lit.span(), path_buf, ast)
            },
            expr_ => abort!(
                expr_,
                "Parameter argument must be a string literal path to the prost-generated rust source code"
            )
        };

        let source = SourceFile {
            path_span,
            path_buf,
            fqn,
            ast,
        };
        config_builder.source(source);
    }

    /// Parser for Self::QUASI_FN_THIS_MOD_PATH
    fn parse_this_mod_path(
        config_builder: &mut ConfigBuilder,
        call_args: &mut Punctuated<Expr, Token![,]>,
        expr_span: &Span,
    ) {
        if config_builder.this_mod_path.is_some() {
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
                config_builder.this_mod_path(path_expr.path.clone());
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
        config_builder: &mut ConfigBuilder,
        call_args: &mut Punctuated<Expr, Token![,]>,
        expr_span: &Span,
    ) {
        if config_builder.orig_mod_path.is_some() {
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
                config_builder.orig_mod_path(path_expr.path.clone());
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
        config_builder: &mut ConfigBuilder,
        call_args: &mut Punctuated<Expr, Token![,]>,
        expr_span: &Span,
    ) {
        if config_builder.items_suffix.is_some() {
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
            config_builder
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
        config_builder: &mut ConfigBuilder,
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

        let struct_spec = SpecTreeLeaf::new_struct_spec(fqn, fields);

        config_builder
            .spec_tree
            .as_mut()
            .expect("Expected spec_tree to be Some")
            .push(struct_spec);
    }

    /// Parser for Self::QUASI_FN_ENUM_SPEC
    fn parse_enum_spec(
        config_builder: &mut ConfigBuilder,
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

        let enum_spec = SpecTreeLeaf::new_enum_spec(fqn);

        config_builder
            .spec_tree
            .as_mut()
            .expect("Expected spec_tree to be Some")
            .push(enum_spec);
    }
}

impl Config {
    pub fn orig_item_typepath<I: IntoIterator<Item = String>>(&self, ident_path: I) -> Path {
        let path = self.orig_mod_path.clone();
        self.item_typepath(path, ident_path)
    }

    pub fn this_item_typepath<I: IntoIterator<Item = String>>(&self, ident_path: I) -> Path {
        let path = self.this_mod_path.clone();
        self.item_typepath(path, ident_path)
    }

    pub fn error_typepath<'a>(&self) -> Path {
        let path = self.this_mod_path.clone();
        self.item_typepath(path, vec!["Error".to_string()])
    }

    fn item_typepath<I: IntoIterator<Item = String>>(
        &self,
        mut mod_path: Path,
        ident_path: I,
    ) -> Path {
        for ident in ident_path {
            mod_path.segments.push(syn::PathSegment {
                ident: Ident::new(ident.as_str(), Span::call_site()),
                arguments: syn::PathArguments::None,
            });
        }
        mod_path
    }
}

/// Contains config structures and functions to collect and operate on a tree
/// of specificated structs and enums.
pub(crate) mod spec_tree {
    use std::collections::hash_map::Entry;
    use std::collections::HashMap;

    use proc_macro2::Span;
    use proc_macro_error::abort;
    use syn::punctuated::Punctuated;
    use syn::Ident;
    use syn::Path;
    use syn::PathSegment;

    #[derive(Clone, Debug)]
    pub(crate) struct StructSpec {
        fqn: Path,
        fields: Vec<Ident>,
    }

    impl StructSpec {
        pub fn fields_map(&self) -> HashMap<String, &Ident> {
            let mut hashmap = HashMap::new();
            for field in self.fields.iter() {
                hashmap.insert(field.to_string(), field);
            }
            hashmap
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) struct EnumSpec {
        fqn: Path,
    }

    #[derive(Clone, Debug)]
    #[allow(dead_code)]
    pub(crate) enum SpecTreeLeaf {
        Struct(StructSpec),
        Enum(EnumSpec),
    }

    impl SpecTreeLeaf {
        pub fn new_struct_spec(fqn: Path, fields: Vec<Ident>) -> Self {
            SpecTreeLeaf::Struct(StructSpec { fqn, fields })
        }

        pub fn new_enum_spec(fqn: Path) -> Self {
            SpecTreeLeaf::Enum(EnumSpec { fqn })
        }

        pub fn fqn_ref(&self) -> &Path {
            match self {
                SpecTreeLeaf::Struct(StructSpec { ref fqn, .. }) => fqn,
                SpecTreeLeaf::Enum(EnumSpec { ref fqn, .. }) => fqn,
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

        /// Returns a mutable reference to a child SpecTreeNode under
        /// `ident` key. If no such node if present, it is created.
        pub fn child<'node, 'leaf>(&'node mut self, ident: String) -> &'leaf mut SpecTreeNode
        where
            'node: 'leaf,
        {
            if !self.nodes.contains_key(&ident) {
                let mut fqn = self.fqn.clone();
                fqn.segments.push(PathSegment {
                    ident: Ident::new(ident.as_ref(), Span::call_site()),
                    arguments: syn::PathArguments::None,
                });
                let child = SpecTreeNode {
                    fqn,
                    nodes: HashMap::new(),
                    leafs: HashMap::new(),
                };
                self.nodes.insert(ident.clone(), child);
            }
            self.nodes.get_mut(&ident).unwrap()
        }
    }

    #[derive(Clone, Debug)]
    pub(crate) struct SpecTree {
        inner: SpecTreeNode,
    }

    impl SpecTree {
        pub fn new() -> Self {
            SpecTree {
                inner: SpecTreeNode::new(),
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

            let mut current_node = &mut self.inner;

            while let Some(ident) = path.next() {
                current_node = current_node.child(ident);
            }

            match current_node.leafs.entry(leaf_name) {
                Entry::Occupied(_leaf) => {
                    abort!(spec_tree_leaf.fqn_ref(), "Duplicate specs are not allowed");
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(spec_tree_leaf);
                }
            }
            return self;
        }

        pub fn get_leaf<'tree, 'path, 'leaf>(
            &'tree self,
            path: impl IntoIterator<Item = &'path String>,
        ) -> Option<&'leaf SpecTreeLeaf>
        where
            'tree: 'leaf,
        {
            let mut path_vec: Vec<_> = path.into_iter().collect();
            let leaf_name = path_vec.pop()?;
            let mut path = path_vec.into_iter().peekable();

            let mut current_node = &self.inner;

            while let Some(ident) = path.next() {
                current_node = current_node.nodes.get(ident)?;
            }

            current_node.leafs.get(leaf_name)
        }

        // pub fn get_leaf_mut<'tree, 'path, 'leaf>(
        //     &'tree mut self,
        //     path: impl IntoIterator<Item = &'path String>,
        // ) -> Option<&'leaf mut SpecTreeLeaf>
        // where
        //     'tree: 'leaf,
        // {
        //     let mut path_vec: Vec<_> = path.into_iter().collect();
        //     let leaf_name = path_vec.pop()?;
        //     let mut path = path_vec.into_iter().peekable();

        //     let mut current_node = &mut self.inner;

        //     while let Some(ident) = path.next() {
        //         current_node = current_node.nodes.get_mut(ident)?;
        //     }

        //     current_node.leafs.get_mut(leaf_name)
        // }

        // pub fn contains_leaf<'path, I: IntoIterator<Item = &'path String>>(&self, path: I) -> bool {
        //     self.get_leaf(path).is_some()
        // }
    }

    #[cfg(test)]
    pub(crate) mod tests {
        use syn::parse_str;

        use super::*;

        #[test]
        fn push_single() {
            let mut tree = SpecTree::new();
            tree.push(SpecTreeLeaf::new_struct_spec(
                parse_str("root::child::StructA").unwrap(),
                Vec::new(),
            ));

            assert!(tree
                .get_leaf(&vec![
                    "root".to_string(),
                    "child".to_string(),
                    "StructA".to_string(),
                ])
                .is_some());
        }

        #[test]
        fn push_multiple() {
            let mut tree = SpecTree::new();
            tree.push(SpecTreeLeaf::new_struct_spec(
                parse_str("root::child::StructA").unwrap(),
                Vec::new(),
            ))
            .push(SpecTreeLeaf::new_enum_spec(
                parse_str("root::child::EnumA").unwrap(),
            ));

            assert!(tree
                .get_leaf(&vec![
                    "root".to_string(),
                    "child".to_string(),
                    "StructA".to_string(),
                ])
                .is_some());

            assert!(tree
                .get_leaf(&vec![
                    "root".to_string(),
                    "child".to_string(),
                    "EnumA".to_string()
                ])
                .is_some());
        }

        #[test]
        // should panic?
        fn push_conflict() {
            let mut tree = SpecTree::new();
            tree.push(SpecTreeLeaf::new_struct_spec(
                parse_str("root::child::StructA").unwrap(),
                Vec::new(),
            ))
            .push(SpecTreeLeaf::new_struct_spec(
                parse_str("root::child::StructA::Whatever").unwrap(),
                Vec::new(),
            ));

            assert!(tree
                .get_leaf(&vec![
                    "root".to_string(),
                    "child".to_string(),
                    "StructA".to_string(),
                ])
                .is_some());

            assert!(tree
                .get_leaf(&vec![
                    "root".to_string(),
                    "child".to_string(),
                    "StructA".to_string(),
                    "Whatever".to_string(),
                ])
                .is_some());
        }

        #[test]
        fn get_non_existent() {
            let tree = SpecTree::new();

            assert!(tree
                .get_leaf(&vec![
                    "root".to_string(),
                    "child".to_string(),
                    "EnumA".to_string()
                ])
                .is_none());
        }

        #[test]
        fn get_module() {
            let mut tree = SpecTree::new();
            tree.push(SpecTreeLeaf::new_struct_spec(
                parse_str("root::child::StructA").unwrap(),
                Vec::new(),
            ));

            assert!(tree
                .get_leaf(&vec!["root".to_string(), "child".to_string()])
                .is_none());
        }

        #[test]
        #[should_panic]
        fn push_diplicate_panic() {
            let mut tree = SpecTree::new();
            tree.push(SpecTreeLeaf::new_struct_spec(
                parse_str("root::child::StructA").unwrap(),
                Vec::new(),
            ))
            .push(SpecTreeLeaf::new_struct_spec(
                parse_str("root::child::StructA").unwrap(),
                Vec::new(),
            ));
        }
    }
}
