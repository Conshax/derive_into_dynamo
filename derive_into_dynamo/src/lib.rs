#![warn(clippy::pedantic)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, DataEnum, DataStruct, DeriveInput,
    Field, Ident, Type, Variant,
};

#[proc_macro_derive(IntoDynamoItem, attributes(dynamo))]
pub fn derive_dynamo_item_fn(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        syn::Data::Struct(data) => derive_struct(&input.ident, data),
        syn::Data::Enum(data) => derive_enum(&input.ident, data),
        syn::Data::Union(_) => quote!(compiler_error("Unions not implemented yet")),
    }
    .into()
}

struct NamedVariant {
    name: Ident,
    fields: Vec<NamedField>,
}

struct NamedField {
    name: Ident,
}

struct UnnamedVariant {
    name: Ident,
    fields: Vec<UnnamedField>,
}

struct UnnamedField {
    type_: Type,
}

fn split_variants(
    variants: Punctuated<Variant, Comma>,
) -> (Vec<NamedVariant>, Vec<UnnamedVariant>, Vec<Ident>) {
    variants.into_iter().fold(
        (Vec::new(), Vec::new(), Vec::new()),
        |(mut named_variants, mut unnamed_variants, mut unit_variants), variant| {
            match variant.fields {
                syn::Fields::Named(fields) => {
                    named_variants.push(NamedVariant {
                        name: variant.ident,
                        fields: fields
                            .named
                            .into_iter()
                            .map(|field| NamedField {
                                name: field.ident.unwrap(),
                            })
                            .collect(),
                    });
                }
                syn::Fields::Unnamed(fields) => {
                    unnamed_variants.push(UnnamedVariant {
                        name: variant.ident,
                        fields: fields
                            .unnamed
                            .into_iter()
                            .map(|field| UnnamedField { type_: field.ty })
                            .collect(),
                    });
                }
                syn::Fields::Unit => {
                    unit_variants.push(variant.ident);
                }
            };
            (named_variants, unnamed_variants, unit_variants)
        },
    )
}

