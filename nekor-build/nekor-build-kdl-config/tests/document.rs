//! Behavioral tests for typed KDL document invariants.

use nekor_build_kdl_config::{
    document::Document,
    error::{LoadError, TypeError},
    merge::{Merge, MergeError},
    node::NodeKind,
    overlay::{Overlay, OverlayError},
    scalar::{IntegerType, ScalarValue, TypeAnnotation},
    source::Source,
};

/// Verify that the full unsigned sixty-four-bit domain survives KDL parsing.
#[test]
fn parse_u64_max_without_loss() {
    let result = Document::parse("value (u64)0xffff_ffff_ffff_ffff\n", &Source::anonymous());

    assert!(result.is_ok(), "{result:?}");

    if let Ok(document) = result {
        let value = document.get("value");

        assert!(value.is_some_and(|node| {
            matches!(
                node.kind(),
                NodeKind::Scalar(scalar)
                    if scalar.annotation()
                        == Some(&TypeAnnotation::Integer(IntegerType::U64))
                        && scalar.value() == &ScalarValue::U64(u64::MAX)
            )
        }));
    }
}

/// Verify that reserved integer annotations enforce their exact range.
#[test]
fn reject_out_of_range_integer() {
    let result = Document::parse("value (u8)256\n", &Source::anonymous());

    assert!(matches!(
        result,
        Err(LoadError::Type(TypeError::IntegerRange {
            representation: IntegerType::U8,
            value: 256,
            ..
        }))
    ));
}

/// Verify that dashed children preserve one-element list identity.
#[test]
fn parse_single_element_list() {
    let result = Document::parse("values {\n    - only\n}\n", &Source::anonymous());

    assert!(result.is_ok(), "{result:?}");
    if let Ok(document) = result {
        assert!(document.get("values").is_some_and(|node| {
            matches!(node.kind(), NodeKind::List(values) if values.len() == 1)
        }));
    }
}

/// Verify that same-layer terminal duplicates report both source origins.
#[test]
fn reject_duplicate_terminal_merge() {
    let left = Document::parse("layout { base 1 }\n", &Source::file("left.kdl"));
    let right = Document::parse("layout { base 2 }\n", &Source::file("right.kdl"));

    assert!(left.is_ok(), "{left:?}");
    assert!(right.is_ok(), "{right:?}");

    if let (Ok(left), Ok(right)) = (left, right) {
        let result = left.merge(right);

        assert!(result.is_err(), "{result:?}");
        if let Err(MergeError::DuplicateValue {
            path,
            existing,
            incoming,
        }) = result
        {
            assert_eq!(path.to_string(), "layout.base");
            assert_eq!(
                existing.source().path().map(|path| path.as_str()),
                Some("left.kdl")
            );
            assert_eq!(
                incoming.source().path().map(|path| path.as_str()),
                Some("right.kdl")
            );
        }
    }
}

/// Verify that an unannotated derived integer inherits its exact
/// representation.
#[test]
fn overlay_inherits_integer_representation() {
    let base = Document::parse("layout { base (u64)1 }\n", &Source::file("base.kdl"));
    let derived = Document::parse("layout { base 0 }\n", &Source::file("derived.kdl"));

    assert!(base.is_ok(), "{base:?}");
    assert!(derived.is_ok(), "{derived:?}");

    if let (Ok(base), Ok(derived)) = (base, derived) {
        let result = base.overlay(derived);

        assert!(result.is_ok(), "{result:?}");
        if let Ok(resolved) = result {
            assert!(resolved.get("layout").is_some_and(|layout| {
                let NodeKind::Object(layout) = layout.kind() else {
                    return false;
                };
                layout.get("base").is_some_and(|node| {
                    matches!(
                        node.kind(),
                        NodeKind::Scalar(scalar)
                            if scalar.annotation()
                                == Some(&TypeAnnotation::Integer(IntegerType::U64))
                                && scalar.value() == &ScalarValue::U64(0)
                    )
                })
            }));
        }
    }
}

/// Verify that derived platforms cannot change an inherited scalar
/// representation.
#[test]
fn overlay_rejects_representation_change() {
    let base = Document::parse("layout { base (u64)1 }\n", &Source::file("base.kdl"));
    let derived = Document::parse("layout { base (i64)0 }\n", &Source::file("derived.kdl"));

    assert!(base.is_ok(), "{base:?}");
    assert!(derived.is_ok(), "{derived:?}");

    if let (Ok(base), Ok(derived)) = (base, derived) {
        let result = base.overlay(derived);

        assert!(matches!(result, Err(OverlayError::ValueAnnotation { .. })));
    }
}
