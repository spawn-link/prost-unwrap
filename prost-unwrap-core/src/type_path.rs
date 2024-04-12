use proc_macro2::Span;
use proc_macro_error::abort;
use syn::punctuated::Punctuated;
use syn::Ident;
use syn::Path;
use syn::PathArguments;
use syn::PathSegment;
use syn::Token;
use syn::Type;
use syn::TypePath;

pub fn maybe_unwrap_option_type(ty: &Type) -> &Type {
    if let Type::Path(ty_path) = ty {
        if let Some(last_segment) = ty_path.path.segments.last() {
            if let PathArguments::AngleBracketed(angle_bracketed_args) = &last_segment.arguments {
                if let Some(syn::GenericArgument::Type(inner_ty)) =
                    angle_bracketed_args.args.first()
                {
                    return inner_ty;
                }
            }
        }
    }
    abort!(ty, "cannot unwrap the type path");
}

pub fn is_std_option_type(ty: &Type) -> bool {
    if let Type::Path(ty_path) = ty {
        return ty_path.path.segments.last().map_or(false, |segment| {
            segment.ident == "Option" && !segment.arguments.is_empty()
        });
    }
    false
}

pub fn is_std_vec_type(ty: &Type) -> bool {
    if let Type::Path(ty_path) = ty {
        return ty_path.path.segments.last().map_or(false, |segment| {
            segment.ident == "Vec" && !segment.arguments.is_empty()
        });
    }
    false
}

pub fn error_typepath(mod_stack: &mut [String]) -> TypePath {
    let super_segments = std::iter::repeat(PathSegment {
        ident: Ident::new("super", Span::call_site()),
        arguments: PathArguments::None,
    })
    .take(mod_stack.len());

    let mut segments: Punctuated<PathSegment, Token![::]> = Punctuated::new();
    segments.extend(super_segments);
    segments.push(PathSegment {
        ident: Ident::new("Error", Span::call_site()),
        arguments: PathArguments::None,
    });

    // Construct the TypePath
    TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    }
}
