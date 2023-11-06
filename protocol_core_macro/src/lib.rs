use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::parse::Parser;

/// # Panics
///
/// This macro will panic if the attributes are not paths separated by commas
#[proc_macro_attribute]
pub fn wire_protocol_for(attr: TokenStream, mut input: TokenStream) -> TokenStream {
    let args_parsed = syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
        .parse(attr.clone())
        .expect("This macro expects a list of syn::Path");

    let protocol_impls = args_parsed.iter().map(|path| {
        let name = path.segments.first().unwrap().ident.clone();
        let write_method = Ident::new(format!("write_{name}").as_str(), Span::call_site());
        let read_method = Ident::new(format!("read_{name}").as_str(), Span::call_site());

        Into::<TokenStream>::into(
            quote! {

                impl <W: tokio::io::AsyncWrite + Unpin + Send> WireWrite<W> for #name {
                    fn write<'life0,'life1,'async_trait>(&'life0 self,writer: &'life1 mut W) ->  core::pin::Pin<Box<dyn core::future::Future<Output = WireWriteResult> + core::marker::Send+'async_trait>>
                    where 'life0:'async_trait,'life1:'async_trait,Self:'async_trait
                    {
                        Box::pin(async {
                            writer.#write_method(*self).await?;
                            Ok(())
                        })
                    }
                }
                impl <R: tokio::io::AsyncRead + Unpin + Send> WireRead<R> for #name {
                    fn read<'life0,'async_trait>(reader: &'life0 mut R) ->  core::pin::Pin<Box<dyn core::future::Future<Output = WireReadResult<Box<Self> > > + core::marker::Send+'async_trait>>
                    where 'life0:'async_trait,Self:'async_trait
                    {
                        Box::pin(async {
                            let value = reader.#read_method().await?;
                            Ok(Box::new(value))
                        })
                    }
                }
            }
        )
    });

    input.extend(protocol_impls);
    input
}
