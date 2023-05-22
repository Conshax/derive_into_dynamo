#![warn(clippy::pedantic)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::Lit::Str;
use syn::{parse_macro_input, DataStruct, DeriveInput, Field, Ident, MetaNameValue, Type};

mod enum_type;

#[proc_macro_derive(IntoDynamoItem, attributes(dynamo))]
pub fn derive_dynamo_item_fn(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        syn::Data::Struct(data) => derive_struct(&input.ident, data),
        syn::Data::Enum(data) => enum_type::derive_enum(&input.ident, data),
        syn::Data::Union(_) => quote!(compiler_error("Unions not implemented yet")),
    }
    .into()
}

fn is_default(attrs: &[syn::Attribute]) -> bool {
    if let Some(attr) = attrs.iter().find(|attr| attr.path.is_ident("dynamo")) {
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
    }
}

fn rename(attrs: &[syn::Attribute]) -> Option<String> {
    let get_rename = |name_value: &MetaNameValue| {
        if name_value.path.is_ident("rename") {
            if let Str(str) = &name_value.lit {
                Some(str.value())
            } else {
                None
            }
        } else {
            None
        }
    };

    attrs.iter().find_map(|attr| {
        if attr.path.is_ident("dynamo") {
            match attr.parse_meta() {
                Ok(meta) => match meta {
                    syn::Meta::Path(_) => None,
                    syn::Meta::NameValue(name_value) => get_rename(&name_value),
                    syn::Meta::List(l) => l.nested.iter().find_map(|meta| match meta {
                        syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) => get_rename(nv),
                        _ => None,
                    }),
                },
                Err(_) => None,
            }
        } else {
            None
        }
    })
}

fn derive_from_field_line(field: &Field) -> TokenStream2 {
    let Field {
        ident,
        attrs,
        vis: _,
        colon_token: _,
        ty,
    } = field;

    let default = is_default(attrs);

    let field_name = ident.clone().unwrap();
    let field_name_string = field_name.to_string();

    if default || is_option(ty) {
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

    let field_name = ident.clone().unwrap();
    let field_name_string = field_name.to_string();

    if is_option(ty) {
        quote! {
            if self.#field_name.is_none(){
                None
            } else {
                Some((#field_name_string.to_string(), self.#field_name.into_av()))
            }
        }
    } else {
        quote! {
            Some((#field_name_string.to_string(), self.#field_name.into_av()))
        }
    }
}

fn is_option(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        path.path
            .segments
            .iter()
            .any(|segment| segment.ident == "Option")
    } else {
        false
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