fn derive_enum(enum_name: &Ident, data: DataEnum) -> TokenStream2 {
    let (named_variants, unnamed_variants, unit_variants): (Vec<_>, Vec<_>, Vec<_>) =
        split_variants(data.variants);

    let unit_variants_string: Vec<String> = unit_variants
        .iter()
        .map(std::string::ToString::to_string)
        .collect();

    let (named_into, named_from): (Vec<_>, Vec<_>) = named_variants
        .into_iter()
        .map(|variant| {
            let name = variant.name;
            let name_string = name.to_string();
            let field_names: Vec<_> = variant.fields.into_iter().map(|field| field.name).collect();
            let field_name_strings: Vec<_> = field_names.iter().map(std::string::ToString::to_string).collect();

            (quote!(
                #enum_name::#name { #(#field_names),* } => aws_sdk_dynamodb::types::AttributeValue::M(
                    std::collections::HashMap::from_iter(
                        [#((#field_name_strings.to_string(), #field_names.into_av())),*,
                            (String::from("dynamo_enum_variant_name"), aws_sdk_dynamodb::types::AttributeValue::S(#name_string.to_string()))
                        ]
                ))
            ),
            quote!(
                #name_string => Ok(#enum_name::#name {
                    #(#field_names: into_dynamo::IntoAttributeValue::from_av(map.remove(#field_name_strings).ok_or(into_dynamo::Error::WrongType(format!("Missing field {}", #field_name_strings)))?)?),*
                })
            )
        )
        })
        .unzip();

    let (unnamed_into, unnamed_from): (Vec<_>, Vec<_>) = unnamed_variants.into_iter().map(|variant|
        {
            let name = variant.name;
            let name_string = name.to_string();
            let field_types: Vec<_> = variant.fields.into_iter().map(|field| field.type_).collect();
            let field_names: Vec<_> = (0..field_types.len()).map(|i| format_ident!("field_{}", i)).collect();
            let field_name_strings: Vec<_> = field_names.iter().map(std::string::ToString::to_string).collect();

            (quote!(
                #enum_name::#name(#(#field_names),*) => aws_sdk_dynamodb::types::AttributeValue::M(
                    std::collections::HashMap::from_iter(
                        [#((#field_name_strings.to_string(), #field_names.into_av())),*,
                            (String::from("dynamo_enum_variant_name"), aws_sdk_dynamodb::types::AttributeValue::S(#name_string.to_string()))
                        ]
                ))
            ),
            quote!(
                #name_string => Ok(#enum_name::#name(
                    #(into_dynamo::IntoAttributeValue::from_av(map.remove(#field_name_strings).ok_or(into_dynamo::Error::WrongType(format!("Missing field {}", #field_name_strings)))?)?),*
                ))
            )
        )
        }
    ).unzip();

    let into_attribute_value = format_ident!("IntoAttributeValue_{}", enum_name);

    let enum_name_string = enum_name.to_string();
    quote!(
        use into_dynamo::IntoAttributeValue as #into_attribute_value;

        impl #into_attribute_value for #enum_name {
            fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
                match self {
                    #(#enum_name::#unit_variants => aws_sdk_dynamodb::types::AttributeValue::S(#unit_variants_string.to_string()),)*
                    #(#named_into,)*
                    #(#unnamed_into),*
                }
            }

            fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> std::result::Result<Self, into_dynamo::Error> {
                match av {
                    aws_sdk_dynamodb::types::AttributeValue::S(s) => {
                        match s.as_str() {
                            #(#unit_variants_string => Ok(#enum_name::#unit_variants),)*
                            _ => Err(into_dynamo::Error::WrongType(format!("Expected variant of enum {}, got {:?}", #enum_name_string, s)))
                        }
                    }
                    aws_sdk_dynamodb::types::AttributeValue::M(mut map) => {
                        match map.remove("dynamo_enum_variant_name") {
                            Some(aws_sdk_dynamodb::types::AttributeValue::S(s)) => match s.as_str() {
                                #(#named_from,)*
                                #(#unnamed_from,)*
                                _ => Err(into_dynamo::Error::WrongType(format!("Expected variant of enum {}, got {:?}", #enum_name_string, s)))
                            },
                            av => Err(into_dynamo::Error::WrongType(format!("Expected S for dynamo_enum_variant_name, got {:?}", av)))
                        }
                    }
                    _ => Err(into_dynamo::Error::WrongType(format!("Expected S, got {:?}", av)))
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

    let field_name = ident.clone().unwrap();
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

fn derive_into_field_line(field: &Field) -> TokenStream2 {
    let Field {
        ident,
        attrs: _,
        vis: _,
        colon_token: _,
        ty,
    } = field;

    let option = if let Type::Path(path) = ty {
        path.path
            .segments
            .iter()
            .any(|segment| segment.ident == "Option")
    } else {
        false
    };

    let field_name = ident.clone().unwrap();
    let field_name_string = field_name.to_string();

    if option {
        quote! {
            if self.#field_name.is_none(){
                None
            } else {
                Some((#field_name_string.to_string(), self.#field_name.clone().into_av()))
            }
        }
    } else {
        quote! {
            Some((#field_name_string.to_string(), self.#field_name.into_av()))
        }
    }
}

fn derive_struct(struct_name: &Ident, data_struct: DataStruct) -> TokenStream2 {
    let binding = data_struct.fields;

    let from_field_lines: Vec<_> = binding.iter().map(derive_from_field_line).collect();

    let into_field_lines: Vec<_> = binding.iter().map(derive_into_field_line).collect();

    let into_attribute_value = format_ident!("IntoAttributeValue_{}", struct_name);
    let into_dynamo_item = format_ident!("IntoDynamoItem_{}", struct_name);

    quote! {
        use into_dynamo::IntoAttributeValue as #into_attribute_value;
        use into_dynamo::IntoDynamoItem as #into_dynamo_item;

        impl #into_dynamo_item for #struct_name {
            fn into_item(self) -> std::collections::HashMap<String, aws_sdk_dynamodb::types::AttributeValue> {
                std::collections::HashMap::from_iter(
                    [#(#into_field_lines),*].into_iter().filter_map(|x| x)
                )
            }

            fn from_item(mut map: std::collections::HashMap<String, aws_sdk_dynamodb::types::AttributeValue>) -> std::result::Result<Self, into_dynamo::Error> {
                Ok(#struct_name {
                    #(#from_field_lines),*
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
                        #(#from_field_lines),*
                    })
                } else {
                    Err(into_dynamo::Error::WrongType(format!("Expected M, got {:?}", av)))
                }
            }
        }

    }
}
