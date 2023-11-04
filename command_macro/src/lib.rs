use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;

/// # Panics
///
/// Will panic if attribute is not an identifier with our withouth a package path, or an int
#[proc_macro_attribute]
pub fn command(attr: TokenStream, mut input: TokenStream) -> TokenStream {
    let code = {
        let args_parsed =
            syn::punctuated::Punctuated::<syn::LitInt, syn::Token![,]>::parse_terminated
                .parse(attr.clone());

        if let Ok(args_parsed) = args_parsed {
            let value = args_parsed.first().unwrap().base10_parse::<u8>().unwrap();
            quote!(#value)
        } else {
            let args_parsed =
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
                    .parse(attr)
                    .unwrap();

            let path = args_parsed.first().unwrap();

            quote!(#path)
        }
    };

    let cloned = input.clone();
    let input_parsed = syn::parse_macro_input!(cloned as syn::ItemStruct);

    let name = input_parsed.ident.clone();

    let impl_write_wire = derive_write_wire(&input_parsed, true);
    let impl_read_wire = derive_read_wire(&input_parsed);

    let gen = quote! {
        #impl_write_wire
        #impl_read_wire

        impl felis_command::Command for #name {
            fn code() -> u8 {
                #code
            }
        }
    };

    input.extend::<TokenStream>(gen.into());
    input
}

#[proc_macro_derive(WriteWire)]
pub fn write_wire_derive(item: TokenStream) -> TokenStream {
    let input_parsed = syn::parse_macro_input!(item as syn::ItemStruct);

    derive_write_wire(&input_parsed, false).into()
}

#[proc_macro_derive(ReadWire)]
pub fn read_wire_derive(item: TokenStream) -> TokenStream {
    let input_parsed = syn::parse_macro_input!(item as syn::ItemStruct);

    derive_read_wire(&input_parsed).into()
}

fn derive_write_wire(item: &syn::ItemStruct, write_code: bool) -> proc_macro2::TokenStream {
    let name = item.ident.clone();
    let write_fields = item.fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        quote! {
            self.#name.write(writer).await?;
        }
    });

    let code = if write_code {
        quote!(#name::code().write(writer).await?;)
    } else {
        proc_macro2::TokenStream::default()
    };

    quote! {
        impl <W: AsyncWrite + Unpin + Send> felis_command::WriteWire<W> for #name {
            fn write<'life0,'life1,'async_trait>(&'life0 self,writer: &'life1 mut W) ->  core::pin::Pin<Box<dyn core::future::Future<Output = felis_command::WriteResult> + core::marker::Send+'async_trait>>
            where 'life0:'async_trait,'life1:'async_trait,Self:'async_trait
            {
                Box::pin(async {
                    #code
                    #(#write_fields)*
                    Ok(())
                })
            }
        }
    }
}

fn derive_read_wire(item: &syn::ItemStruct) -> proc_macro2::TokenStream {
    let init_fields = item.fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        let tpe = field.ty.clone();
        quote! {
            let #name = *#tpe::read(reader).await?;
        }
    });

    let field_names = item
        .fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap());

    let name = item.ident.clone();

    quote! {
        impl <R: AsyncRead + Unpin + Send> felis_command::ReadWire<R> for #name {
            fn read<'life0,'async_trait>(reader: &'life0 mut R) ->  core::pin::Pin<Box<dyn core::future::Future<Output = felis_command::ReadResult<Box<Self> > > + core::marker::Send+'async_trait>>
            where 'life0:'async_trait,Self:'async_trait
            {
                Box::pin(async {
                    #(#init_fields)*
                    Ok(Box::new(Self {
                        #(#field_names),*
                    }))
                })
            }
        }
    }
}
