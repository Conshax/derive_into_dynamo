use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Field, Ident};

#[proc_macro_derive(IntoDynamoItem)]
pub fn derive_dynamo_item_fn(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;

    let binding = match input.data {
        syn::Data::Struct(data) => data.fields,
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };

    let (field_name_strings, field_names): (Vec<TokenStream2>, Vec<Ident>) = binding
        .into_iter()
        .map(|field| {
            let Field { ident, .. } = field;

            let field_name = ident.unwrap();
            let field_name_string = field_name.to_string();

            (quote!(#field_name_string.to_string()), field_name)
        })
        .unzip();

    quote! {
        pub use into_dynamo::*;

        impl #struct_name {
            pub fn into_dynamo_item(self) -> std::collections::HashMap<String, aws_sdk_dynamodb::model::AttributeValue> {
                std::collections::HashMap::from_iter(
                    [#((#field_name_strings, self.#field_names.into_av())),*]
                )
            }

            pub fn from_dynamo_item(map: &std::collections::HashMap<String, aws_sdk_dynamodb::model::AttributeValue>) -> Result<Self, into_dynamo::Error> {
                Ok(#struct_name {
                    #(#field_names: IntoAttributeValue::from_av(map.get(&#field_name_strings).ok_or(into_dynamo::Error::WrongType)?)?),*
                })
            }
        }

        impl IntoAttributeValue for #struct_name {
            fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
                aws_sdk_dynamodb::model::AttributeValue::M(self.into_dynamo_item())
            }

            fn from_av(av: &aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, into_dynamo::Error> {
                av.as_m().map_err(|_| into_dynamo::Error::WrongType).and_then(|map| {
                    Ok(#struct_name {
                        #(#field_names: IntoAttributeValue::from_av(map.get(&#field_name_strings).ok_or(into_dynamo::Error::WrongType)?)?),*
                    })
                })
            }
        }

    }
    .into()
}
