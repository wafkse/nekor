//! Exercise rejection boundaries and target-dependent bit positions.

use super::{
    AccessorKind, AdaptationError, Adapter, AstIndex, BitfieldAccessor, CallCollector, Endian, LayoutResolver,
};

#[test]
fn offsets_follow_target_byte_order() -> Result<(), AdaptationError> {
    assert_eq!(Adapter::integer_offset(Endian::Little, 64, 2, 2, 3, 5)?, 19);
    assert_eq!(Adapter::integer_offset(Endian::Big, 64, 2, 2, 3, 5)?, 40);
    assert_eq!(Adapter::integer_offset(Endian::Little, 64, 7, 1, 7, 1)?, 63);
    assert_eq!(Adapter::integer_offset(Endian::Big, 64, 7, 1, 7, 1)?, 0);

    Ok(())
}

#[test]
fn ranges_cannot_escape_integer_storage() {
    for endian in [Endian::Little, Endian::Big] {
        assert!(matches!(
            Adapter::integer_offset(endian, 64, 7, 1, 7, 2),
            Err(AdaptationError::OutOfBounds(_))
        ));
    }

    assert!(matches!(
        Adapter::integer_offset(Endian::Little, 64, usize::MAX, 1, 0, 1),
        Err(AdaptationError::OutOfBounds(_))
    ));
    assert!(matches!(
        Adapter::integer_offset(Endian::Little, 64, 0, usize::MAX, 0, 1),
        Err(AdaptationError::Overflow(_))
    ));
}

#[test]
fn storage_requires_an_unqualified_byte_array() {
    for ty in [
        syn::parse_quote!(__BindgenBitfieldUnit<[u16; 2]>),
        syn::parse_quote!(__BindgenBitfieldUnit<[u8; 2], u8>),
        syn::parse_quote!(other::__BindgenBitfieldUnit<[u8; 2]>),
    ] {
        assert!(matches!(
            Adapter::unit_storage_bytes(&ty),
            Err(AdaptationError::Unsupported(_))
        ));
    }
}

#[test]
fn malformed_storage_is_not_skipped_during_discovery() -> Result<(), Box<dyn core::error::Error>> {
    let binding_ast = syn::parse_file("struct Record { unit: __BindgenBitfieldUnit<[u16; 2]> }")?;
    let index = AstIndex::new(&binding_ast)?;
    let resolver = LayoutResolver {
        index: &index,
        pointer_width: 64,
    };

    assert!(matches!(
        Adapter::accessors(&index, &resolver),
        Err(AdaptationError::Unsupported(_))
    ));

    Ok(())
}

#[test]
fn accessor_ranges_require_supported_widths() {
    for width in [0_usize, 65, usize::MAX] {
        let call = syn::parse_quote!(self.unit.get_const::<0, #width>());

        assert!(matches!(
            CallCollector::parse_call(&call, "unit".into(), AccessorKind::Getter),
            Err(AdaptationError::Unsupported(_))
        ));
    }
}

#[test]
fn accessors_require_matching_borrows_and_unsigned_scalars() -> Result<(), Box<dyn core::error::Error>> {
    let binding_ast = syn::parse_file("")?;
    let index = AstIndex::new(&binding_ast)?;
    let resolver = LayoutResolver {
        index: &index,
        pointer_width: 64,
    };

    for (method, kind, width) in [
        (
            syn::parse_quote!(
                fn set_flag(&self, value: u8) {}
            ),
            AccessorKind::Setter,
            1,
        ),
        (
            syn::parse_quote!(
                unsafe fn flag(&self) -> u8 {
                    0
                }
            ),
            AccessorKind::Getter,
            1,
        ),
        (
            syn::parse_quote!(
                fn flag(&self) -> i8 {
                    0
                }
            ),
            AccessorKind::Getter,
            1,
        ),
        (
            syn::parse_quote!(
                fn flag(&self) -> u8 {
                    0
                }
            ),
            AccessorKind::Getter,
            9,
        ),
        (
            syn::parse_quote!(
                fn flag<T>(&self) -> u8 {
                    0
                }
            ),
            AccessorKind::Getter,
            1,
        ),
    ] {
        assert!(matches!(
            BitfieldAccessor::validate_signature(&method, width, kind, &resolver),
            Err(AdaptationError::Unsupported(_))
        ));
    }

    Ok(())
}

#[test]
fn alias_cycles_fail_resolution() -> Result<(), Box<dyn core::error::Error>> {
    let binding_ast = syn::parse_file("type First = Second; type Second = First;")?;
    let index = AstIndex::new(&binding_ast)?;
    let resolver = LayoutResolver {
        index: &index,
        pointer_width: 64,
    };
    let ty = syn::parse_quote!(First);

    assert!(matches!(
        resolver.unsigned_integer_bits(&ty),
        Err(AdaptationError::Invalid(_))
    ));

    Ok(())
}
