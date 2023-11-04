use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(WriteWire)]
pub fn write_wire_derive(item: TokenStream) -> TokenStream {
    let cloned = item.clone();
    let input_parsed = syn::parse_macro_input!(cloned as DeriveInput);

    match input_parsed.data {
        syn::Data::Enum(_) => {
            derive_write_wire_enum(&syn::parse_macro_input!(item as syn::ItemEnum)).into()
        }
        syn::Data::Struct(_) => {
            derive_write_wire_struct(&syn::parse_macro_input!(item as syn::ItemStruct)).into()
        }
        syn::Data::Union(_) => todo!(),
    }
}

#[proc_macro_derive(ReadWire)]
pub fn read_wire_derive(item: TokenStream) -> TokenStream {
    let cloned = item.clone();
    let input_parsed = syn::parse_macro_input!(cloned as DeriveInput);

    match input_parsed.data {
        syn::Data::Enum(_) => {
            derive_read_wire_enum(&syn::parse_macro_input!(item as syn::ItemEnum)).into()
        }
        syn::Data::Struct(_) => {
            derive_read_wire_struct(&syn::parse_macro_input!(item as syn::ItemStruct)).into()
        }
        syn::Data::Union(_) => todo!(),
    }
}

fn derive_write_wire_struct(item: &syn::ItemStruct) -> proc_macro2::TokenStream {
    let name = item.ident.clone();
    let write_fields = item.fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        quote! {
            self.#name.write(writer).await?;
        }
    });

    quote! {
        impl <W: AsyncWrite + Unpin + Send> felis_command::WriteWire<W> for #name {
            fn write<'life0,'life1,'async_trait>(&'life0 self,writer: &'life1 mut W) ->  core::pin::Pin<Box<dyn core::future::Future<Output = felis_command::WriteResult> + core::marker::Send+'async_trait>>
            where 'life0:'async_trait,'life1:'async_trait,Self:'async_trait
            {
                Box::pin(async {
                    #(#write_fields)*
                    Ok(())
                })
            }
        }
    }
}

fn derive_read_wire_struct(item: &syn::ItemStruct) -> proc_macro2::TokenStream {
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
        impl <R: tokio::io::AsyncRead + Unpin + Send> felis_command::ReadWire<R> for #name {
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

fn derive_write_wire_enum(item: &syn::ItemEnum) -> proc_macro2::TokenStream {
    let name = item.ident.clone();
    let variant_cases = item.variants.iter().enumerate().map(|(ordinal, variant)| {
        let variant_name = variant.ident.clone();

        match variant.fields.clone() {
            syn::Fields::Named(named_fields) => {
                let fields = named_fields
                    .named
                    .iter()
                    .map(|named| named.ident.as_ref().unwrap());
                let write_fields = fields.clone().map(|field| {
                    quote! {
                        #field.write(writer).await?;
                    }
                });

                quote! {
                    #name::#variant_name { #(#fields),* } => {
                        #ordinal.write(writer).await?;
                        #(#write_fields)*
                        Ok(())
                    }
                }
            }
            syn::Fields::Unnamed(unnamed_fields) => {
                let fields = (0..unnamed_fields.unnamed.len()).map(|index| {
                    syn::Ident::new(format!("value_{index}").as_str(), Span::call_site())
                });

                let write_fields = fields.clone().map(|field| {
                    quote! {
                        #field.write(writer).await?;
                    }
                });

                quote! {
                    #name::#variant_name(#(#fields),*) => {
                        #ordinal.write(writer).await?;
                        #(#write_fields)*
                        Ok(())
                    }
                }
            }
            syn::Fields::Unit => quote! {
                #name::#variant_name => #ordinal.write(writer).await,
            },
        }
    });

    quote! {
        impl <W: tokio::io::AsyncWrite + Unpin + Send> felis_command::WriteWire<W> for #name {
            fn write<'life0,'life1,'async_trait>(&'life0 self,writer: &'life1 mut W) ->  core::pin::Pin<Box<dyn core::future::Future<Output = felis_command::WriteResult> + core::marker::Send+'async_trait>>
            where 'life0:'async_trait,'life1:'async_trait,Self:'async_trait
            {
                Box::pin(async move {
                    match self {
                        #(#variant_cases)*
                    }
                })
            }
        }
    }
}

fn derive_read_wire_enum(item: &syn::ItemEnum) -> proc_macro2::TokenStream {
    let name = item.ident.clone();
    let variant_cases = item.variants.iter().enumerate().map(|(ordinal, variant)| {
        let variant_name = variant.ident.clone();

        let value =
            match variant.fields.clone() {
                syn::Fields::Named(named_fields) => {
                    let fields = named_fields
                        .named
                        .iter()
                        .map(|named| named.ident.as_ref().unwrap());
                    let read_fields = named_fields.named.iter().map(|field| {
                        let tpe = field.ty.clone();
                        let field_name = field.ident.as_ref().unwrap();
                        quote! {
                            let #field_name = *#tpe::read(reader).await?;
                        }
                    });
                    quote! {
                        {
                            #(#read_fields)*
                            Ok(#name::#variant_name { #(#fields),* })
                        }
                    }
                }
                syn::Fields::Unnamed(unnamed_fields) => {
                    let fields = (0..unnamed_fields.unnamed.len()).map(|index| {
                        syn::Ident::new(format!("value_{index}").as_str(), Span::call_site())
                    });
                    let read_fields = unnamed_fields.unnamed.iter().zip(fields.clone()).map(
                        |(field, field_name)| {
                            let tpe = field.ty.clone();
                            quote! {
                                let #field_name = *#tpe::read(reader).await?;
                            }
                        },
                    );
                    quote! {
                        {
                            #(#read_fields)*
                            Ok(#name::#variant_name(#(#fields),*))
                        }
                    }
                }
                syn::Fields::Unit => quote!(Ok(#name::#variant_name)),
            };

        quote! {
            #ordinal => #value,
        }
    });
    quote! {
        impl <R: tokio::io::AsyncRead + Unpin + Send> felis_command::ReadWire<R> for #name {
            fn read<'life0,'async_trait>(reader: &'life0 mut R) ->  core::pin::Pin<Box<dyn core::future::Future<Output = felis_command::ReadResult<Box<Self> > > + core::marker::Send+'async_trait>>
            where 'life0:'async_trait,Self:'async_trait
            {
                Box::pin(async {
                    let ordinal = *usize::read(reader).await?;

                    let enum_variant = match ordinal {
                        #(#variant_cases)*
                        _ => Err(WireFormatReadError::UnexpectedError {message: format!("Couldn't create enum from ordinal {ordinal}")} )
                    };

                    enum_variant.map(Box::new)
                })
            }
        }
    }
}
