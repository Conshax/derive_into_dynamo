use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{punctuated::Punctuated, token::Comma, DataEnum, Ident, Type, Variant};

struct NamedVariant {
    name: Ident,
    fields: Vec<NamedField>,
    rename: Option<String>,
}

struct NamedField {
    name: Ident,
}

struct UnnamedVariant {
    name: Ident,
    fields: Vec<UnnamedField>,
    rename: Option<String>,
}

struct UnnamedField {
    type_: Type,
}

struct UnitVariant {
    name: Ident,
    rename: Option<String>,
}

fn split_variants(
    variants: Punctuated<Variant, Comma>,
) -> (Vec<NamedVariant>, Vec<UnnamedVariant>, Vec<UnitVariant>) {
    variants.into_iter().fold(
        (Vec::new(), Vec::new(), Vec::new()),
        |(mut named_variants, mut unnamed_variants, mut unit_variants), variant| {
            let rename = super::rename(&variant.attrs);
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
                        rename,
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
                        rename,
                    });
                }
                syn::Fields::Unit => {
                    unit_variants.push(UnitVariant {
                        name: variant.ident,
                        rename,
                    });
                }
            };
            (named_variants, unnamed_variants, unit_variants)
        },
    )
}

fn build_named(
    enum_name: &Ident,
    named_variants: Vec<NamedVariant>,
) -> (Vec<TokenStream2>, Vec<TokenStream2>) {
    named_variants
        .into_iter()
        .map(|variant| {
            let name = variant.name;
            let name_string = variant.rename.unwrap_or(name.to_string());
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
        .unzip()
}

fn build_unnamed(
    enum_name: &Ident,
    unnamed_variants: Vec<UnnamedVariant>,
) -> (Vec<TokenStream2>, Vec<TokenStream2>) {
    unnamed_variants.into_iter().map(|variant|
        {
            let name = variant.name;
            let name_string = variant.rename.unwrap_or(name.to_string());
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
    ).unzip()
}

fn build_unions(
    enum_name: &Ident,
    unit_variants: Vec<UnitVariant>,
) -> (Vec<TokenStream2>, Vec<TokenStream2>) {
    unit_variants.into_iter().map(|variant|
        {
            let name = variant.name;
            let name_string = variant.rename.unwrap_or(name.to_string());

            (quote!(
                #enum_name::#name => aws_sdk_dynamodb::types::AttributeValue::S(#name_string.to_string())
            ),
            quote!(
                #name_string => Ok(#enum_name::#name)
            )
        )
        }
    ).unzip()
}

pub fn derive_enum(enum_name: &Ident, data: DataEnum) -> TokenStream2 {
    let (named_variants, unnamed_variants, unit_variants): (Vec<_>, Vec<_>, Vec<_>) =
        split_variants(data.variants);

    let (unit_into, unit_from): (Vec<TokenStream2>, Vec<TokenStream2>) =
        build_unions(enum_name, unit_variants);

    let (named_into, named_from): (Vec<_>, Vec<_>) = build_named(enum_name, named_variants);

    let (unnamed_into, unnamed_from): (Vec<_>, Vec<_>) = build_unnamed(enum_name, unnamed_variants);

    let into_attribute_value = format_ident!("IntoAttributeValue_{}", enum_name);

    let enum_name_string = enum_name.to_string();
    quote!(
        use into_dynamo::IntoAttributeValue as #into_attribute_value;

        impl #into_attribute_value for #enum_name {
            fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
                match self {
                    #(#unit_into,)*
                    #(#named_into,)*
                    #(#unnamed_into),*
                }
            }

            fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> std::result::Result<Self, into_dynamo::Error> {
                match av {
                    aws_sdk_dynamodb::types::AttributeValue::S(s) => {
                        match s.as_str() {
                            #(#unit_from,)*
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
