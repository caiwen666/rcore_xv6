use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Expr, ItemFn, LitInt, LitStr, ReturnType, Token, parse::Parse, parse::ParseStream,
    parse_macro_input, punctuated::Punctuated,
};

struct SyscallAttr {
    name: LitStr,
    id: LitInt,
}

impl Parse for SyscallAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut id = None;

        let args = Punctuated::<Expr, Token![,]>::parse_terminated(input)?;
        for arg in args {
            let Expr::Assign(assign) = arg else {
                return Err(syn::Error::new_spanned(
                    arg,
                    "expected `name = ...` or `id = ...`",
                ));
            };
            let Expr::Path(path) = *assign.left else {
                return Err(syn::Error::new_spanned(assign.left, "expected identifier"));
            };
            let Some(ident) = path.path.get_ident() else {
                return Err(syn::Error::new_spanned(path, "expected identifier"));
            };

            match ident.to_string().as_str() {
                "name" => name = Some(parse_expr(*assign.right)?),
                "id" => id = Some(parse_expr(*assign.right)?),
                other => {
                    return Err(syn::Error::new_spanned(
                        ident,
                        format!("unknown key `{other}`, expected `name` or `id`"),
                    ));
                }
            }
        }

        Ok(Self {
            name: name.ok_or_else(|| input.error("missing `name = \"...\"`"))?,
            id: id.ok_or_else(|| input.error("missing `id = ...`"))?,
        })
    }
}

fn parse_expr<T: Parse>(expr: Expr) -> syn::Result<T> {
    syn::parse2(expr.into_token_stream())
}

#[proc_macro_attribute]
pub fn syscall(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as SyscallAttr);
    let func = parse_macro_input!(item as ItemFn);

    let sig = &func.sig;
    if sig.inputs.len() != 1
        || matches!(sig.output, ReturnType::Default)
        || !sig.generics.params.is_empty()
    {
        return syn::Error::new_spanned(
            &func.sig,
            "syscall handler must be `fn(args: [usize; 6]) -> isize`",
        )
        .to_compile_error()
        .into();
    }

    let fn_name = &sig.ident;
    let handle_name = format_ident!("HANDLE_{}", fn_name.to_string().to_uppercase());

    let name = &attr.name;
    let id = &attr.id;

    let expanded = quote! {
        #func

        #[unsafe(link_section = ".syscall_table")]
        #[used]
        pub static #handle_name: crate::exception::syscall::SyscallHandle =
            crate::exception::syscall::SyscallHandle {
                id: #id,
                name: #name,
                handle: #fn_name,
            };
    };

    expanded.into()
}
