use aws_sdk_dynamodb::model::AttributeValue;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Field, GenericArgument, Ident};

fn into_av(
    ty: syn::Type,
    field_name: &proc_macro2::TokenStream,
) -> Option<proc_macro2::TokenStream> {
    match ty {
        syn::Type::Path(path) => {
            let path = path.path;
            let segment = path.segments.into_iter().next().unwrap();
            let ident = segment.ident;

            match ident.to_string().as_str() {
                "bool" => Some(quote!(aws_sdk_dynamodb::model::AttributeValue::Bool(#field_name))),
                "String" => Some(quote!(aws_sdk_dynamodb::model::AttributeValue::S(#field_name))),
                "usize" | "isize" | "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16"
                | "u32" | "u64" | "u128" | "f32" | "f64" => Some(
                    quote!(aws_sdk_dynamodb::model::AttributeValue::N(#field_name.to_string())),
                ),
                "Option" => match segment.arguments {
                    syn::PathArguments::AngleBracketed(arguments) => {
                        match arguments.args.into_iter().next() {
                            Some(generic_argument) => match generic_argument {
                                GenericArgument::Type(ty) => {
                                    into_av(ty, &quote!(#field_name.unwrap()))
                                }
                                _ => todo!(),
                            },
                            _ => todo!(),
                        }
                    }
                    _ => todo!(),
                },

                "Vec" => match segment.arguments {
                    syn::PathArguments::AngleBracketed(arguments) => {
                        match arguments.args.into_iter().next() {
                            Some(generic_argument) => match generic_argument {
                                GenericArgument::Type(ty) => {
                                    let vec_item = quote!(vec_item);
                                    let inner = into_av(ty, &vec_item);
                                    Some(quote!(aws_sdk_dynamodb::model::AttributeValue::L(
                                        #field_name.into_iter().map(|#vec_item| #inner).collect()
                                    )))
                                }
                                _ => todo!(),
                            },
                            _ => todo!(),
                        }
                    }
                    _ => todo!(),
                },
                _ => todo!(),
            }
        }
        _ => todo!(),
    }
}

#[proc_macro_derive(DynamoItem)]
pub fn derive_dynamo_item_fn(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;

    let binding = match input.data {
        syn::Data::Struct(data) => data.fields,
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };

    let fields = binding.into_iter().filter_map(|field| {
        let Field { ident, ty, .. } = field;

        let field_name = ident.unwrap();
        let field_name_string = field_name.to_string();

        let struct_field_name = quote!(self.#field_name);

        let av = into_av(ty, &struct_field_name);

        quote!((#field_name_string.to_string(), #av))
    });

    quote! {
        use aws_sdk_dynamodb;

        impl #struct_name {

            pub fn into_dynamo_item(self) -> std::collections::HashMap<String, aws_sdk_dynamodb::model::AttributeValue> {
                std::collections::HashMap::from_iter(
                    [#(#fields),*]
                )
            }
        }
    }
    .into()
}
