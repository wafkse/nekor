//! Serde projection for typed configuration values.

use serde::ser::{Serialize, SerializeMap, Serializer};

use crate::{
    document::Document,
    node::{Node, NodeKind, ScalarList},
    scalar::{Scalar, ScalarValue},
};

impl Serialize for Document {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields = self.fields();
        let mut map = serializer.serialize_map(Some(fields.len()))?;

        for (name, node) in fields {
            map.serialize_entry(name.as_str(), node)?;
        }

        map.end()
    }
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.kind().serialize(serializer)
    }
}
impl Serialize for NodeKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            &Self::Scalar(ref value) => value.serialize(serializer),
            &Self::Object(ref value) => value.serialize(serializer),
            &Self::List(ref value) => value.serialize(serializer),
        }
    }
}

impl Serialize for Scalar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value().serialize(serializer)
    }
}

impl Serialize for ScalarValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            &Self::Null => serializer.serialize_none(),
            &Self::Bool(value) => serializer.serialize_bool(value),
            &Self::String(ref value) => serializer.serialize_str(value),
            &Self::Integer(value) | &Self::I128(value) | &Self::Isize(value) => serializer.serialize_i128(value),
            &Self::I8(value) => serializer.serialize_i8(value),
            &Self::I16(value) => serializer.serialize_i16(value),
            &Self::I32(value) => serializer.serialize_i32(value),
            &Self::I64(value) => serializer.serialize_i64(value),
            &Self::U8(value) => serializer.serialize_u8(value),
            &Self::U16(value) => serializer.serialize_u16(value),
            &Self::U32(value) => serializer.serialize_u32(value),
            &Self::U64(value) => serializer.serialize_u64(value),
            &Self::U128(value) | &Self::Usize(value) => serializer.serialize_u128(value),
            &Self::F32(value) => serializer.serialize_f32(value),
            &Self::F64(value) => serializer.serialize_f64(value),
        }
    }
}

impl Serialize for ScalarList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_slice().serialize(serializer)
    }
}
