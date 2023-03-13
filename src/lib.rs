use std::collections::{HashMap, HashSet};

use aws_sdk_dynamodb::types::Blob;
use thiserror::Error;

pub enum IterableType {
    Blob,
    List,
}
pub trait IntoAttributeValue {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue;

    fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized;
}

pub trait IntoDynamoItem {
    fn into_item(self) -> HashMap<String, aws_sdk_dynamodb::model::AttributeValue>;

    fn from_item(
        item: HashMap<String, aws_sdk_dynamodb::model::AttributeValue>,
    ) -> Result<Self, Error>
    where
        Self: Sized;
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Wrong type")]
    WrongType,

    #[error("Could not parse")]
    Parse,
}

macro_rules! number {
    ($ty:ident) => {
        impl IntoAttributeValue for $ty {
            fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
                aws_sdk_dynamodb::model::AttributeValue::N(self.to_string())
            }

            fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error> {
                if let aws_sdk_dynamodb::model::AttributeValue::N(n) = av {
                    n.parse::<$ty>().map_err(|_| Error::Parse)
                } else {
                    Err(Error::WrongType)
                }
            }
        }
    };
}

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

    fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::model::AttributeValue::S(s) = av {
            Ok(s)
        } else {
            Err(Error::WrongType)
        }
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

    fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::model::AttributeValue::Null(_) = av {
            Ok(None)
        } else {
            T::from_av(av).map(Some)
        }
    }
}

impl IntoAttributeValue for Vec<u8> {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        aws_sdk_dynamodb::model::AttributeValue::B(Blob::new(self))
    }

    fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match av {
            aws_sdk_dynamodb::model::AttributeValue::B(blob) => Ok(blob.into_inner()),
            _ => Err(Error::WrongType),
        }
    }
}

impl<T: IntoAttributeValue> IntoAttributeValue for Vec<T> {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        aws_sdk_dynamodb::model::AttributeValue::L(
            self.into_iter().map(|item| item.into_av()).collect(),
        )
    }

    fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::model::AttributeValue::L(l) = av {
            l.into_iter()
                .map(|item| T::from_av(item))
                .collect::<Result<Vec<_>, _>>()
        } else {
            Err(Error::WrongType)
        }
    }
}

impl IntoAttributeValue for bool {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        aws_sdk_dynamodb::model::AttributeValue::Bool(self)
    }

    fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::model::AttributeValue::Bool(b) = av {
            Ok(b)
        } else {
            Err(Error::WrongType)
        }
    }
}

impl<T: IntoAttributeValue> IntoAttributeValue for HashMap<String, T> {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        aws_sdk_dynamodb::model::AttributeValue::M(
            self.into_iter()
                .map(|(key, value)| (key, value.into_av()))
                .collect(),
        )
    }

    fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::model::AttributeValue::M(m) = av {
            m.into_iter()
                .map(|(key, value)| T::from_av(value).map(|value| (key, value)))
                .collect::<Result<HashMap<_, _>, _>>()
        } else {
            Err(Error::WrongType)
        }
    }
}

impl<T: IntoAttributeValue> IntoDynamoItem for HashMap<String, T> {
    fn into_item(self) -> HashMap<String, aws_sdk_dynamodb::model::AttributeValue> {
        self.into_iter().map(|(k, v)| (k, v.into_av())).collect()
    }

    fn from_item(
        item: HashMap<String, aws_sdk_dynamodb::model::AttributeValue>,
    ) -> Result<Self, Error>
    where
        Self: Sized,
    {
        item.into_iter()
            .map(|(key, value)| T::from_av(value).map(|value| (key, value)))
            .collect::<Result<HashMap<_, _>, _>>()
    }
}

impl IntoAttributeValue for HashSet<String> {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        aws_sdk_dynamodb::model::AttributeValue::Ss(self.into_iter().collect())
    }

    fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        av.as_ss()
            .map_err(|_| Error::WrongType)
            .map(|ss| ss.iter().map(|s| s.to_owned()).collect())
    }
}

mod tests {
    use aws_sdk_dynamodb::model::AttributeValue;

    use super::IntoAttributeValue;

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

        fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, crate::Error>
        where
            Self: Sized,
        {
            todo!()
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
