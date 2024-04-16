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

/// The `include!`` macro arguments.
/// Contains all needed information for AST traversal.
#[derive(Builder, Clone)]
pub struct IncludeArgs {
    pub this_mod_path: Path,
    pub orig_mod_path: Path,
    #[builder(default = "None")]
    pub items_suffix: Option<Ident>,
    pub sources: Vec<SourceFile>,
    #[builder(default = "Vec::new()")]
    pub struct_specs: Vec<StructSpec>,
    #[builder(default = "Vec::new()")]
    pub enum_specs: Vec<EnumSpec>,
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
                    path_buf.to_str().unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let struct_specs = self
            .struct_specs
            .iter()
            .map(|StructSpec { fqn, fields }| {
                format!(
                    "{}<({})>",
                    quote!(#fqn).to_string().replace(' ', ""),
                    fields
                        .iter()
                        .map(|ident| { quote!(#ident).to_string() })
                        .collect::<Vec<String>>()
                        .join(",")
                )
            })
            .collect::<Vec<String>>()
            .join(", ");

        let enum_specs = self
            .enum_specs
            .iter()
            .map(|EnumSpec { fqn }| quote!(#fqn).to_string().replace(' ', ""))
            .collect::<Vec<String>>()
            .join(", ");

        write!(
            f,
            "IncludeArgs< this_mod_path = {}, orig_mod_path = {}, items_suffix = {:?}, sources = [ {} ], struct_specs = [ {} ], enum_specs = [ {} ] >",
            quote!(#this_mod_path), quote!(#orig_mod_path), self.items_suffix, sources, struct_specs, enum_specs
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
        let mut expr = input
            .parse::<Expr>()
            .map_err(|e| {
                abort_call_site!(format!("Unexpected syntax: {}", e));
            })
            .unwrap();

        Self::parse_call_chain(&mut include_args_builder, &mut expr);

        if include_args_builder.struct_specs.is_none() && include_args_builder.enum_specs.is_none()
        {
            abort_call_site!(format!(
                "At least one `{}` or `{}` parameter is required",
                Self::QUASI_FN_ENUM_SPEC,
                Self::QUASI_FN_STRUCT_SPEC
            ));
        }

        Ok(include_args_builder
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
            .unwrap())
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

        let (path_buf, ast, items) = match call_args_iter.next().unwrap()
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
                                std::env::current_dir().unwrap()
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

                let items = traverse::collect_structs_and_enums(&ast);

                (path_buf, ast, items)
            },
            expr_ => abort!(
                expr_,
                "Parameter argument must be a string literal path to the prost-generated rust source code"
            )
        };

        let source = SourceFile {
            path_buf,
            ast,
            fqn,
            items,
        };
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

        let struct_spec = StructSpec { fqn, fields };

        match &mut include_args_builder.struct_specs {
            Some(vec) => vec.push(struct_spec),
            None => include_args_builder.struct_specs = Some(vec![struct_spec]),
        }
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

        let enum_spec = EnumSpec { fqn };

        match &mut include_args_builder.enum_specs {
            Some(vec) => vec.push(enum_spec),
            None => include_args_builder.enum_specs = Some(vec![enum_spec]),
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct SourceFile {
    pub path_buf: PathBuf,
    pub ast: File,
    pub fqn: Path,
    pub items: Vec<SourceFileItem>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) enum SourceFileItem {
    Struct(String),
    Enum(String),
}

#[derive(Clone)]
pub(crate) struct StructSpec {
    pub fqn: Path,
    pub fields: Vec<Ident>,
}

#[derive(Clone)]
pub(crate) struct EnumSpec {
    pub fqn: Path,
}

pub(crate) mod traverse {
    use syn::visit;
    use syn::visit::Visit;
    use syn::ItemEnum;
    use syn::ItemMod;
    use syn::ItemStruct;

    /// Custom visitor to collect paths to all structs and enums.
    struct CollectPaths {
        current_path: Vec<String>,
        paths: Vec<super::SourceFileItem>,
    }

    impl CollectPaths {
        fn new() -> Self {
            CollectPaths {
                current_path: Vec::new(),
                paths: Vec::new(),
            }
        }
    }

    impl<'ast> Visit<'ast> for CollectPaths {
        fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
            let ident = item.ident.to_string();
            self.paths.push(super::SourceFileItem::Struct(
                self.current_path
                    .iter()
                    .chain([ident].iter())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("::"),
            ));
            visit::visit_item_struct(self, item);
        }

        fn visit_item_enum(&mut self, item: &'ast ItemEnum) {
            let ident = item.ident.to_string();
            self.paths.push(super::SourceFileItem::Enum(
                self.current_path
                    .iter()
                    .chain([ident].iter())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("::"),
            ));
            visit::visit_item_enum(self, item);
        }

        fn visit_item_mod(&mut self, item: &'ast ItemMod) {
            if let Some(content) = &item.content {
                self.current_path.push(item.ident.to_string());
                for item in &content.1 {
                    self.visit_item(item);
                }
                self.current_path.pop();
            }
            visit::visit_item_mod(self, item);
        }
    }

    pub(crate) fn collect_structs_and_enums(file: &syn::File) -> Vec<super::SourceFileItem> {
        let mut collector = CollectPaths::new();
        collector.visit_file(file);
        collector.paths
    }
}
