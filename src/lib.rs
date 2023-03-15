use std::{
    collections::{HashMap, HashSet},
    num::NonZeroUsize,
};

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
    WrongType(String),
}

macro_rules! number {
    ($ty:ident) => {
        impl IntoAttributeValue for $ty {
            fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
                aws_sdk_dynamodb::model::AttributeValue::N(self.to_string())
            }

            fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error> {
                if let aws_sdk_dynamodb::model::AttributeValue::N(n) = av {
                    n.parse::<$ty>().map_err(|e| {
                        Error::WrongType(format!("Could not parse number, parse error {:?}", e))
                    })
                } else {
                    Err(Error::WrongType(format!("Expected N, got {:?}", av)))
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
            Err(Error::WrongType(format!("Expected S, got {:?}", av)))
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
            _ => Err(Error::WrongType(format!("Expected B, got {:?}", av))),
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
            Err(Error::WrongType(format!("Expected L, got {:?}", av)))
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
            Err(Error::WrongType(format!("Expected Bool, got {:?}", av)))
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
            Err(Error::WrongType(format!("Expected M, got {:?}", av)))
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
            .map_err(|_| Error::WrongType(format!("Expected SS, got {:?}", av)))
            .map(|ss| ss.iter().map(|s| s.to_owned()).collect())
    }
}

impl IntoAttributeValue for NonZeroUsize {
    fn into_av(self) -> aws_sdk_dynamodb::model::AttributeValue {
        aws_sdk_dynamodb::model::AttributeValue::N(self.get().to_string())
    }

    fn from_av(av: aws_sdk_dynamodb::model::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::model::AttributeValue::N(n) = av {
            n.parse::<NonZeroUsize>()
                .map_err(|e| Error::WrongType(format!("Expected N>0, parse error: {e:?}",)))
        } else {
            Err(Error::WrongType(format!("Expected N, got {:?}", av)))
        }
    }
}
