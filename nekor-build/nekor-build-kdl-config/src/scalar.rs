//! Typed scalar representation.

use crate::{
    annotation::Annotation,
    error::TypeError,
    name::Path,
    overlay::{Overlay, OverlayError},
    source::Origin,
};

/// The broad KDL scalar kind used for representation checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarKind {
    /// A null value.
    Null,
    /// A boolean value.
    Bool,
    /// A string value.
    String,
    /// An integer value.
    Integer,
    /// A floating point value.
    Float,
}

/// A scalar value and its KDL value annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct Scalar {
    // NOTE(invariant): When `annotation` is a reserved representation,
    // `value` has already been range checked for that representation.
    /// The optional reserved or application-defined annotation.
    annotation: Option<TypeAnnotation>,
    /// The scalar value after applying any reserved representation.
    value: ScalarValue,
    /// The source location that supplied the scalar.
    origin: Origin,
}

/// A scalar value after reserved KDL annotations are interpreted.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarValue {
    /// A null value.
    Null,
    /// A boolean value.
    Bool(bool),
    /// A string value.
    String(String),
    /// An unannotated KDL integer.
    Integer(i128),
    /// An eight-bit signed integer.
    I8(i8),
    /// A sixteen-bit signed integer.
    I16(i16),
    /// A thirty-two-bit signed integer.
    I32(i32),
    /// A sixty-four-bit signed integer.
    I64(i64),
    /// A one-hundred-twenty-eight-bit signed integer.
    I128(i128),
    /// A target-sized signed integer.
    Isize(i128),
    /// An eight-bit unsigned integer.
    U8(u8),
    /// A sixteen-bit unsigned integer.
    U16(u16),
    /// A thirty-two-bit unsigned integer.
    U32(u32),
    /// A sixty-four-bit unsigned integer.
    U64(u64),
    /// A one-hundred-twenty-eight-bit unsigned integer.
    U128(u128),
    /// A target-sized unsigned integer.
    Usize(u128),
    /// A thirty-two-bit floating point value.
    F32(f32),
    /// A sixty-four-bit floating point value.
    F64(f64),
}

/// A reserved signed or unsigned integer representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerType {
    /// `i8`.
    I8,
    /// `i16`.
    I16,
    /// `i32`.
    I32,
    /// `i64`.
    I64,
    /// `i128`.
    I128,
    /// `isize`.
    Isize,
    /// `u8`.
    U8,
    /// `u16`.
    U16,
    /// `u32`.
    U32,
    /// `u64`.
    U64,
    /// `u128`.
    U128,
    /// `usize`.
    Usize,
}

/// A reserved floating point representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatType {
    /// `f32`.
    F32,
    /// `f64`.
    F64,
}

impl IntegerType {
    /// Represent one KDL integer with this exact integer representation.
    #[must_use]
    pub fn represent(self, value: i128) -> Option<ScalarValue> {
        macro_rules! checked {
            ($target:ty, $variant:ident) => {
                <$target>::try_from(value).ok().map(ScalarValue::$variant)
            };
        }

        match self {
            Self::I8 => checked!(i8, I8),
            Self::I16 => checked!(i16, I16),
            Self::I32 => checked!(i32, I32),
            Self::I64 => checked!(i64, I64),
            Self::I128 => Some(ScalarValue::I128(value)),
            Self::Isize => Some(ScalarValue::Isize(value)),
            Self::U8 => checked!(u8, U8),
            Self::U16 => checked!(u16, U16),
            Self::U32 => checked!(u32, U32),
            Self::U64 => checked!(u64, U64),
            Self::U128 => checked!(u128, U128),
            Self::Usize => checked!(u128, Usize),
        }
    }
}

impl FloatType {
    /// Represent one KDL floating point value with this exact representation.
    #[must_use]
    pub fn represent(self, value: f64) -> Option<ScalarValue> {
        match self {
            Self::F32 => num_traits::ToPrimitive::to_f32(&value).map(ScalarValue::F32),
            Self::F64 => Some(ScalarValue::F64(value)),
        }
    }
}

/// A classified KDL value annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeAnnotation {
    /// A reserved integer representation.
    Integer(IntegerType),
    /// A reserved floating point representation.
    Float(FloatType),
    /// An application-defined annotation.
    Custom(Annotation),
}

impl TypeAnnotation {
    /// Classify a KDL value annotation.
    #[must_use]
    pub fn classify(annotation: Annotation) -> Self {
        match annotation.as_str() {
            "i8" => Self::Integer(IntegerType::I8),
            "i16" => Self::Integer(IntegerType::I16),
            "i32" => Self::Integer(IntegerType::I32),
            "i64" => Self::Integer(IntegerType::I64),
            "i128" => Self::Integer(IntegerType::I128),
            "isize" => Self::Integer(IntegerType::Isize),
            "u8" => Self::Integer(IntegerType::U8),
            "u16" => Self::Integer(IntegerType::U16),
            "u32" => Self::Integer(IntegerType::U32),
            "u64" => Self::Integer(IntegerType::U64),
            "u128" => Self::Integer(IntegerType::U128),
            "usize" => Self::Integer(IntegerType::Usize),
            "f32" => Self::Float(FloatType::F32),
            "f64" => Self::Float(FloatType::F64),
            _ => Self::Custom(annotation),
        }
    }
}

