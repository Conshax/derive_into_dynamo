use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DataEnum, DataStruct, DeriveInput, Field, Ident, Variant, Type};

#[proc_macro_derive(IntoDynamoItem)]
pub fn derive_dynamo_item_fn(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    match input.data {
        syn::Data::Struct(data) => derive_dynamo_item_from_struct(data, name),
        syn::Data::Enum(data) => derive_dynamo_item_from_enum(data, name),
        syn::Data::Union(_) => quote!(compile_error!("Unions are not supported")).into(),
    }
}

fn enum_fields_into_map_tuples(variant: &Variant) -> TokenStream2 {

    let Variant {
        attrs,
        ident,
        fields,
        discriminant,
    } = variant;

    let (field_keys, field_name_idents): (Vec<String>, Vec<Ident>) = fields
        .iter()
        .enumerate()
        .map(|(i, _field)| (format!("field_{}_key", i), format_ident!("field_{}", i)))
        .unzip();

    quote!(
        #(
            (
                #field_keys,
                #field_name_idents.into_av()
            ),
        )*
    )
}

fn enum_from_fields(variant: &Variant) -> TokenStream2 {

    let Variant {
        attrs,
        ident,
        fields,
        discriminant,
    } = variant;

    let field_keys: Vec<String> = fields
        .iter()
        .enumerate()
        .map(|(i, _)| (format!("field_{}_key", i)))
        .collect();

    quote!(
        (
            #(
                into_dynamo::IntoAttributeValue::from_av(map.remove(&#field_keys).ok_or(into_dynamo::Error::WrongType)?),
            )*
        )
    )

}

Enum::ABC(field_1, field_2, field_3);



fn derive_dynamo_item_from_enum(data_enum: DataEnum, enum_name: Ident) -> TokenStream {
    let variants = data_enum.variants;

    let (variant_names, (variant_pattern_namess, variant_map_tupless)): (
        Vec<Ident>,
        (Vec<_>, Vec<_>),
    ) = variants
        .into_iter()
        .map(|variant| {
            let Variant {
                attrs,
                ident,
                fields,
                discriminant,
            } = variant;

            let (field_names, field_types): (Vec<String>, Vec<Type>) = fields
                .iter()
                .enumerate()
                .map(|(i, field)| (format_ident!("field_{}", i), field.ty))
                .unzip();

            let variant_name = ident;
            let variant_name_string = variant_name.to_string();

            let variant_pattern_names = quote! { #(#field_names),* };
            let variant_map_tuples = quote! { #((#field_name_strings, #field_names.into_av()),)* };
            let variant_map_tuples_from = quote! { #(#field_types::from_av(),)*}

            (ident, (variant_pattern_names, variant_map_tuples))
        })
        .unzip();

    let into_attribute_value = format_ident!("IntoAttributeValue_{}", enum_name);
    let into_dynamo_item = format_ident!("IntoDynamoItem_{}", enum_name);

    let variant_names_string = variant_names
        .iter()
        .map(|ident| ident.to_string())
        .collect::<Vec<_>>();

    quote! {
        use into_dynamo::IntoAttributeValue as #into_attribute_value;
        use into_dynamo::IntoDynamoItem as #into_dynamo_item;

        impl #into_dynamo_item for #enum_name {
            fn into_item(self) -> std::collections::HashMap<String, aws_sdk_dynamodb::model::AttributeValue> {
                let mut map = std::collections::HashMap::new();
                match self {
                    #(#enum_name::#variant_names(#variant_pattern_namess) => {
                        std::collections::HashMap::from_iter(
                            [#variant_map_tupless, ("enum", aws_sdk_dynamodb::model::AttributeValue::S(#variant_names_string))]
                        )
                    })*
                }
                map
            }

            fn from_item(mut map: std::collections::HashMap<String, aws_sdk_dynamodb::model::AttributeValue>) -> std::result::Result<Self, into_dynamo::Error> {
                let enum_name = map.remove("enum").ok_or(into_dynamo::Error::MissingField("enum".to_string()))?;
                #(let field_names = map.remove(#field_name_strings).ok_or(into_dynamo::Error::MissingField(#field_name_strings))?;)*

                match enum_name {
                    #(#variant_names_string => {
                        #enum_name::#variant_names(

                                #into_attribute_value::from_av(map.remove(#field_name_strings).ok_or(into_dynamo::Error::MissingField(#field_name_strings))?)?
                        )
                    })*
                    _ => Err(into_dynamo::Error::WrongType)
                }
            }
        }

        impl #into_attribute_value for #enum_name {
            fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
                match self {
                    #(#enum_name::#variant_names { .. } => {
                        let mut map = std::collections::HashMap::new();
                        map.insert("variant".to_string(), #variant_name_strings.into_av());
                        map.insert("value".to_string(), self.into_av());
                        aws_sdk_dynamodb::model::AttributeValue::M(map)
                    })*
                }
            }

            fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> std::result::Result<Self, into_dynamo::Error> {
                match av {
                    aws_sdk_dynamodb::model::AttributeValue::M(map) => {
                        let variant = map.get("variant").ok_or(into_dynamo::Error::MissingField("variant".to_string()))?;
                        let value = map.get("value").ok_or(into_dynamo::Error::MissingField("value".to_string()))?;

                        match variant {
                            #(#variant_name_strings => {
                                #into_attribute_value::from_av(value.clone()).map(#enum_name::#variant_names)
                            })*
                            _ => Err(into_dynamo::Error::InvalidVariant(variant.clone()))
                        }
                    }
                    _ => Err(into_dynamo::Error::InvalidAttributeValue(av))
                }
            }
        }

    }.into()
}

fn derive_dynamo_item_from_struct(data_struct: DataStruct, struct_name: Ident) -> TokenStream {
    let fields = data_struct.fields;

    let (field_name_strings, field_names): (Vec<TokenStream2>, Vec<Ident>) = fields
        .into_iter()
        .map(|field| {
            let Field { ident, .. } = field;

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
            fn into_item(self) -> std::collections::HashMap<String, aws_sdk_dynamodb::model::AttributeValue> {
                std::collections::HashMap::from_iter(
                    [#((#field_name_strings, self.#field_names.into_av())),*]
                )
            }

            fn from_item(mut map: std::collections::HashMap<String, aws_sdk_dynamodb::model::AttributeValue>) -> std::result::Result<Self, into_dynamo::Error> {
                Ok(#struct_name {
                    #(#field_names: #into_attribute_value::from_av(map.remove(&#field_name_strings).ok_or(into_dynamo::Error::WrongType)?)?),*
                })
            }
        }

        impl #into_attribute_value for #struct_name {
            fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
                aws_sdk_dynamodb::model::AttributeValue::M(self.into_item())
            }

            fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> std::result::Result<Self, into_dynamo::Error> {
                if let aws_sdk_dynamodb::model::AttributeValue::M(mut map) = av {
                    Ok(#struct_name {
                        #(#field_names: #into_attribute_value::from_av(map.remove(&#field_name_strings).ok_or(into_dynamo::Error::WrongType)?)?),*
                    })
                } else {
                    Err(into_dynamo::Error::WrongType)
                }
            }
        }

    }
    .into()
}
