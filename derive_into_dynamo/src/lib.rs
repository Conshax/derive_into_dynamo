use proc_macro::TokenStream;
use quote::{quote};
use syn::{parse_macro_input, DeriveInput, Field};


#[proc_macro_derive(IntoDynamoItem)]
pub fn derive_dynamo_item_fn(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;

    let binding = match input.data {
        syn::Data::Struct(data) => data.fields,
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };

    let fields = binding.into_iter().map(|field| {
        let Field { ident,  .. } = field;

        let field_name = ident.unwrap();
        let field_name_string = field_name.to_string();

        quote!((#field_name_string.to_string(), self.#field_name.into_av()))
    });

    quote! {
        impl #struct_name {
            pub fn into_dynamo_item(self) -> std::collections::HashMap<String, aws_sdk_dynamodb::model::AttributeValue> {
                std::collections::HashMap::from_iter(
                    [#(#fields),*]
                )
                
            }
        }

        impl IntoAttributeValue for #struct_name {
            fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
                aws_sdk_dynamodb::model::AttributeValue::M(self.into_dynamo_item())
            }
        }

    }
    .into()
}
