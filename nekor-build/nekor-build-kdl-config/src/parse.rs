//! KDL parsing and reserved annotation interpretation.

use indexmap::IndexMap;
use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};

use crate::{
    annotation::{Annotation, NodeAnnotation},
    document::Document,
    error::{LoadError, NodeShape, ParseError, ProfileError, TypeError},
    name::Name,
    node::{Node, NodeKind, ScalarList},
    scalar::{FloatType, IntegerType, Scalar, ScalarKind, ScalarValue, TypeAnnotation},
    source::{Origin, Source},
};

/// Parse one KDL source into a semantic document.
pub fn document(input: &str, source: &Source) -> Result<Document, LoadError> {
    let parsed = KdlDocument::parse_v2(input).map_err(|error| ParseError {
        source: source.clone(),
        error,
    })?;

    parse_document(&parsed, source)
}

/// Convert a parsed KDL document into the typed configuration profile.
fn parse_document(parsed: &KdlDocument, source: &Source) -> Result<Document, LoadError> {
    let mut fields: IndexMap<Name, Node> = IndexMap::new();

    for node in parsed.nodes() {
        let name = Name::new(node.name().value());
        let target = parse_node(node, source)?;

        if let Some((index, existing_name, existing)) = fields.shift_remove_full(&name) {
            use crate::merge::Merge;

            let merged = existing.merge(target).map_err(|error| error.prefixed(name))?;
            _ = fields.shift_insert(index, existing_name, merged);
        } else {
            _ = fields.insert(name, target);
        }
    }

    Ok(Document::from_fields(fields))
}

/// Convert one parsed KDL node into a semantic node.
fn parse_node(node: &KdlNode, source: &Source) -> Result<Node, LoadError> {
    let name = Name::new(node.name().value());
    let origin = Origin::new(source.clone(), node.span());

    if node.entries().iter().any(|entry| entry.name().is_some()) {
        return Err(ProfileError::UnsupportedNodeShape {
            name,
            shape: NodeShape::Properties,
            origin,
        }
        .into());
    }

    let annotation = node.ty().map(|annotation| NodeAnnotation::new(annotation.value()));

    let kind = match (node.entries(), node.children()) {
        (&[], Some(children)) => parse_children(children, source, &name, &origin)?,
        (&[], None) => NodeKind::Object(Document::default()),
        (&[ref entry], None) => NodeKind::Scalar(parse_scalar(entry, source)?),
        (_, None) => {
            return Err(ProfileError::UnsupportedNodeShape {
                name,
                shape: NodeShape::MultipleValues,
                origin,
            }
            .into());
        },
        (_, Some(_)) => {
            return Err(ProfileError::UnsupportedNodeShape {
                name,
                shape: NodeShape::ValuesAndChildren,
                origin,
            }
            .into());
        },
    };

    Ok(Node::from_parts(annotation, kind, origin))
}

/// Interpret a child block as either an object or a dashed scalar list.
fn parse_children(
    children: &KdlDocument,
    source: &Source,
    name: &Name,
    origin: &Origin,
) -> Result<NodeKind, LoadError> {
    let has_list_entry = children.nodes().iter().any(|child| child.name().value() == "-");

    if !has_list_entry {
        return Ok(NodeKind::Object(parse_document(children, source)?));
    }

    if children.nodes().iter().any(|child| child.name().value() != "-") {
        return Err(ProfileError::UnsupportedNodeShape {
            name: name.clone(),
            shape: NodeShape::MixedListChildren,
            origin: origin.clone(),
        }
        .into());
    }

    children
        .nodes()
        .iter()
        .map(|entry| parse_list_entry(entry, source, name))
        .collect::<Result<Vec<_>, _>>()
        .map(ScalarList::new)
        .map(NodeKind::List)
}

/// Interpret one dashed child node as a scalar list element.
fn parse_list_entry(node: &KdlNode, source: &Source, parent: &Name) -> Result<Scalar, LoadError> {
    let origin = Origin::new(source.clone(), node.span());
    let entries = node.entries();

    if node.ty().is_some() || node.children().is_some() || entries.len() != 1 || entries[0].name().is_some() {
        return Err(ProfileError::UnsupportedNodeShape {
            name: parent.clone(),
            shape: NodeShape::InvalidListEntry,
            origin,
        }
        .into());
    }

    parse_scalar(&entries[0], source)
}

