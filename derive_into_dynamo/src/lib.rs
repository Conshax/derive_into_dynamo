use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DataEnum, DataStruct, DeriveInput, Field, Ident};

#[proc_macro_derive(IntoDynamoItem, attributes(dynamo))]
pub fn derive_dynamo_item_fn(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        syn::Data::Struct(data) => derive_struct(input.ident, data),
        syn::Data::Enum(data) => derive_enum(input.ident, data),
        syn::Data::Union(_) => quote!(compiler_error("Unions not implemented yet")),
    }
    .into()
}

fn derive_enum(enum_name: Ident, data: DataEnum) -> TokenStream2 {
    let variants: Result<Vec<_>, TokenStream2> = data
        .variants
        .into_iter()
        .map(|variant| {
            if !variant.fields.is_empty() {
                Err(quote!(compiler_error!(
                    "Variants of enum must not have no fields, with fields have to be implemented manually for now"
                )))
            } else {
                Ok(variant.ident)
            }
        })
        .collect();

    let variants = match variants {
        Ok(variants) => variants,
        Err(err) => return err,
    };

    let variants_string: Vec<_> = variants.iter().map(|variant| variant.to_string()).collect();

    let into_attribute_value = format_ident!("IntoAttributeValue_{}", enum_name);

    quote!(
        use into_dynamo::IntoAttributeValue as #into_attribute_value;

        impl #into_attribute_value for #enum_name {
            fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
                match self {
                    #(#enum_name::#variants => aws_sdk_dynamodb::types::AttributeValue::S(#variants_string.to_string())),*
                }
            }

            fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> std::result::Result<Self, into_dynamo::Error> {
                if let aws_sdk_dynamodb::types::AttributeValue::S(s) = av {
                    match s.as_str() {
                        #(#variants_string => Ok(#enum_name::#variants),)*
                        _ => Err(into_dynamo::Error::WrongType(format!("Expected one of {:?}, got {:?}", [#(#variants_string),*], s)))
                    }
                } else {
                    Err(into_dynamo::Error::WrongType(format!("Expected S, got {:?}", av)))
                }
            }
        }

    )
}

fn derive_from_field_line(field: &Field) -> TokenStream2 {
    let Field {
        ident,
        attrs,
        vis: _,
        colon_token: _,
        ty: _,
    } = field;

    let default = if let Some(attr) = attrs.iter().find(|attr| attr.path.is_ident("dynamo")) {
        match attr.parse_meta() {
            Ok(meta) => match meta {
                syn::Meta::List(l) => match l.nested.first() {
                    Some(syn::NestedMeta::Meta(meta)) => meta.path().is_ident("default"),
                    _ => false,
                },
                _ => false,
            },
            Err(_) => false,
        }
    } else {
        false
    };

    let field_name = ident.to_owned().unwrap();
    let field_name_string = field_name.to_string();

    if default {
        quote! {
            #field_name: map.remove(#field_name_string).map(into_dynamo::IntoAttributeValue::from_av).transpose()?.unwrap_or_default()
        }
    } else {
        quote! {
            #field_name: into_dynamo::IntoAttributeValue::from_av(map.remove(#field_name_string).ok_or(into_dynamo::Error::WrongType(format!("Missing field {}", #field_name_string)))?)?
        }
    }
}

fn derive_struct(struct_name: Ident, data_struct: DataStruct) -> TokenStream2 {
    let binding = data_struct.fields;

    let field_lines: Vec<_> = binding.iter().map(derive_from_field_line).collect();

    let (field_name_strings, field_names): (Vec<TokenStream2>, Vec<Ident>) = binding
        .into_iter()
        .map(|field| {
            let Field {
                ident,
                attrs: _,
                vis: _,
                colon_token: _,
                ty: _,
            } = field;

            let field_name = ident.unwrap();
            let field_name_string = field_name.to_string();

            (quote!(#field_name_string.to_string()), field_name)
        })
        .unzip();

    let into_attribute_value = format_ident!("IntoAttributeValue_{}", struct_name);
    let into_dynamo_item = format_ident!("IntoDynamoItem_{}", struct_name);

    quote! {
        use into_dynamo::IntoAttributeValue as #into_attribute_value;
        use into_dynamo::IntoDynamoItem as #into_dynamo_item;

        impl #into_dynamo_item for #struct_name {
            fn into_item(self) -> std::collections::HashMap<String, aws_sdk_dynamodb::types::AttributeValue> {
                std::collections::HashMap::from_iter(
                    [#((#field_name_strings, self.#field_names.into_av())),*]
                )
            }

            fn from_item(mut map: std::collections::HashMap<String, aws_sdk_dynamodb::types::AttributeValue>) -> std::result::Result<Self, into_dynamo::Error> {
                Ok(#struct_name {
                    #(#field_lines),*
                })
            }
        }

        impl #into_attribute_value for #struct_name {
            fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
                aws_sdk_dynamodb::types::AttributeValue::M(self.into_item())
            }

            fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> std::result::Result<Self, into_dynamo::Error> {
                if let aws_sdk_dynamodb::types::AttributeValue::M(mut map) = av {
                    Ok(#struct_name {
                        #(#field_lines),*
                    })
                } else {
                    Err(into_dynamo::Error::WrongType(format!("Expected M, got {:?}", av)))
                }
            }
        }

    }
}
