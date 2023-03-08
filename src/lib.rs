use std::collections::HashSet;

use aws_sdk_dynamodb::model::AttributeValue;

pub trait IntoAttributeValue {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue;
}

macro_rules! number {
    ($ty:ident) => {
        impl IntoAttributeValue for $ty {
            fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
                aws_sdk_dynamodb::model::AttributeValue::N(self.to_string())
            }
        }
    };
}
number!(u8);
number!(u16);
number!(u32);
number!(u64);
number!(u128);
number!(usize);
number!(i8);
number!(i16);
number!(i32);
number!(i64);
number!(i128);
number!(isize);
number!(f32);
number!(f64);

impl IntoAttributeValue for String {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        aws_sdk_dynamodb::model::AttributeValue::S(self)
    }
}

impl<T: IntoAttributeValue> IntoAttributeValue for Option<T> {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        if let Some(inner) = self {
            inner.into_av()
        } else {
            aws_sdk_dynamodb::model::AttributeValue::Null(true)
        }
    }
}

impl<T: IntoAttributeValue> IntoAttributeValue for Vec<T> {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        aws_sdk_dynamodb::model::AttributeValue::L(
            self.into_iter().map(|item| item.into_av()).collect(),
        )
    }
}

impl IntoAttributeValue for bool {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        aws_sdk_dynamodb::model::AttributeValue::Bool(self)
    }
}

impl IntoAttributeValue for HashSet<String> {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        aws_sdk_dynamodb::model::AttributeValue::Ss(self.into_iter().collect())
    }
}

mod tests {
    use aws_sdk_dynamodb::model::AttributeValue;

    use crate::IntoAttributeValue;

    pub struct TestStruct {
        string_name: String,
        isize_name: isize,
        bool_name: bool,
        vec_string_name: Vec<String>,
        option_name_some: Option<String>,
        option_name_none: Option<Vec<String>>,
    }

    impl IntoAttributeValue for TestStruct {
        fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
            let mut map = std::collections::HashMap::new();
            map.insert("string_name".to_string(), self.string_name.into_av());
            map.insert("isize_name".to_string(), self.isize_name.into_av());
            map.insert("bool_name".to_string(), self.bool_name.into_av());
            map.insert(
                "vec_string_name".to_string(),
                self.vec_string_name.into_av(),
            );
            map.insert(
                "option_name_some".to_string(),
                self.option_name_some.into_av(),
            );
            map.insert(
                "option_name_none".to_string(),
                self.option_name_none.into_av(),
            );
            aws_sdk_dynamodb::model::AttributeValue::M(map)
        }
    }

    impl TestStruct {
        pub fn into_dynamo_item(self) -> std::collections::HashMap<String, AttributeValue> {
            let mut map = std::collections::HashMap::new();
            map.insert("string_name".to_string(), self.string_name.into_av());
            map.insert("isize_name".to_string(), self.isize_name.into_av());
            map.insert("bool_name".to_string(), self.bool_name.into_av());
            map.insert(
                "vec_string_name".to_string(),
                self.vec_string_name.into_av(),
            );
            map.insert(
                "option_name_some".to_string(),
                self.option_name_some.into_av(),
            );
            map.insert(
                "option_name_none".to_string(),
                self.option_name_none.into_av(),
            );

            map
        }
    }

    #[test]
    fn test_sruct() {
        let test = TestStruct {
            string_name: "test_value".to_string(),
            isize_name: -5000,
            bool_name: true,
            vec_string_name: vec!["test_value".to_string(), "test_value2".to_string()],
            option_name_some: Some("x".to_string()),
            option_name_none: None,
        };

        let item = test.into_dynamo_item();

        assert!(item.contains_key("string_name"));
    }
}