/// Convert one KDL entry into an interpreted scalar.
fn parse_scalar(entry: &KdlEntry, source: &Source) -> Result<Scalar, LoadError> {
    let origin = Origin::new(source.clone(), entry.span());
    let annotation = entry.ty().map(|annotation| classify_annotation(annotation.value()));

    let value = interpret_value(entry.value(), annotation.as_ref(), &origin)?;

    Ok(Scalar::from_parts(annotation, value, origin))
}

/// Classify one KDL value annotation.
fn classify_annotation(annotation: &str) -> TypeAnnotation {
    match annotation {
        "i8" => TypeAnnotation::Integer(IntegerType::I8),
        "i16" => TypeAnnotation::Integer(IntegerType::I16),
        "i32" => TypeAnnotation::Integer(IntegerType::I32),
        "i64" => TypeAnnotation::Integer(IntegerType::I64),
        "i128" => TypeAnnotation::Integer(IntegerType::I128),
        "isize" => TypeAnnotation::Integer(IntegerType::Isize),
        "u8" => TypeAnnotation::Integer(IntegerType::U8),
        "u16" => TypeAnnotation::Integer(IntegerType::U16),
        "u32" => TypeAnnotation::Integer(IntegerType::U32),
        "u64" => TypeAnnotation::Integer(IntegerType::U64),
        "u128" => TypeAnnotation::Integer(IntegerType::U128),
        "usize" => TypeAnnotation::Integer(IntegerType::Usize),
        "f32" => TypeAnnotation::Float(FloatType::F32),
        "f64" => TypeAnnotation::Float(FloatType::F64),
        value => TypeAnnotation::Custom(Annotation::new(value)),
    }
}
/// Interpret a raw KDL value under an optional classified annotation.
fn interpret_value(
    value: &KdlValue,
    annotation: Option<&TypeAnnotation>,
    origin: &Origin,
) -> Result<ScalarValue, TypeError> {
    match annotation {
        Some(&TypeAnnotation::Integer(representation)) => {
            let &KdlValue::Integer(value) = value else {
                return Err(TypeError::ScalarKind {
                    annotation: TypeAnnotation::Integer(representation),
                    expected: ScalarKind::Integer,
                    actual: scalar_kind(value),
                    origin: origin.clone(),
                });
            };

            interpret_integer(value, representation, origin)
        },
        Some(&TypeAnnotation::Float(representation)) => {
            let &KdlValue::Float(value) = value else {
                return Err(TypeError::ScalarKind {
                    annotation: TypeAnnotation::Float(representation),
                    expected: ScalarKind::Float,
                    actual: scalar_kind(value),
                    origin: origin.clone(),
                });
            };

            interpret_float(value, representation, origin)
        },
        Some(&TypeAnnotation::Custom(..)) | None => Ok(natural_value(value)),
    }
}
/// Interpret a KDL integer under one reserved integer representation.
fn interpret_integer(value: i128, representation: IntegerType, origin: &Origin) -> Result<ScalarValue, TypeError> {
    representation.represent(value).ok_or_else(|| TypeError::IntegerRange {
        representation,
        value,
        origin: origin.clone(),
    })
}

/// Interpret a KDL float under one reserved floating point representation.
fn interpret_float(value: f64, representation: FloatType, origin: &Origin) -> Result<ScalarValue, TypeError> {
    representation.represent(value).ok_or_else(|| TypeError::FloatRange {
        representation,
        value,
        origin: origin.clone(),
    })
}

/// Convert an unannotated or custom-annotated KDL value without narrowing it.
fn natural_value(value: &KdlValue) -> ScalarValue {
    match value {
        &KdlValue::Null => ScalarValue::Null,
        &KdlValue::Bool(value) => ScalarValue::Bool(value),
        &KdlValue::String(ref value) => ScalarValue::String(value.clone()),
        &KdlValue::Integer(value) => ScalarValue::Integer(value),
        &KdlValue::Float(value) => ScalarValue::F64(value),
    }
}

/// Determine the scalar kind of a raw KDL value.
const fn scalar_kind(value: &KdlValue) -> ScalarKind {
    match value {
        &KdlValue::Null => ScalarKind::Null,
        &KdlValue::Bool(..) => ScalarKind::Bool,
        &KdlValue::String(..) => ScalarKind::String,
        &KdlValue::Integer(..) => ScalarKind::Integer,
        &KdlValue::Float(..) => ScalarKind::Float,
    }
}
