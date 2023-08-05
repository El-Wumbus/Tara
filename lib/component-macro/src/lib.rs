use convert_case::{Case, Casing};
use proc_macro::TokenStream as CompilerTokenStream;
use proc_macro2::{Ident, TokenStream};
use quote::TokenStreamExt;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, ItemFn,
};

#[proc_macro_attribute]
pub fn component(args: CompilerTokenStream, tokens: CompilerTokenStream) -> CompilerTokenStream {
    let args = parse_macro_input!(args as ComponentArgs);
    let function = parse_macro_input!(tokens as ItemFn);
    if function.sig.asyncness.is_none() {
        panic!("Function isn't an asyncronous one!");
    }

    let cleanup = {
        let mut cleanup = TokenStream::new();
        if let Some(ident) = args.cleanup_ident {
            cleanup.append_all(quote::quote! {
                #[inline]
                async fn cleanup(&self, args: (String, Arc<Http>, Arc<Cache>)) -> ::anyhow::Result<()> {
                    #ident(args).await
                }
            })
        }
        cleanup
    };

    let ident = function.sig.ident;
    let inputs = function.sig.inputs;
    let vis = function.vis;
    let ret = function.sig.output;
    let statements = &function.block.stmts;
    let struct_ident = Ident::new(
        &format!("{}_component", ident).to_case(Case::UpperCamel),
        ident.span(),
    );
    let t = quote::quote! {
        #vis struct #struct_ident;

        #[::async_trait::async_trait]
        impl Component for #struct_ident {
            async fn run(&self, #inputs) #ret {
                #(#statements)*
            }

            #cleanup
        }

        #[allow(non_upper_case_globals)]
        #vis const #ident:#struct_ident = #struct_ident;
    };
    t.into()
}

struct ComponentArgs {
    // The name of the cleanup function.
    cleanup_ident: Option<Ident>,
}

impl Parse for ComponentArgs {
    #[inline]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let cleanup_ident = input.parse().ok();
        Ok(Self { cleanup_ident })
    }
}