impl Scalar {
    /// Construct a scalar after type interpretation has succeeded.
    #[inline]
    pub(crate) const fn from_parts(annotation: Option<TypeAnnotation>, value: ScalarValue, origin: Origin) -> Self {
        Self {
            annotation,
            value,
            origin,
        }
    }

    /// Determine the classified value annotation.
    #[inline]
    #[must_use]
    pub const fn annotation(&self) -> Option<&TypeAnnotation> {
        let &Self { ref annotation, .. } = self;

        annotation.as_ref()
    }

    /// Determine the interpreted scalar value.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &ScalarValue {
        let &Self { ref value, .. } = self;

        value
    }

    /// Determine the source location that supplied this scalar.
    #[inline]
    #[must_use]
    pub const fn origin(&self) -> &Origin {
        let &Self { ref origin, .. } = self;

        origin
    }
}

impl ScalarValue {
    /// Determine the underlying scalar kind.
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> ScalarKind {
        match self {
            &Self::Null => ScalarKind::Null,
            &Self::Bool(..) => ScalarKind::Bool,
            &Self::String(..) => ScalarKind::String,
            &Self::Integer(..)
            | &Self::I8(..)
            | &Self::I16(..)
            | &Self::I32(..)
            | &Self::I64(..)
            | &Self::I128(..)
            | &Self::Isize(..)
            | &Self::U8(..)
            | &Self::U16(..)
            | &Self::U32(..)
            | &Self::U64(..)
            | &Self::U128(..)
            | &Self::Usize(..) => ScalarKind::Integer,
            &Self::F32(..) | &Self::F64(..) => ScalarKind::Float,
        }
    }
}

impl Overlay for Scalar {
    type Error = OverlayError;

    fn overlay(self, derived: Self) -> Result<Self, Self::Error> {
        let Self { annotation, origin, .. } = self;
        let Self {
            annotation: derived_annotation,
            value,
            origin: derived_origin,
        } = derived;

        let annotation = match (annotation, derived_annotation) {
            (Some(expected), Some(actual)) if expected != actual => {
                return Err(OverlayError::ValueAnnotation {
                    path: Path::new(),
                    expected,
                    actual,
                    inherited: origin,
                    derived: derived_origin,
                });
            },
            (Some(annotation), _) | (None, Some(annotation)) => Some(annotation),
            (None, None) => None,
        };

        let value = match (&annotation, value) {
            (&Some(TypeAnnotation::Integer(representation)), ScalarValue::Integer(value)) => representation
                .represent(value)
                .ok_or_else(|| OverlayError::ValueRepresentation {
                    path: Path::new(),
                    inherited: origin.clone(),
                    derived: derived_origin.clone(),
                    error: TypeError::IntegerRange {
                        representation,
                        value,
                        origin: derived_origin.clone(),
                    },
                })?,
            (&Some(TypeAnnotation::Float(representation)), ScalarValue::F64(value)) => representation
                .represent(value)
                .ok_or_else(|| OverlayError::ValueRepresentation {
                    path: Path::new(),
                    inherited: origin.clone(),
                    derived: derived_origin.clone(),
                    error: TypeError::FloatRange {
                        representation,
                        value,
                        origin: derived_origin.clone(),
                    },
                })?,
            (&Some(TypeAnnotation::Integer(representation)), value) if value.kind() != ScalarKind::Integer => {
                return Err(OverlayError::ValueRepresentation {
                    path: Path::new(),
                    inherited: origin,
                    derived: derived_origin.clone(),
                    error: TypeError::ScalarKind {
                        annotation: TypeAnnotation::Integer(representation),
                        expected: ScalarKind::Integer,
                        actual: value.kind(),
                        origin: derived_origin,
                    },
                });
            },
            (&Some(TypeAnnotation::Float(representation)), value) if value.kind() != ScalarKind::Float => {
                return Err(OverlayError::ValueRepresentation {
                    path: Path::new(),
                    inherited: origin,
                    derived: derived_origin.clone(),
                    error: TypeError::ScalarKind {
                        annotation: TypeAnnotation::Float(representation),
                        expected: ScalarKind::Float,
                        actual: value.kind(),
                        origin: derived_origin,
                    },
                });
            },
            (_, value) => value,
        };

        Ok(Self {
            annotation,
            value,
            origin: derived_origin,
        })
    }
}
