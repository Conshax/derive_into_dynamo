use std::{
    collections::{HashMap, HashSet},
    num::NonZeroUsize,
};

use aws_sdk_dynamodb::primitives::Blob;
use thiserror::Error;

pub enum IterableType {
    Blob,
    List,
}
pub trait IntoAttributeValue {
    fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue;

    fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized;
}

pub trait IntoDynamoItem {
    fn into_item(self) -> HashMap<String, aws_sdk_dynamodb::types::AttributeValue>;

    fn from_item(
        item: HashMap<String, aws_sdk_dynamodb::types::AttributeValue>,
    ) -> Result<Self, Error>
    where
        Self: Sized;
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Wrong type {0}")]
    WrongType(String),
}

macro_rules! number {
    ($ty:ident) => {
        impl IntoAttributeValue for $ty {
            fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
                aws_sdk_dynamodb::types::AttributeValue::N(self.to_string())
            }

            fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error> {
                if let aws_sdk_dynamodb::types::AttributeValue::N(n) = av {
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
    fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
        aws_sdk_dynamodb::types::AttributeValue::S(self)
    }

    fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::types::AttributeValue::S(s) = av {
            Ok(s)
        } else {
            Err(Error::WrongType(format!("Expected S, got {:?}", av)))
        }
    }
}

impl<T: IntoAttributeValue> IntoAttributeValue for Option<T> {
    fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
        if let Some(inner) = self {
            inner.into_av()
        } else {
            aws_sdk_dynamodb::types::AttributeValue::Null(true)
        }
    }

    fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::types::AttributeValue::Null(_) = av {
            Ok(None)
        } else {
            T::from_av(av).map(Some)
        }
    }
}

impl IntoAttributeValue for Vec<u8> {
    fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
        aws_sdk_dynamodb::types::AttributeValue::B(Blob::new(self))
    }

    fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match av {
            aws_sdk_dynamodb::types::AttributeValue::B(blob) => Ok(blob.into_inner()),
            _ => Err(Error::WrongType(format!("Expected B, got {:?}", av))),
        }
    }
}

impl<T: IntoAttributeValue> IntoAttributeValue for Vec<T> {
    fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
        aws_sdk_dynamodb::types::AttributeValue::L(
            self.into_iter().map(|item| item.into_av()).collect(),
        )
    }

    fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::types::AttributeValue::L(l) = av {
            l.into_iter()
                .map(|item| T::from_av(item))
                .collect::<Result<Vec<_>, _>>()
        } else {
            Err(Error::WrongType(format!("Expected L, got {:?}", av)))
        }
    }
}

impl IntoAttributeValue for bool {
    fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
        aws_sdk_dynamodb::types::AttributeValue::Bool(self)
    }

    fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::types::AttributeValue::Bool(b) = av {
            Ok(b)
        } else {
            Err(Error::WrongType(format!("Expected Bool, got {:?}", av)))
        }
    }
}

impl<T: IntoAttributeValue> IntoAttributeValue for HashMap<String, T> {
    fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
        aws_sdk_dynamodb::types::AttributeValue::M(
            self.into_iter()
                .map(|(key, value)| (key, value.into_av()))
                .collect(),
        )
    }

    fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::types::AttributeValue::M(m) = av {
            m.into_iter()
                .map(|(key, value)| T::from_av(value).map(|value| (key, value)))
                .collect::<Result<HashMap<_, _>, _>>()
        } else {
            Err(Error::WrongType(format!("Expected M, got {:?}", av)))
        }
    }
}

impl<T: IntoAttributeValue> IntoDynamoItem for HashMap<String, T> {
    fn into_item(self) -> HashMap<String, aws_sdk_dynamodb::types::AttributeValue> {
        self.into_iter().map(|(k, v)| (k, v.into_av())).collect()
    }

    fn from_item(
        item: HashMap<String, aws_sdk_dynamodb::types::AttributeValue>,
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
    fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
        if self.is_empty() {
            aws_sdk_dynamodb::types::AttributeValue::Null(true)
        } else {
            aws_sdk_dynamodb::types::AttributeValue::Ss(self.into_iter().collect())
        }
    }

    fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match av {
            aws_sdk_dynamodb::types::AttributeValue::Ss(ss) => Ok(ss.into_iter().collect()),
            aws_sdk_dynamodb::types::AttributeValue::Null(_) => Ok(HashSet::new()),
            _ => Err(Error::WrongType(format!(
                "Expected SS or Null, got {:?}",
                av
            ))),
        }
    }
}

impl IntoAttributeValue for NonZeroUsize {
    fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
        aws_sdk_dynamodb::types::AttributeValue::N(self.get().to_string())
    }

    fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::types::AttributeValue::N(n) = av {
            n.parse::<NonZeroUsize>()
                .map_err(|e| Error::WrongType(format!("Expected N>0, parse error: {e:?}",)))
        } else {
            Err(Error::WrongType(format!("Expected N, got {:?}", av)))
        }
    }
}

impl IntoAttributeValue for (u64, String) {
    fn into_av(self) -> aws_sdk_dynamodb::types::AttributeValue {
        let first = self.0.into_av();
        let second = self.1.into_av();

        aws_sdk_dynamodb::types::AttributeValue::L(vec![first, second])
    }

    fn from_av(av: aws_sdk_dynamodb::types::AttributeValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let aws_sdk_dynamodb::types::AttributeValue::L(mut l) = av {
            let second = String::from_av(
                l.pop()
                    .ok_or(Error::WrongType("Expected L with 2 elements".into()))?,
            )?;
            let first = u64::from_av(
                l.pop()
                    .ok_or(Error::WrongType("Expected L with 2 elements".into()))?,
            )?;

            Ok((first, second))
        } else {
            Err(Error::WrongType(format!("Expected L, got {:?}", av)))
        }
    }
}
