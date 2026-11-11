//! Translate Bindgen bitfield helpers into integer-backed bitwise views.
//!
//! Accessor signatures and C layout assertions establish the field ranges before
//! helper records are replaced. Unrecognized representations stop generation.

#[cfg(test)]
mod tests;

use alloc::collections::{BTreeMap, BTreeSet};

use syn::{
    Attribute, Expr, ExprArray, ExprBinary, ExprCall, ExprIndex, ExprLit, ExprMacro, ExprMethodCall, ExprPath, Fields,
    GenericArgument, ImplItem, Item, ItemImpl, ItemStruct, ItemUnion, Lit, Member, Meta, Type, TypeArray, TypePath,
    visit_mut::VisitMut,
};

use super::error::AdaptationError;

/// Bindgen's name for its bitfield storage helper that holds a byte array.
const BINDGEN_BITFIELD_UNIT: &str = "__BindgenBitfieldUnit";

/// The target byte order used when mapping C bit offsets onto an integer image.
#[derive(Clone, Copy)]
pub enum Endian {
    /// The lowest addressed byte contains the least significant bits.
    Little,
    /// The lowest addressed byte contains the most significant bits.
    Big,
}

/// The transformed Bindgen file and the types whose Bindgen impls were replaced.
pub struct Translation {
    /// Bindgen declarations with bitfield helper records replaced by raw wrappers.
    pub binding_ast: syn::File,

    /// Original bitfield records, retained in the layout-reference AST only.
    pub removed_record_names: BTreeSet<String>,
}

/// Translate Bindgen bitfield records into `nekor-bitwise` aliases and wrappers.
// NOTE(invariant): `pointer_width` and `endian` come from the target triple passed to Bindgen.
pub struct Adapter<'binding> {
    /// Bindgen output to translate.
    binding_ast: &'binding syn::File,

    /// Target pointer width in bits.
    pointer_width: u32,

    /// Target byte order.
    endian: Endian,
}

impl<'binding> Adapter<'binding> {
    /// Create a bitfield adapter for the selected target.
    #[inline]
    pub const fn new(binding_ast: &'binding syn::File, pointer_width: u32, endian: Endian) -> Self {
        Self {
            binding_ast,
            pointer_width,
            endian,
        }
    }

    /// Translate every Bindgen bitfield helper in the file.
    ///
    /// # Failure
    ///
    /// Rejects unsupported declarations, conflicting accessors, and unproven layouts.
    pub fn apply(self) -> Result<Translation, AdaptationError> {
        let Self {
            binding_ast,
            pointer_width,
            endian,
        } = self;

        let index = AstIndex::new(binding_ast)?;
        let layout_resolver = LayoutResolver {
            index: &index,
            pointer_width,
        };
        let record_translation_list = Self::records(&index, &layout_resolver)?;
        let mut union_translation_map = BTreeMap::new();
        let mut removed_record_names = BTreeSet::new();

        for record_translation in record_translation_list {
            removed_record_names.insert(record_translation.record_ident.to_string());

            match union_translation_map.insert(record_translation.union_ident.to_string(), record_translation) {
                None => {},
                Some(_) => Err(AdaptationError::Duplicate(
                    "multiple Bindgen bitfield records target the same union".into(),
                ))?,
            }
        }

        let mut translated_binding_ast = binding_ast.clone();
        let declared_type_names = Self::declared_type_names(binding_ast);
        let mut generated_type_names = BTreeSet::new();
        let mut translated_item_list = Vec::with_capacity(translated_binding_ast.items.len());

        for binding_item in translated_binding_ast.items {
            match binding_item {
                Item::Struct(struct_declaration)
                    if removed_record_names.contains(&struct_declaration.ident.to_string()) => {},
                Item::Impl(implementation)
                    if Self::implementation_target(&implementation).is_some_and(|target| {
                        removed_record_names.contains(&target) || target == BINDGEN_BITFIELD_UNIT
                    }) => {},
                Item::Struct(struct_declaration) if struct_declaration.ident == BINDGEN_BITFIELD_UNIT => {},
                Item::Union(union_declaration) => {
                    match union_translation_map.remove(&union_declaration.ident.to_string()) {
                        Some(union_translation) => {
                            let (raw_struct, generated_item_list) = Self::translate_union(
                                union_declaration,
                                union_translation,
                                &declared_type_names,
                                &mut generated_type_names,
                                endian,
                            )?;

                            translated_item_list.push(Item::Struct(raw_struct));
                            translated_item_list.extend(generated_item_list);
                        },
                        None => translated_item_list.push(Item::Union(union_declaration)),
                    }
                },
                binding_item => translated_item_list.push(binding_item),
            }
        }

        if !union_translation_map.is_empty() {
            Err(AdaptationError::Missing(
                "a Bindgen bitfield record refers to a union absent from the file".into(),
            ))?;
        }

        translated_binding_ast.items = translated_item_list;

        if Self::contains_type(&translated_binding_ast, BINDGEN_BITFIELD_UNIT) {
            Err(AdaptationError::Untranslated(
                "a Bindgen bitfield helper remains after translation".into(),
            ))?;
        }

        Ok(Translation {
            binding_ast: translated_binding_ast,
            removed_record_names,
        })
    }

    /// Discover bitfield records and validate their accessors and layouts.
    fn records(
        index: &AstIndex,
        layout_resolver: &LayoutResolver<'_>,
    ) -> Result<Vec<RecordTranslation>, AdaptationError> {
        let accessor_map = Self::accessors(index, layout_resolver)?;
        let mut record_translation_list = Vec::new();

        for record in index.struct_map.values() {
            let unit_field_map = Self::unit_fields(record)?;

            if unit_field_map.is_empty() {
                continue;
            }

            let record_ident = record.ident.clone();
            let accessor_group_map = accessor_map.get(&record_ident.to_string()).ok_or_else(|| {
                AdaptationError::Missing(format!("Bindgen bitfield record `{record_ident}` has no accessors"))
            })?;

            match accessor_group_map.len().cmp(&unit_field_map.len()) {
                core::cmp::Ordering::Equal => {},
                _ => Err(AdaptationError::Conflict(format!(
                    "Bindgen bitfield record `{record_ident}` has unmatched storage and accessor groups"
                )))?,
            }

            let record_layout = Self::byte_record_layout(index, record, &unit_field_map, layout_resolver)?;
            let mut field_accessor_list = Vec::new();

            for (unit_field_name, storage_bytes) in &unit_field_map {
                let accessor_group = accessor_group_map.get(unit_field_name).ok_or_else(|| {
                    AdaptationError::Unsupported(format!(
                        "Bindgen bitfield record `{record_ident}` has an unimplemented storage field"
                    ))
                })?;
                let storage_offset = *record_layout.field_offset_map.get(unit_field_name).ok_or_else(|| {
                    AdaptationError::Missing(format!("missing C layout for `{record_ident}.{unit_field_name}`"))
                })?;

                match accessor_group.as_slice() {
                    [_, ..] => {},
                    _ => Err(AdaptationError::Missing(format!(
                        "Bindgen bitfield storage `{unit_field_name}` has no fields"
                    )))?,
                }

                let storage_bits = storage_bytes.checked_mul(8).ok_or_else(|| {
                    AdaptationError::Overflow(format!(
                        "Bindgen storage size overflow for `{record_ident}.{unit_field_name}`"
                    ))
                })?;
                let mut accessor_range_list = Vec::new();

                for accessor in accessor_group {
                    let field_end = accessor
                        .offset
                        .checked_add(usize::from(accessor.width))
                        .ok_or_else(|| {
                            AdaptationError::Overflow(format!(
                                "Bindgen field range overflow for `{record_ident}::{}`",
                                accessor.name
                            ))
                        })?;

                    match field_end.cmp(&storage_bits) {
                        core::cmp::Ordering::Less | core::cmp::Ordering::Equal => {},
                        _ => Err(AdaptationError::OutOfBounds(format!(
                            "Bindgen accessor `{}` exceeds `{record_ident}.{unit_field_name}`",
                            accessor.name
                        )))?,
                    }

                    accessor_range_list.push((accessor.offset, field_end, accessor.name.to_string()));

                    let name = accessor.name.clone();
                    let storage_bytes = *storage_bytes;

                    field_accessor_list.push(FieldAccessor {
                        name,
                        offset: accessor.offset,
                        width: accessor.width,
                        storage_offset,
                        storage_bytes,
                    });
                }

                accessor_range_list.sort_by_key(|(start, _, _)| *start);

                for range_pair in accessor_range_list.windows(2) {
                    let [(_, previous_end, previous_name), (next_start, _, next_name)] = range_pair else {
                        continue;
                    };

                    match previous_end.cmp(next_start) {
                        core::cmp::Ordering::Less | core::cmp::Ordering::Equal => {},
                        _ => Err(AdaptationError::Overlap(format!(
                            "Bindgen fields `{}` and `{}` overlap in `{record_ident}.{unit_field_name}`",
                            previous_name, next_name
                        )))?,
                    }
                }
            }

            field_accessor_list.sort_by_key(|accessor| (accessor.storage_offset, accessor.offset));

            let union_list = index
                .union_map
                .values()
                .filter(|union| Self::has_field_type(union, &record_ident.to_string()))
                .collect::<Vec<_>>();

            let union = match union_list.as_slice() {
                [union] => *union,
                _ => Err(AdaptationError::Unsupported(format!(
                    "Bindgen bitfield record `{record_ident}` must be the field arm of exactly one union"
                )))?,
            };

            let (raw_field, storage_bits) = Self::integer_union_arm(union, &record_ident, layout_resolver)?;

            let union_alignment =
                Self::validate_union_layout(index, union, record, raw_field, storage_bits, layout_resolver)?;
            let type_prefix = Self::owner_prefix(index, union);

            let union_ident = union.ident.clone();
            let raw_field = raw_field.clone();

            record_translation_list.push(RecordTranslation {
                record_ident,
                union_ident,
                type_prefix,
                raw_field,
                storage_bits,
                union_alignment,
                field_accessor_list,
            });
        }

        Ok(record_translation_list)
    }

    /// Parse safe getter and setter calls from the generated impl AST.
    fn accessors(index: &AstIndex, layout_resolver: &LayoutResolver<'_>) -> Result<AccessorMap, AdaptationError> {
        let mut bitfield_record_name_set = BTreeSet::new();

        for record in index.struct_map.values() {
            match Self::unit_fields(record)?.len() {
                0 => {},
                _ => {
                    bitfield_record_name_set.insert(record.ident.to_string());
                },
            }
        }

        let mut accessor_map = BTreeMap::new();
        let mut ignored_raw_accessor_list = Vec::new();
        let mut constructor_signature_list = Vec::new();

        for implementation in index.binding_ast.items.iter().filter_map(|item| match item {
            Item::Impl(implementation) => Some(implementation),
            _ => None,
        }) {
            let Some(record_name) = Self::implementation_target(implementation) else {
                continue;
            };

            if !bitfield_record_name_set.contains(&record_name) {
                continue;
            }

            match &implementation.trait_ {
                None => {},
                _ => Err(AdaptationError::Unsupported(format!(
                    "Bindgen bitfield record `{record_name}` has an unsupported trait implementation"
                )))?,
            }

            let record = index
                .struct_map
                .get(&record_name)
                .ok_or_else(|| AdaptationError::Missing(format!("missing bitfield record `{record_name}`")))?;
            let unit_field_map = Self::unit_fields(record)?;
            let record_accessor_map = accessor_map.entry(record_name.clone()).or_insert_with(BTreeMap::new);

            for implementation_item in &implementation.items {
                let method = match implementation_item {
                    ImplItem::Fn(method) => method,
                    _ => Err(AdaptationError::Unsupported(format!(
                        "Bindgen bitfield implementation for `{record_name}` has a non-method item"
                    )))?,
                };

                let mut collector = CallCollector::default();
                let mut method_body = method.block.clone();

                collector.visit_block_mut(&mut method_body);

                let CallCollector { call_list } = collector;

                let call = match call_list.as_slice() {
                    [] => match Self::raw_accessor(method, layout_resolver)? {
                        Some((field_name, kind, scalar_bits)) => {
                            ignored_raw_accessor_list.push((record_name.clone(), field_name, kind, scalar_bits));

                            continue;
                        },
                        None => match Self::bitfield_constructor(method, &unit_field_map, layout_resolver)? {
                            Some((unit_name, parameter_list)) => {
                                constructor_signature_list.push((record_name.clone(), unit_name, parameter_list));

                                continue;
                            },
                            None => Err(AdaptationError::Missing(format!(
                                "Bindgen bitfield method `{record_name}::{}` has no recognized storage call",
                                method.sig.ident
                            )))?,
                        },
                    },
                    [_] => {
                        let mut call_list = call_list.into_iter();

                        call_list.next().ok_or_else(|| {
                            AdaptationError::Missing(format!(
                                "Bindgen accessor `{record_name}::{}` lost its parsed call",
                                method.sig.ident
                            ))
                        })??
                    },
                    _ => Err(AdaptationError::Unsupported(format!(
                        "Bindgen accessor `{record_name}::{}` contains multiple bitfield calls",
                        method.sig.ident
                    )))?,
                };

                match unit_field_map.get(&call.unit_field) {
                    Some(_) => {},
                    _ => Err(AdaptationError::Unsupported(format!(
                        "Bindgen accessor `{record_name}::{}` uses unknown storage `{}`",
                        method.sig.ident, call.unit_field
                    )))?,
                }

                let (field_name, kind) = if matches!(call.kind, AccessorKind::Setter) {
                    let field_name = method
                        .sig
                        .ident
                        .to_string()
                        .strip_prefix("set_")
                        .map(str::to_owned)
                        .ok_or_else(|| {
                            AdaptationError::Unsupported(format!(
                                "Bindgen setter `{record_name}::{}` has an unexpected name",
                                method.sig.ident
                            ))
                        })?;

                    (field_name, AccessorKind::Setter)
                } else {
                    (method.sig.ident.to_string(), AccessorKind::Getter)
                };

                let scalar_bits = BitfieldAccessor::validate_signature(method, call.width, kind, layout_resolver)?;
                let group = record_accessor_map.entry(call.unit_field).or_insert_with(BTreeMap::new);
                let pair = group.entry(field_name.clone()).or_insert_with(AccessorPair::default);
                let name = syn::Ident::new(&field_name, method.sig.ident.span());

                let accessor = BitfieldAccessor {
                    name,
                    offset: call.offset,
                    width: call.width,
                    scalar_bits,
                };

                let (accessor_slot, accessor_kind) = match kind {
                    AccessorKind::Setter => (&mut pair.setter, "setter"),
                    AccessorKind::Getter => (&mut pair.getter, "getter"),
                };

                match accessor_slot.replace(accessor) {
                    None => {},
                    _ => Err(AdaptationError::Duplicate(format!(
                        "duplicate Bindgen {accessor_kind} for `{record_name}::{field_name}`"
                    )))?,
                }
            }
        }

        let mut completed_accessor_map = BTreeMap::new();

        for (record_name, unit_map) in accessor_map {
            let mut completed_unit_map = BTreeMap::new();

            for (unit_name, pair_map) in unit_map {
                let mut field_accessor_list = Vec::new();

                for (field_name, pair) in pair_map {
                    let AccessorPair { getter, setter } = pair;

                    let (getter, setter) = match (getter, setter) {
                        (Some(getter), Some(setter)) => (getter, setter),
                        _ => Err(AdaptationError::Missing(format!(
                            "Bindgen bitfield `{record_name}::{field_name}` needs both safe accessors"
                        )))?,
                    };

                    match (getter.offset, getter.width, getter.scalar_bits).cmp(&(
                        setter.offset,
                        setter.width,
                        setter.scalar_bits,
                    )) {
                        core::cmp::Ordering::Equal => {},
                        _ => Err(AdaptationError::Conflict(format!(
                            "Bindgen getter and setter disagree for `{record_name}::{field_name}`"
                        )))?,
                    }

                    field_accessor_list.push(getter);
                }

                completed_unit_map.insert(unit_name, field_accessor_list);
            }

            completed_accessor_map.insert(record_name, completed_unit_map);
        }

        let mut ignored_raw_accessor_name_set = BTreeSet::new();

        for (record_name, field_name, kind, scalar_bits) in ignored_raw_accessor_list {
            if !ignored_raw_accessor_name_set.insert((record_name.clone(), field_name.clone(), kind)) {
                Err(AdaptationError::Duplicate(format!(
                    "duplicate Bindgen raw accessor for `{record_name}::{field_name}`"
                )))?;
            }

            let matching_accessor_list = completed_accessor_map
                .get(&record_name)
                .into_iter()
                .flat_map(BTreeMap::values)
                .flatten()
                .filter(|accessor| accessor.name == field_name)
                .collect::<Vec<_>>();

            match matching_accessor_list.as_slice() {
                [accessor] if accessor.scalar_bits == scalar_bits => {},
                _ => {
                    Err(AdaptationError::Missing(format!(
                        "Bindgen raw accessor `{record_name}::{field_name}` has no matching safe accessor"
                    )))?;
                },
            }
        }

        let mut constructor_name_set = BTreeSet::new();

        for (record_name, unit_name, parameter_list) in constructor_signature_list {
            if !constructor_name_set.insert((record_name.clone(), unit_name.clone())) {
                Err(AdaptationError::Duplicate(format!(
                    "duplicate Bindgen constructor for `{record_name}.{unit_name}`"
                )))?;
            }

            let field_accessor_list = completed_accessor_map
                .get(&record_name)
                .and_then(|unit_map| unit_map.get(&unit_name))
                .ok_or_else(|| {
                    AdaptationError::Unsupported(format!(
                        "Bindgen constructor has unknown storage `{record_name}.{unit_name}`"
                    ))
                })?;

            let parameter_set = parameter_list.iter().cloned().collect::<BTreeSet<_>>();
            let accessor_set = field_accessor_list
                .iter()
                .map(|accessor| (accessor.name.to_string(), accessor.scalar_bits))
                .collect::<BTreeSet<_>>();

            match (parameter_list.len(), parameter_set).cmp(&(field_accessor_list.len(), accessor_set)) {
                core::cmp::Ordering::Equal => {},
                _ => Err(AdaptationError::Conflict(format!(
                    "Bindgen constructor disagrees with accessors for `{record_name}.{unit_name}`"
                )))?,
            }
        }

        Ok(completed_accessor_map)
    }

    /// Read the Bindgen storage members from a named record.
    fn unit_fields(record: &ItemStruct) -> Result<BTreeMap<String, usize>, AdaptationError> {
        let mut unit_field_map = BTreeMap::new();
        let field_list = match &record.fields {
            Fields::Named(fields) => fields.named.iter().collect::<Vec<_>>(),
            Fields::Unnamed(fields) => fields.unnamed.iter().collect::<Vec<_>>(),
            Fields::Unit => Vec::new(),
        };

        for field in field_list {
            match Self::unit_storage_bytes(&field.ty)? {
                Some(storage_bytes) => {
                    let field_name = field
                        .ident
                        .as_ref()
                        .ok_or_else(|| {
                            AdaptationError::Unsupported(format!(
                                "unnamed Bindgen bitfield storage in `{}`",
                                record.ident
                            ))
                        })?
                        .to_string();

                    match unit_field_map.insert(field_name, storage_bytes) {
                        None => {},
                        Some(_) => Err(AdaptationError::Duplicate(format!(
                            "duplicate Bindgen bitfield storage in `{}`",
                            record.ident
                        )))?,
                    }
                },
                None => {},
            }
        }

        Ok(unit_field_map)
    }

    /// Read the fixed byte-array size from a Bindgen bitfield-unit type.
    fn unit_storage_bytes(ty: &Type) -> Result<Option<usize>, AdaptationError> {
        match ty {
            Type::Path(TypePath { qself: None, path, .. }) => match path.segments.last() {
                Some(segment) if segment.ident == BINDGEN_BITFIELD_UNIT => {
                    match (path.segments.len(), &segment.arguments) {
                        (1, syn::PathArguments::AngleBracketed(arguments)) => {
                            let argument_list = arguments.args.iter().collect::<Vec<_>>();

                            match argument_list.as_slice() {
                                [GenericArgument::Type(Type::Array(TypeArray { elem, len, .. }))] => {
                                    match Self::path_type_name(elem).as_deref() {
                                        Some("u8") => Ok(Some(Self::integer_literal(len)?)),
                                        _ => Err(AdaptationError::Unsupported(
                                            "Bindgen bitfield storage element must be `u8`".into(),
                                        )),
                                    }
                                },
                                _ => Err(AdaptationError::Unsupported(
                                    "Bindgen bitfield storage must be a fixed byte array argument".into(),
                                )),
                            }
                        },
                        _ => Err(AdaptationError::Unsupported(
                            "Bindgen bitfield unit must have an unqualified path and one storage argument".into(),
                        )),
                    }
                },
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    /// Recognize Bindgen raw-pointer accessors that the borrowed views replace.
    fn raw_accessor(
        method: &syn::ImplItemFn,
        layout_resolver: &LayoutResolver<'_>,
    ) -> Result<Option<(String, AccessorKind, usize)>, AdaptationError> {
        let method_name = method.sig.ident.to_string();

        match method_name.strip_suffix("_raw") {
            None => Ok(None),
            Some(method_base) => {
                let (field_name, kind) = match method_base.strip_prefix("set_") {
                    Some(field_name) => (field_name, AccessorKind::Setter),
                    None => (method_base, AccessorKind::Getter),
                };

                let parameter_list = method
                    .sig
                    .inputs
                    .iter()
                    .filter_map(|argument| match argument {
                        syn::FnArg::Typed(argument) => Some(argument.ty.as_ref()),
                        syn::FnArg::Receiver(_) => None,
                    })
                    .collect::<Vec<_>>();

                match (
                    field_name,
                    &method.sig.safety,
                    method.sig.receiver(),
                    method.sig.generics.params.len(),
                    &method.sig.generics.where_clause,
                ) {
                    ("", _, _, _, _) => Err(AdaptationError::Unsupported(format!(
                        "Bindgen raw accessor `{method_name}` has an empty field name"
                    ))),
                    (_, syn::Safety::Unsafe(_), None, 0, None) => Ok(()),
                    _ => Err(AdaptationError::Unsupported(format!(
                        "Bindgen raw accessor `{method_name}` has an unsupported signature"
                    ))),
                }?;

                let (pointer_type, value_type) = match (kind, parameter_list.as_slice(), &method.sig.output) {
                    (AccessorKind::Setter, [pointer_type, value_type], syn::ReturnType::Default) => {
                        (*pointer_type, *value_type)
                    },
                    (AccessorKind::Getter, [pointer_type], syn::ReturnType::Type(_, return_type)) => {
                        (*pointer_type, return_type.as_ref())
                    },
                    _ => Err(AdaptationError::Unsupported(format!(
                        "Bindgen raw accessor `{method_name}` has an unsupported signature"
                    )))?,
                };

                if !Self::pointer_to_self(pointer_type, kind) {
                    Err(AdaptationError::Unsupported(format!(
                        "Bindgen raw accessor `{method_name}` has an unsupported pointer"
                    )))?;
                }

                let scalar_bits = layout_resolver.unsigned_integer_bits(value_type)?.ok_or_else(|| {
                    AdaptationError::Unsupported(format!(
                        "Bindgen raw accessor `{method_name}` uses a non-unsigned integer"
                    ))
                })?;
                let field_name = field_name.to_owned();

                Ok(Some((field_name, kind, scalar_bits)))
            },
        }
    }

    /// Recognize Bindgen's safe constructor for a bitfield storage unit.
    fn bitfield_constructor(
        method: &syn::ImplItemFn,
        unit_field_map: &BTreeMap<String, usize>,
        layout_resolver: &LayoutResolver<'_>,
    ) -> Result<Option<(String, Vec<(String, usize)>)>, AdaptationError> {
        let method_name = method.sig.ident.to_string();
        let matching_unit_name_list = unit_field_map
            .keys()
            .filter(|unit_name| method_name == format!("new_{}", unit_name.trim_start_matches('_')))
            .collect::<Vec<_>>();

        match matching_unit_name_list.as_slice() {
            [] => Ok(None),
            [unit_name] => {
                let return_type = match (
                    &method.sig.safety,
                    method.sig.receiver(),
                    method.sig.generics.params.len(),
                    &method.sig.generics.where_clause,
                    &method.sig.output,
                ) {
                    (syn::Safety::Default, None, 0, None, syn::ReturnType::Type(_, return_type)) => return_type,
                    _ => Err(AdaptationError::Unsupported(format!(
                        "Bindgen constructor `{method_name}` has an unsupported signature"
                    )))?,
                };

                match (Self::unit_storage_bytes(return_type)?, unit_field_map.get(*unit_name)) {
                    (Some(actual), Some(expected)) if actual == *expected => {},
                    _ => Err(AdaptationError::Conflict(format!(
                        "Bindgen constructor `{method_name}` returns the wrong storage type"
                    )))?,
                }

                let mut parameter_list = Vec::new();

                for argument in &method.sig.inputs {
                    let argument = match argument {
                        syn::FnArg::Typed(argument) => argument,
                        syn::FnArg::Receiver(_) => Err(AdaptationError::Unsupported(format!(
                            "Bindgen constructor `{method_name}` has an unexpected receiver"
                        )))?,
                    };
                    let parameter_name = match argument.pat.as_ref() {
                        syn::Pat::Ident(parameter_name) => parameter_name.ident.to_string(),
                        _ => Err(AdaptationError::Unsupported(format!(
                            "Bindgen constructor `{method_name}` has an unnamed field parameter"
                        )))?,
                    };
                    let scalar_bits = layout_resolver.unsigned_integer_bits(&argument.ty)?.ok_or_else(|| {
                        AdaptationError::Unsupported(format!(
                            "Bindgen constructor `{method_name}` uses a non-unsigned field"
                        ))
                    })?;

                    parameter_list.push((parameter_name, scalar_bits));
                }

                let unit_name = (*unit_name).clone();

                Ok(Some((unit_name, parameter_list)))
            },
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen constructor `{method_name}` has ambiguous storage"
            ))),
        }
    }

    /// Check the pointer type used by a Bindgen raw accessor.
    fn pointer_to_self(ty: &Type, kind: AccessorKind) -> bool {
        match ty {
            Type::Ptr(pointer) => match (pointer.elem.as_ref(), &pointer.mutability, kind) {
                (
                    Type::Path(TypePath { qself: None, path, .. }),
                    syn::PointerMutability::Mut(_),
                    AccessorKind::Setter,
                )
                | (
                    Type::Path(TypePath { qself: None, path, .. }),
                    syn::PointerMutability::Const(_),
                    AccessorKind::Getter,
                ) => path.is_ident("Self"),
                _ => false,
            },
            _ => false,
        }
    }

    /// Derive a byte-aligned bitfield record layout and compare it with Bindgen's assertions.
    fn byte_record_layout(
        index: &AstIndex,
        record: &ItemStruct,
        unit_field_map: &BTreeMap<String, usize>,
        layout_resolver: &LayoutResolver<'_>,
    ) -> Result<RecordLayout, AdaptationError> {
        let record_name = record.ident.to_string();

        match (
            Self::plain_repr_c(&record.attrs),
            record.generics.params.len(),
            &record.generics.where_clause,
        ) {
            (true, 0, None) => {},
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen bitfield record `{record_name}` must be nongeneric and use plain `repr(C)`"
            )))?,
        }

        let evidence = index.layout_evidence(&record_name)?;
        let fields = match &record.fields {
            Fields::Named(fields) => fields,
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen bitfield record `{record_name}` is not a named struct"
            )))?,
        };

        let ordinary_field_count = fields
            .named
            .iter()
            .filter(|field| {
                field
                    .ident
                    .as_ref()
                    .is_none_or(|field_name| !unit_field_map.contains_key(&field_name.to_string()))
            })
            .count();

        match evidence.offsets.len().cmp(&ordinary_field_count) {
            core::cmp::Ordering::Equal => {},
            _ => Err(AdaptationError::Missing(format!(
                "Bindgen layout offsets are incomplete for `{record_name}`"
            )))?,
        }

        let mut calculated_field_offset_map = BTreeMap::new();
        let mut cursor = 0_usize;

        for field in &fields.named {
            let field_name = field
                .ident
                .as_ref()
                .ok_or_else(|| {
                    AdaptationError::Unsupported(format!("unnamed field in Bindgen bitfield record `{record_name}`"))
                })?
                .to_string();
            let field_size = match unit_field_map.get(&field_name) {
                Some(storage_bytes) => *storage_bytes,
                None => layout_resolver.byte_size(&field.ty).map_err(|error| {
                    AdaptationError::Unsupported(format!(
                        "Bindgen bitfield record `{record_name}` contains non-byte-aligned field `{field_name}` ({error})"
                    ))
                })?,
            };

            calculated_field_offset_map.insert(field_name.clone(), cursor);

            match (unit_field_map.get(&field_name), evidence.offsets.get(&field_name)) {
                (Some(_), None) => {},
                (_, Some(offset)) if *offset == cursor => {},
                _ => Err(AdaptationError::Conflict(format!(
                    "computed byte offset disagrees with Bindgen for `{record_name}.{field_name}`"
                )))?,
            }

            cursor = cursor
                .checked_add(field_size)
                .ok_or_else(|| AdaptationError::Overflow(format!("Bindgen record `{record_name}` size overflow")))?;
        }

        match (evidence.size, evidence.alignment) {
            (Some(size), Some(1)) if size == cursor => {},
            _ => Err(AdaptationError::Conflict(format!(
                "computed byte layout disagrees with Bindgen for `{record_name}`"
            )))?,
        }

        let field_offset_map = calculated_field_offset_map;

        Ok(RecordLayout { field_offset_map })
    }

    /// Find the integer arm of a Bindgen bitfield union with two arms.
    fn integer_union_arm<'union>(
        union: &'union ItemUnion,
        record_ident: &syn::Ident,
        layout_resolver: &LayoutResolver<'_>,
    ) -> Result<(&'union syn::Field, usize), AdaptationError> {
        let arm_list = union.fields.named.iter().collect::<Vec<_>>();
        let record_name = record_ident.to_string();

        match arm_list.as_slice() {
            [first, second] => {
                let integer_arm = match (
                    Self::path_type_name(&first.ty).as_deref(),
                    Self::path_type_name(&second.ty).as_deref(),
                ) {
                    (Some(first_name), Some(second_name))
                        if first_name == record_name && second_name != record_name =>
                    {
                        second
                    },
                    (Some(first_name), Some(second_name))
                        if second_name == record_name && first_name != record_name =>
                    {
                        first
                    },
                    _ => Err(AdaptationError::Unsupported(format!(
                        "Bindgen bitfield union `{}` must pair its record with an integer arm",
                        union.ident
                    )))?,
                };
                let storage_bits = layout_resolver.unsigned_integer_bits(&integer_arm.ty)?.ok_or_else(|| {
                    AdaptationError::Unsupported(format!(
                        "Bindgen bitfield union `{}` has no unsigned integer arm",
                        union.ident
                    ))
                })?;

                Ok((integer_arm, storage_bits))
            },
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen bitfield union `{}` must have exactly two arms",
                union.ident
            ))),
        }
    }

    /// Validate union arms and return Bindgen's expected alignment.
    fn validate_union_layout(
        index: &AstIndex,
        union: &ItemUnion,
        record: &ItemStruct,
        raw_field: &syn::Field,
        storage_bits: usize,
        layout_resolver: &LayoutResolver<'_>,
    ) -> Result<usize, AdaptationError> {
        let union_name = union.ident.to_string();

        if !Self::plain_repr_c(&union.attrs) {
            Err(AdaptationError::Unsupported(format!(
                "Bindgen bitfield union `{union_name}` must use plain `repr(C)`"
            )))?;
        }

        let evidence = index.layout_evidence(&union_name)?;
        let record_name = record.ident.to_string();

        match evidence.offsets.len().cmp(&union.fields.named.len()) {
            core::cmp::Ordering::Equal => {},
            _ => Err(AdaptationError::Missing(format!(
                "Bindgen layout offsets are incomplete for `{union_name}`"
            )))?,
        }

        let union_alignment = evidence.alignment.ok_or_else(|| {
            AdaptationError::Missing(format!("Bindgen omitted alignment evidence for `{union_name}`"))
        })?;

        match (
            storage_bits % 8,
            evidence.size,
            union_alignment,
            layout_resolver.unsigned_integer_bits(&raw_field.ty)?,
        ) {
            (0, Some(size), 1.., Some(raw_bits)) if size == storage_bits / 8 && raw_bits == storage_bits => {},
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen bitfield union `{union_name}` is not integer-backed"
            )))?,
        }

        for field in &union.fields.named {
            let field_name = field
                .ident
                .as_ref()
                .ok_or_else(|| {
                    AdaptationError::Unsupported(format!("unnamed arm in Bindgen bitfield union `{union_name}`"))
                })?
                .to_string();

            match evidence.offsets.get(&field_name) {
                Some(0) => {},
                _ => Err(AdaptationError::Missing(format!(
                    "Bindgen omitted or changed `{union_name}.{field_name}` offset evidence"
                )))?,
            }
        }

        if !Self::has_field_type(union, &record_name) {
            Err(AdaptationError::Untranslated(format!(
                "Bindgen union `{union_name}` lost record arm `{record_name}`"
            )))?;
        }

        Ok(union_alignment)
    }

    /// Map a Bindgen bit offset to the corresponding integer bit index.
    fn integer_offset(
        endian: Endian,
        integer_bits: usize,
        storage_offset: usize,
        storage_bytes: usize,
        field_offset: usize,
        field_width: u8,
    ) -> Result<usize, AdaptationError> {
        let storage_bits = storage_bytes
            .checked_mul(8)
            .ok_or_else(|| AdaptationError::Overflow("Bindgen bitfield storage size overflow".to_owned()))?;
        let field_end = field_offset
            .checked_add(usize::from(field_width))
            .ok_or_else(|| AdaptationError::Overflow("Bindgen bitfield range overflow".to_owned()))?;

        let integer_offset = match endian {
            Endian::Little => storage_offset
                .checked_mul(8)
                .and_then(|base| base.checked_add(field_offset)),
            Endian::Big => storage_offset
                .checked_add(storage_bytes)
                .and_then(|end| end.checked_mul(8))
                .and_then(|storage_end| integer_bits.checked_sub(storage_end))
                .and_then(|base| {
                    storage_bits
                        .checked_sub(field_end)
                        .and_then(|local| base.checked_add(local))
                }),
        }
        .ok_or_else(|| {
            AdaptationError::OutOfBounds("Bindgen bitfield lies outside its integer union arm".to_owned())
        })?;

        let integer_end = integer_offset
            .checked_add(usize::from(field_width))
            .ok_or_else(|| AdaptationError::Overflow("Bindgen bitfield integer range overflow".to_owned()))?;

        match integer_end.cmp(&integer_bits) {
            core::cmp::Ordering::Less | core::cmp::Ordering::Equal => {},
            _ => Err(AdaptationError::OutOfBounds(
                "Bindgen bitfield exceeds its integer union arm".into(),
            ))?,
        }

        Ok(integer_offset)
    }

    /// Emit a raw wrapper and its borrowed field aliases for a translated union.
    fn translate_union(
        union: ItemUnion,
        translation: RecordTranslation,
        declared_type_names: &BTreeSet<String>,
        generated_type_names: &mut BTreeSet<String>,
        endian: Endian,
    ) -> Result<(ItemStruct, Vec<Item>), AdaptationError> {
        let union_name = union.ident.to_string();
        let type_prefix = translation.type_prefix.clone();
        let raw_alias = syn::Ident::new(&format!("Raw{type_prefix}"), union.ident.span());
        let raw_field_name = translation.raw_field.ident.as_ref().ok_or_else(|| {
            AdaptationError::Unsupported(format!("Bindgen union `{union_name}` has an unnamed integer arm"))
        })?;
        let raw_field_type = &translation.raw_field.ty;
        let storage_bits = translation.storage_bits;
        let storage_bytes = storage_bits / 8;
        let union_alignment = translation.union_alignment;
        let union_ident = union.ident.clone();
        let union_generics = union.generics.clone();
        let union_visibility = union.vis.clone();
        let mut raw_field = translation.raw_field.clone();
        let mut union_attribute_list = union
            .attrs
            .iter()
            .filter(|attribute| !attribute.path().is_ident("repr"))
            .cloned()
            .collect::<Vec<_>>();

        match (
            union_generics.params.len(),
            &union_generics.where_clause,
            &union_visibility,
        ) {
            (0, None, syn::Visibility::Public(_)) => {},
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen bitfield union `{union_name}` must be public and nongeneric"
            )))?,
        }

        for attribute in &union.attrs {
            match attribute.path().get_ident().map(syn::Ident::to_string).as_deref() {
                Some("repr" | "derive" | "doc") => {},
                _ => Err(AdaptationError::Unsupported(format!(
                    "Bindgen bitfield union `{union_name}` has unsupported attributes"
                )))?,
            }
        }

        for attribute in &raw_field.attrs {
            match attribute.path().get_ident().map(syn::Ident::to_string).as_deref() {
                Some("doc") => {},
                _ => Err(AdaptationError::Unsupported(format!(
                    "Bindgen integer arm in `{union_name}` has unsupported attributes"
                )))?,
            }
        }

        raw_field.vis = syn::Visibility::Inherited;

        let endian_name = match endian {
            Endian::Little => "little-endian",
            Endian::Big => "big-endian",
        };
        let raw_struct_doc = format!(
            "A raw integer image for the KVM bitfield union.\n\n\
             The sole `{raw_field_name}` field begins at byte offset 0. The image is\n\
             {storage_bytes} bytes with {union_alignment}-byte alignment. Field bit\n\
             positions use the target's {endian_name} mapping derived from Bindgen's\n\
             accessor offsets.\n\n\
             The field fills the image, so no tail padding is present. Every integer\n\
             pattern remains representable. Borrowed field views do not validate KVM policy."
        );

        for line in raw_struct_doc.lines() {
            let line = format!(" {line}");

            union_attribute_list.push(syn::parse_quote!(#[doc = #line]));
        }

        union_attribute_list.push(syn::parse_quote!(#[repr(transparent)]));

        let raw_struct: ItemStruct = syn::parse_quote! {
            #(#union_attribute_list)*
            #union_visibility struct #union_ident {
                #raw_field
            }
        };
        let mut generated_item_list = Vec::new();

        Self::insert_generated_type_name(raw_alias.to_string(), declared_type_names, generated_type_names)?;
        generated_item_list.push(syn::parse_quote! {
            #[doc = concat!("Raw integer-backed image for [`", stringify!(#union_ident), "`].")]
            pub type #raw_alias = #union_ident;
        });

        let mut method_item_list: Vec<ImplItem> = Vec::new();
        let mut generated_method_name_set =
            BTreeSet::from(["new".to_owned(), "bits".to_owned(), "bits_mut".to_owned()]);

        for accessor in translation.field_accessor_list {
            let alias = syn::Ident::new(
                &format!("{type_prefix}{}", Self::pascal_case(&accessor.name.to_string())),
                accessor.name.span(),
            );
            let mutable_alias = syn::Ident::new(&format!("{alias}Mut"), accessor.name.span());
            let method_name = accessor.name;
            let mutable_method_name = syn::Ident::new(&format!("{}_mut", method_name), method_name.span());
            let field_offset = Self::integer_offset(
                endian,
                storage_bits,
                accessor.storage_offset,
                accessor.storage_bytes,
                accessor.offset,
                accessor.width,
            )?;
            let field_end = field_offset
                .checked_add(usize::from(accessor.width))
                .and_then(|end| end.checked_sub(1))
                .ok_or_else(|| AdaptationError::Invalid(format!("invalid Bindgen width for `{method_name}`")))?;
            let raw_type = raw_field_type;
            let accessor_doc = if accessor.width == 1 {
                format!(" Borrow the `{method_name}` bit view.")
            } else {
                format!(" Borrow the `{method_name}` field view.")
            };
            let mutable_accessor_doc = if accessor.width == 1 {
                format!(" Mutably borrow the `{method_name}` bit view.")
            } else {
                format!(" Mutably borrow the `{method_name}` field view.")
            };

            Self::insert_generated_type_name(alias.to_string(), declared_type_names, generated_type_names)?;
            Self::insert_generated_type_name(mutable_alias.to_string(), declared_type_names, generated_type_names)?;

            for name in [method_name.to_string(), mutable_method_name.to_string()] {
                if !generated_method_name_set.insert(name) {
                    Err(AdaptationError::Conflict(format!(
                        "translated Bindgen accessor `{method_name}` conflicts with wrapper methods"
                    )))?;
                }
            }

            if accessor.width == 1 {
                let bit_index = field_offset;

                generated_item_list.push(syn::parse_quote! {
                    #[doc = concat!("Borrowed bitwise view of the `", stringify!(#method_name), "` bit.")]
                    pub type #alias<'value> = ::nekor_bitwise::prelude::Bit<'value, #raw_type, #bit_index>;
                });
            } else {
                generated_item_list.push(syn::parse_quote! {
                    #[doc = concat!("Borrowed bitwise view of the `", stringify!(#method_name), "` field.")]
                    pub type #alias<'value> = ::nekor_bitwise::prelude::Field<'value, #field_offset, #field_end, #raw_type>;
                });
            }

            generated_item_list.push(syn::parse_quote! {
                #[doc = concat!("Mutable counterpart to [`", stringify!(#alias), "`].")]
                pub type #mutable_alias<'value> = <#alias<'value> as ::nekor_bitwise::prelude::Counterpart>::Mut;
            });

            method_item_list.push(syn::parse_quote! {
                #[doc = #accessor_doc]
                #[inline]
                #[must_use]
                pub const fn #method_name(&self) -> #alias<'_> {
                    let Self { #raw_field_name } = self;

                    #alias::wrap(#raw_field_name)
                }
            });
            method_item_list.push(syn::parse_quote! {
                #[doc = #mutable_accessor_doc]
                #[inline]
                pub const fn #mutable_method_name(&mut self) -> #mutable_alias<'_> {
                    let Self { #raw_field_name } = self;

                    #mutable_alias::wrap(#raw_field_name)
                }
            });
        }

        let raw_field_name = raw_field_name.clone();
        let raw_field_type = raw_field_type.clone();
        let wrapper_impl: ItemImpl = syn::parse_quote! {
            impl #union_ident {
                /// Construct the raw integer-backed bitfield image.
                #[inline]
                #[must_use]
                pub const fn new(target_value: #raw_field_type) -> Self {
                    let #raw_field_name = target_value;

                    Self { #raw_field_name }
                }

                /// Borrow the raw integer image.
                #[inline]
                #[must_use]
                pub const fn bits(&self) -> &#raw_field_type {
                    let Self { #raw_field_name } = self;

                    #raw_field_name
                }

                /// Mutably borrow the raw integer image.
                #[inline]
                pub const fn bits_mut(&mut self) -> &mut #raw_field_type {
                    let Self { #raw_field_name } = self;

                    #raw_field_name
                }

                #(#method_item_list)*
            }
        };

        generated_item_list.push(Item::Impl(wrapper_impl));

        Ok((raw_struct, generated_item_list))
    }

    /// Reserve a generated public type name, rejecting collisions with Bindgen output.
    fn insert_generated_type_name(
        name: String,
        declared_type_names: &BTreeSet<String>,
        generated_type_names: &mut BTreeSet<String>,
    ) -> Result<(), AdaptationError> {
        match declared_type_names.get(&name) {
            Some(_) => Err(AdaptationError::Conflict(format!(
                "generated bitwise type name `{name}` conflicts with another declaration"
            ))),
            None => match generated_type_names.insert(name.clone()) {
                true => Ok(()),
                false => Err(AdaptationError::Conflict(format!(
                    "generated bitwise type name `{name}` conflicts with another declaration"
                ))),
            },
        }
    }

    /// Collect existing named types before generating aliases.
    fn declared_type_names(binding_ast: &syn::File) -> BTreeSet<String> {
        binding_ast
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Enum(item) => Some(item.ident.to_string()),
                Item::Struct(item) => Some(item.ident.to_string()),
                Item::Type(item) => Some(item.ident.to_string()),
                Item::Union(item) => Some(item.ident.to_string()),
                _ => None,
            })
            .collect()
    }

    /// Check whether an integer union contains a record arm of the given type.
    fn has_field_type(union: &ItemUnion, type_name: &str) -> bool {
        union
            .fields
            .named
            .iter()
            .any(|field| Self::path_type_name(&field.ty).as_deref() == Some(type_name))
    }

    /// Name generated wrappers from their containing UAPI record and member when available.
    fn owner_prefix(index: &AstIndex, union: &ItemUnion) -> String {
        let union_name = union.ident.to_string();
        let owner_prefix_set = index
            .struct_map
            .values()
            .flat_map(|record| {
                let Fields::Named(fields) = &record.fields else {
                    return Vec::new();
                };

                fields
                    .named
                    .iter()
                    .filter_map(|field| {
                        let field_name = field.ident.as_ref()?;

                        Self::type_contains(&field.ty, &union_name).then(|| {
                            format!(
                                "{}{}",
                                Self::pascal_case(&record.ident.to_string()),
                                Self::pascal_case(&field_name.to_string())
                            )
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeSet<_>>();

        let mut owner_prefix_list = owner_prefix_set.into_iter().collect::<Vec<_>>();

        owner_prefix_list.sort();

        match owner_prefix_list.as_slice() {
            [owner_prefix] => owner_prefix.clone(),
            _ => Self::pascal_case(&union_name),
        }
    }

    /// Check whether a field type names a target directly or through an array.
    fn type_contains(ty: &Type, target_name: &str) -> bool {
        match ty {
            Type::Array(array) => Self::type_contains(&array.elem, target_name),
            Type::Group(group) => Self::type_contains(&group.elem, target_name),
            Type::Paren(paren) => Self::type_contains(&paren.elem, target_name),
            Type::Path(_) => Self::path_type_name(ty).as_deref() == Some(target_name),
            _ => false,
        }
    }

    /// Read the simple path name of a type.
    fn path_type_name(ty: &Type) -> Option<String> {
        match ty {
            Type::Path(TypePath { qself: None, path, .. }) => {
                path.segments.last().map(|segment| segment.ident.to_string())
            },
            _ => None,
        }
    }

    /// Read an integer literal from a generated type expression.
    fn integer_literal(expr: &Expr) -> Result<usize, AdaptationError> {
        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Int(literal), ..
            }) => literal
                .base10_parse()
                .map_err(|error| AdaptationError::Invalid(format!("invalid Bindgen integer literal: {error}"))),
            _ => Err(AdaptationError::Unsupported(
                "Bindgen bitfield constant is not an integer literal".into(),
            )),
        }
    }

    /// Require the exact C representation emitted by Bindgen.
    fn plain_repr_c(attribute_list: &[Attribute]) -> bool {
        let repr_attribute_list = attribute_list
            .iter()
            .filter(|attribute| attribute.path().is_ident("repr"))
            .collect::<Vec<_>>();

        match repr_attribute_list.as_slice() {
            [
                Attribute {
                    meta: Meta::List(repr), ..
                },
            ] => repr.tokens.to_string() == "C",
            _ => false,
        }
    }

    /// Convert a Bindgen identifier into a collision-resistant PascalCase prefix.
    fn pascal_case(name: &str) -> String {
        name.split('_')
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut character_list = part.chars();

                match character_list.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + character_list.as_str(),
                    None => String::new(),
                }
            })
            .collect()
    }

    /// Resolve the target type name of an inherent implementation.
    fn implementation_target(implementation: &ItemImpl) -> Option<String> {
        Self::path_type_name(&implementation.self_ty)
    }

    /// Find any remaining reference to a helper type in the translated AST.
    fn contains_type(binding_ast: &syn::File, type_name: &str) -> bool {
        let mut scan = TypeNameScan {
            type_name,
            found: false,
        };
        let mut cloned_binding_ast = binding_ast.clone();

        scan.visit_file_mut(&mut cloned_binding_ast);

        scan.found
    }
}

impl VisitMut for TypeNameScan<'_> {
    fn visit_type_path_mut(&mut self, type_path: &mut TypePath) {
        let Self { type_name, found } = self;

        if type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == *type_name)
        {
            *found = true;
        }

        syn::visit_mut::visit_type_path_mut(self, type_path);
    }
}

/// AST indexes and Bindgen's declared C layout evidence.
// NOTE(invariant): Declaration maps contain unique Bindgen type names and layout evidence parsed
// from this AST.
struct AstIndex<'binding> {
    /// Original generated file.
    binding_ast: &'binding syn::File,

    /// Type aliases keyed by their Rust identifier.
    alias_map: BTreeMap<String, Type>,

    /// Named struct declarations keyed by their Rust identifier.
    struct_map: BTreeMap<String, ItemStruct>,

    /// Named union declarations keyed by their Rust identifier.
    union_map: BTreeMap<String, ItemUnion>,

    /// Layout assertions emitted by Bindgen.
    layout_map: BTreeMap<String, LayoutEvidence>,
}

impl<'binding> AstIndex<'binding> {
    /// Index top-level declarations and parse their layout assertions.
    fn new(binding_ast: &'binding syn::File) -> Result<Self, AdaptationError> {
        let mut alias_map = BTreeMap::new();
        let mut struct_map = BTreeMap::new();
        let mut union_map = BTreeMap::new();
        let mut type_name_set = BTreeSet::new();

        for item in &binding_ast.items {
            let type_name = match item {
                Item::Enum(item) => Some(item.ident.to_string()),
                Item::Struct(item) => Some(item.ident.to_string()),
                Item::Type(item) => Some(item.ident.to_string()),
                Item::Union(item) => Some(item.ident.to_string()),
                _ => None,
            };

            match type_name {
                Some(type_name) => match type_name_set.insert(type_name.clone()) {
                    true => {},
                    false => Err(AdaptationError::Duplicate(format!(
                        "Bindgen AST contains duplicate type declaration `{type_name}`"
                    )))?,
                },
                None => {},
            }

            match item {
                Item::Type(item) => {
                    alias_map.insert(item.ident.to_string(), *item.ty.clone());
                },
                Item::Struct(item) => {
                    struct_map.insert(item.ident.to_string(), item.clone());
                },
                Item::Union(item) => {
                    union_map.insert(item.ident.to_string(), item.clone());
                },
                _ => {},
            }
        }

        let layout_map = Self::layout_assertions(binding_ast)?;

        Ok(Self {
            binding_ast,
            alias_map,
            struct_map,
            union_map,
            layout_map,
        })
    }

    /// Return complete size and alignment evidence for a type.
    fn layout_evidence(&self, type_name: &str) -> Result<&LayoutEvidence, AdaptationError> {
        let Self { layout_map, .. } = self;

        match layout_map.get(type_name) {
            Some(
                evidence @ LayoutEvidence {
                    size: Some(_),
                    alignment: Some(_),
                    ..
                },
            ) => Ok(evidence),
            Some(_) => Err(AdaptationError::Missing(format!(
                "Bindgen layout evidence for `{type_name}` is incomplete"
            ))),
            None => Err(AdaptationError::Missing(format!(
                "Bindgen emitted no layout assertions for `{type_name}`"
            ))),
        }
    }

    /// Extract expected size, alignment, and named offsets from Bindgen assertions.
    fn layout_assertions(binding_ast: &syn::File) -> Result<BTreeMap<String, LayoutEvidence>, AdaptationError> {
        let mut layout_map = BTreeMap::new();

        for item in &binding_ast.items {
            let Item::Const(constant) = item else {
                continue;
            };

            if constant.ident != "_" {
                continue;
            }

            let Expr::Block(block) = constant.expr.as_ref() else {
                continue;
            };

            for statement in &block.block.stmts {
                let syn::Stmt::Expr(expression, _) = statement else {
                    continue;
                };

                let Some((label, expected_value)) = Self::layout_assertion(expression)? else {
                    continue;
                };

                let (type_name, assertion_kind) = Self::layout_label(&label)?;

                let evidence = layout_map.entry(type_name).or_insert_with(LayoutEvidence::default);

                match assertion_kind {
                    LayoutAssertion::Size => Self::set_evidence(&mut evidence.size, expected_value, &label)?,
                    LayoutAssertion::Alignment => {
                        Self::set_evidence(&mut evidence.alignment, expected_value, &label)?;
                    },
                    LayoutAssertion::Offset(member_name) => {
                        let previous_value = evidence.offsets.insert(member_name.to_owned(), expected_value);

                        match previous_value {
                            Some(previous) if previous != expected_value => Err(AdaptationError::Conflict(format!(
                                "Bindgen emitted conflicting layout assertions for `{label}`"
                            )))?,
                            _ => {},
                        }
                    },
                }
            }
        }

        Ok(layout_map)
    }

    /// Recognize a Bindgen string-indexed compile-time layout assertion.
    fn layout_assertion(expression: &Expr) -> Result<Option<(String, usize)>, AdaptationError> {
        match Self::unparen_expr(expression) {
            Expr::Index(ExprIndex { expr, index, .. }) => match Self::unparen_expr(expr) {
                Expr::Array(ExprArray { elems, .. }) => {
                    let element_list = elems.iter().collect::<Vec<_>>();

                    match element_list.as_slice() {
                        [element] => match Self::unparen_expr(element) {
                            Expr::Lit(ExprLit {
                                lit: Lit::Str(label), ..
                            }) => {
                                let label = label.value();

                                match Self::unparen_expr(index) {
                                    Expr::Binary(ExprBinary {
                                        left,
                                        op: syn::BinOp::Sub(_),
                                        right,
                                        ..
                                    }) => {
                                        let expected_value = Adapter::integer_literal(right)?;

                                        Self::validate_layout_expression(&label, left)?;

                                        Ok(Some((label, expected_value)))
                                    },
                                    _ => Err(AdaptationError::Unsupported(format!(
                                        "Bindgen layout assertion `{label}` has an unsupported expression"
                                    ))),
                                }
                            },
                            _ => Ok(None),
                        },
                        _ => Ok(None),
                    }
                },
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    /// Validate that the assertion expression agrees with its diagnostic label.
    fn validate_layout_expression(label: &str, expression: &Expr) -> Result<(), AdaptationError> {
        let (type_name, assertion_kind) = Self::layout_label(label)?;

        match (assertion_kind, Self::unparen_expr(expression)) {
            (LayoutAssertion::Size, Expr::Call(call)) => Self::validate_layout_call(call, "size_of", &type_name),
            (LayoutAssertion::Alignment, Expr::Call(call)) => Self::validate_layout_call(call, "align_of", &type_name),
            (LayoutAssertion::Offset(member_name), Expr::Macro(ExprMacro { mac, .. })) => {
                match mac.path.segments.last() {
                    Some(segment) if segment.ident == "offset_of" => {},
                    _ => Err(AdaptationError::Unsupported(format!(
                        "Bindgen offset assertion `{label}` uses an unexpected macro"
                    )))?,
                }

                let OffsetMacroArguments {
                    type_name: actual_type,
                    member_name: actual_member,
                } = syn::parse2(mac.tokens.clone()).map_err(|error| {
                    AdaptationError::Invalid(format!("cannot parse Bindgen offset assertion `{label}`: {error}"))
                })?;

                match (actual_type, actual_member).cmp(&(type_name, member_name)) {
                    core::cmp::Ordering::Equal => Ok(()),
                    _ => Err(AdaptationError::Conflict(format!(
                        "Bindgen offset assertion `{label}` disagrees with its expression"
                    ))),
                }
            },
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen layout assertion `{label}` uses an unsupported expression"
            ))),
        }
    }

    /// Validate a `size_of` or `align_of` call in a Bindgen layout assertion.
    fn validate_layout_call(
        call: &ExprCall,
        expected_function: &str,
        expected_type: &str,
    ) -> Result<(), AdaptationError> {
        let function_path = match (call.func.as_ref(), call.args.len()) {
            (Expr::Path(ExprPath { qself: None, path, .. }), 0) => path,
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen layout assertion must call `{expected_function}` without value arguments"
            )))?,
        };

        match function_path.segments.last() {
            Some(syn::PathSegment {
                ident,
                arguments: syn::PathArguments::AngleBracketed(arguments),
            }) if ident == expected_function => {
                let argument_list = arguments.args.iter().collect::<Vec<_>>();

                match argument_list.as_slice() {
                    [GenericArgument::Type(ty)] => match Adapter::path_type_name(ty) {
                        Some(actual_type) if actual_type == expected_type => Ok(()),
                        _ => Err(AdaptationError::Conflict(format!(
                            "Bindgen `{expected_function}` assertion label and type disagree"
                        ))),
                    },
                    _ => Err(AdaptationError::Invalid(format!(
                        "Bindgen `{expected_function}` assertion must have exactly one type argument"
                    ))),
                }
            },
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen layout assertion must call `{expected_function}` with a type argument"
            ))),
        }
    }

    /// Split a Bindgen layout assertion label into a type and assertion kind.
    fn layout_label(label: &str) -> Result<(String, LayoutAssertion), AdaptationError> {
        match (
            label.strip_prefix("Size of "),
            label.strip_prefix("Alignment of "),
            label.strip_prefix("Offset of field: "),
        ) {
            (Some(type_name), None, None) => Ok((type_name.to_owned(), LayoutAssertion::Size)),
            (None, Some(type_name), None) => Ok((type_name.to_owned(), LayoutAssertion::Alignment)),
            (None, None, Some(field_path)) => match field_path.rsplit_once("::") {
                Some((type_name, member_name)) => {
                    Ok((type_name.to_owned(), LayoutAssertion::Offset(member_name.to_owned())))
                },
                None => Err(AdaptationError::Invalid(format!(
                    "invalid Bindgen offset label `{label}`"
                ))),
            },
            _ => Err(AdaptationError::Unsupported(format!(
                "unknown Bindgen layout assertion label `{label}`"
            ))),
        }
    }

    /// Record a layout value and reject conflicting duplicate assertions.
    fn set_evidence(slot: &mut Option<usize>, value: usize, label: &str) -> Result<(), AdaptationError> {
        match *slot {
            Some(previous) if previous != value => Err(AdaptationError::Conflict(format!(
                "Bindgen emitted conflicting layout assertions for `{label}`"
            ))),
            _ => {
                *slot = Some(value);

                Ok(())
            },
        }
    }

    /// Remove grouping parentheses around a parsed expression.
    fn unparen_expr(mut expression: &Expr) -> &Expr {
        loop {
            match expression {
                Expr::Group(group) => expression = &group.expr,
                Expr::Paren(paren) => expression = &paren.expr,
                _ => return expression,
            }
        }
    }
}

/// Parsed layout values emitted by Bindgen for a record.
#[derive(Default)]
// NOTE(invariant): None.
struct LayoutEvidence {
    /// Expected C size in bytes.
    size: Option<usize>,

    /// Expected C alignment in bytes.
    alignment: Option<usize>,

    /// Expected C byte offsets for named members.
    offsets: BTreeMap<String, usize>,
}

/// The kind of a Bindgen size, alignment, or field-offset assertion.
enum LayoutAssertion {
    /// Record size in bytes.
    Size,

    /// Record alignment in bytes.
    Alignment,

    /// Named field byte offset.
    Offset(String),
}

/// `offset_of!(Type, member)` arguments emitted by Bindgen.
// NOTE(invariant): None.
struct OffsetMacroArguments {
    /// First macro argument.
    type_name: String,

    /// Second macro argument.
    member_name: String,
}

impl syn::parse::Parse for OffsetMacroArguments {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let type_name: syn::Ident = input.parse()?;

        input.parse::<syn::Token![,]>()?;

        let member_name: syn::Ident = input.parse()?;

        let type_name = type_name.to_string();
        let member_name = member_name.to_string();

        Ok(Self { type_name, member_name })
    }
}

/// Integer type information resolved through generated aliases.
#[derive(Clone, Copy)]
// NOTE(invariant): `bits` is a primitive integer width and `is_unsigned` matches the resolved type.
struct IntegerInfo {
    /// Integer width in bits.
    bits: usize,

    /// Whether the type is unsigned.
    is_unsigned: bool,
}

/// Bindgen type alias resolution for the integer and byte fields being translated.
// NOTE(invariant): `pointer_width` belongs to the target that produced the indexed Bindgen AST.
struct LayoutResolver<'index> {
    /// AST declaration index.
    index: &'index AstIndex<'index>,

    /// Target pointer width in bits.
    pointer_width: u32,
}

impl<'index> LayoutResolver<'index> {
    /// Resolve the width of an unsigned integer type.
    fn unsigned_integer_bits(&self, ty: &Type) -> Result<Option<usize>, AdaptationError> {
        match self.integer_info(ty, &mut BTreeSet::new())? {
            Some(IntegerInfo {
                bits,
                is_unsigned: true,
            }) => Ok(Some(bits)),
            Some(IntegerInfo { is_unsigned: false, .. }) | None => Ok(None),
        }
    }

    /// Resolve the width of any integer type.
    fn integer_bits(&self, ty: &Type) -> Result<Option<usize>, AdaptationError> {
        Ok(self.integer_info(ty, &mut BTreeSet::new())?.map(|info| info.bits))
    }

    /// Resolve a byte-aligned scalar or fixed array of bytes to its size.
    fn byte_size(&self, ty: &Type) -> Result<usize, AdaptationError> {
        match ty {
            Type::Array(array) => {
                let element_size = self.byte_size(&array.elem)?;
                let element_count = Adapter::integer_literal(&array.len)?;

                element_size
                    .checked_mul(element_count)
                    .ok_or_else(|| AdaptationError::Overflow("Bindgen byte-array size overflow".to_owned()))
            },
            Type::Path(_) => match self.integer_bits(ty)? {
                Some(8) => Ok(1),
                _ => Err(AdaptationError::Unsupported("type is not a byte-sized integer".into())),
            },
            _ => Err(AdaptationError::Unsupported(
                "type is not a byte-aligned integer or array".into(),
            )),
        }
    }

    /// Resolve primitive and aliased integer types without evaluating Rust code.
    fn integer_info(
        &self,
        ty: &Type,
        alias_stack: &mut BTreeSet<String>,
    ) -> Result<Option<IntegerInfo>, AdaptationError> {
        let Self { index, pointer_width } = self;

        match ty {
            Type::Path(TypePath { qself: None, path, .. }) => match path.segments.last() {
                Some(
                    segment @ syn::PathSegment {
                        arguments: syn::PathArguments::None,
                        ..
                    },
                ) => {
                    let type_name = segment.ident.to_string();

                    match (
                        Self::primitive_integer_info(&type_name, *pointer_width),
                        index.alias_map.get(&type_name),
                    ) {
                        (Some(info), _) => Ok(Some(info)),
                        (None, Some(alias_type)) => match alias_stack.insert(type_name.clone()) {
                            true => {
                                let alias_info = self.integer_info(alias_type, alias_stack)?;

                                alias_stack.remove(&type_name);

                                Ok(alias_info)
                            },
                            false => Err(AdaptationError::Invalid(format!(
                                "cyclic Bindgen type alias involving `{type_name}`"
                            ))),
                        },
                        (None, None) => Ok(None),
                    }
                },
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    /// Resolve integer widths for Rust and C scalar typenames.
    fn primitive_integer_info(type_name: &str, pointer_width: u32) -> Option<IntegerInfo> {
        let (bits, is_unsigned) = match type_name {
            "u8" | "c_uchar" => (8, true),
            "i8" | "c_schar" | "c_char" => (8, false),
            "u16" | "c_ushort" => (16, true),
            "i16" | "c_short" => (16, false),
            "u32" | "c_uint" => (32, true),
            "i32" | "c_int" => (32, false),
            "u64" | "c_ulonglong" => (64, true),
            "i64" | "c_longlong" => (64, false),
            "u128" => (128, true),
            "i128" => (128, false),
            "usize" | "c_ulong" => (pointer_width as usize, true),
            "isize" | "c_long" => (pointer_width as usize, false),
            _ => return None,
        };

        Some(IntegerInfo { bits, is_unsigned })
    }
}

/// C layout for a byte-aligned Bindgen helper record.
// NOTE(invariant): Computed field offsets agree with Bindgen's size, alignment, and reported member
// offsets.
struct RecordLayout {
    /// Byte offsets of named fields.
    field_offset_map: BTreeMap<String, usize>,
}

/// Getter metadata extracted from a Bindgen implementation.
#[derive(Clone)]
// NOTE(invariant): `offset` and `width` come from Bindgen, and the unsigned scalar type can
// represent the width.
struct BitfieldAccessor {
    /// Rust getter name and resulting field suffix.
    name: syn::Ident,

    /// Bindgen-relative bit offset within the helper unit.
    offset: usize,

    /// Bindgen field width in bits.
    width: u8,

    /// Declared scalar width returned or accepted by Bindgen.
    scalar_bits: usize,
}

/// A completed accessor with its containing byte storage location.
// NOTE(invariant): The accessor range is within its storage unit and offsets follow the validated
// record layout.
struct FieldAccessor {
    /// Rust getter name.
    name: syn::Ident,

    /// Bindgen-relative bit offset within its helper unit.
    offset: usize,

    /// Width in bits.
    width: u8,

    /// Byte offset of the helper unit within the field record.
    storage_offset: usize,

    /// Size of the helper unit in bytes.
    storage_bytes: usize,
}

/// One bitfield getter/setter pair under a storage member.
#[derive(Default)]
// NOTE(invariant): None.
struct AccessorPair {
    /// Getter metadata.
    getter: Option<BitfieldAccessor>,

    /// Setter metadata.
    setter: Option<BitfieldAccessor>,
}

/// The signature and receiver mutability required by a bitfield operation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AccessorKind {
    /// Borrow the image and return the field's scalar value.
    Getter,

    /// Mutably borrow the image and accept the field's scalar value.
    Setter,
}

/// Bindgen storage calls parsed from an accessor method.
// NOTE(invariant): `width` is between 1 and 64 bits, as required by Bindgen's scalar accessors.
struct BitfieldCall {
    /// Receiver storage field name.
    unit_field: String,

    /// Bit offset within that storage field.
    offset: usize,

    /// Width in bits.
    width: u8,

    /// Operation performed on the storage unit.
    kind: AccessorKind,
}

/// A method visitor that collects safe calls on `self`'s bitfield units.
#[derive(Default)]
// NOTE(invariant): None.
struct CallCollector {
    /// Parsed calls or errors encountered in the method body.
    call_list: Vec<Result<BitfieldCall, AdaptationError>>,
}

impl VisitMut for CallCollector {
    fn visit_expr_method_call_mut(&mut self, method_call: &mut ExprMethodCall) {
        let Self { call_list } = self;

        let method_name = method_call.method.to_string();
        let kind = match method_name.as_str() {
            "get_const" => Some(AccessorKind::Getter),
            "set_const" => Some(AccessorKind::Setter),
            _ => None,
        };

        match (kind, Self::self_member(&method_call.receiver)) {
            (Some(kind), Some(unit_field)) => call_list.push(Self::parse_call(method_call, unit_field, kind)),
            _ => {},
        }

        syn::visit_mut::visit_expr_method_call_mut(self, method_call);
    }
}

impl CallCollector {
    /// Read a direct `self.field` receiver.
    fn self_member(expression: &Expr) -> Option<String> {
        match expression {
            Expr::Field(field) => match (field.base.as_ref(), &field.member) {
                (Expr::Path(base), Member::Named(member_name)) if base.path.is_ident("self") => {
                    Some(member_name.to_string())
                },
                _ => None,
            },
            _ => None,
        }
    }

    /// Parse Bindgen's two const-generic bit-range arguments.
    fn parse_call(
        method_call: &ExprMethodCall,
        unit_field: String,
        kind: AccessorKind,
    ) -> Result<BitfieldCall, AdaptationError> {
        let arguments = method_call
            .turbofish
            .as_ref()
            .ok_or_else(|| AdaptationError::Missing("Bindgen bitfield accessor has no const-generic range".into()))?;
        let argument_list = arguments.args.iter().collect::<Vec<_>>();

        match argument_list.as_slice() {
            [
                GenericArgument::Const(offset_expression),
                GenericArgument::Const(width_expression),
            ] => {
                let offset = Adapter::integer_literal(offset_expression)?;

                match Adapter::integer_literal(width_expression)? {
                    width @ 1..=64 => {
                        // NOTE: The matched range fits in `u8` and Bindgen's scalar accessor.
                        let width = width as u8;

                        Ok(BitfieldCall {
                            unit_field,
                            offset,
                            width,
                            kind,
                        })
                    },
                    width => Err(AdaptationError::Unsupported(format!(
                        "unsupported Bindgen bitfield width `{width}`"
                    ))),
                }
            },
            _ => Err(AdaptationError::Unsupported(
                "Bindgen bitfield accessor must have const offset and width arguments".into(),
            )),
        }
    }
}

/// Accessor groups indexed by record, helper-unit field, and Bindgen field name.
type AccessorMap = BTreeMap<String, BTreeMap<String, Vec<BitfieldAccessor>>>;

/// A bitfield record paired with its union storage arm.
// NOTE(invariant): Every member records layout and accessor facts validated against Bindgen's
// assertions.
struct RecordTranslation {
    /// Bindgen struct that contains the helper unit.
    record_ident: syn::Ident,

    /// Bindgen union replaced by a transparent integer wrapper.
    union_ident: syn::Ident,

    /// Public alias prefix derived from the containing UAPI record member.
    type_prefix: String,

    /// Raw integer arm retained by the wrapper.
    raw_field: syn::Field,

    /// Width of the raw integer arm in bits.
    storage_bits: usize,

    /// Alignment of the original C union in bytes.
    union_alignment: usize,

    /// Parsed fields to expose as `nekor-bitwise` aliases and methods.
    field_accessor_list: Vec<FieldAccessor>,
}

/// A type-name scanner used after removing Bindgen's helper declaration.
// NOTE(invariant): None.
struct TypeNameScan<'name> {
    /// Helper identifier to find.
    type_name: &'name str,

    /// Whether a type path still names the helper.
    found: bool,
}

impl BitfieldAccessor {
    /// Validate a borrowed accessor signature and resolve its unsigned scalar width.
    fn validate_signature(
        method: &syn::ImplItemFn,
        field_width: u8,
        kind: AccessorKind,
        layout_resolver: &LayoutResolver<'_>,
    ) -> Result<usize, AdaptationError> {
        let parameter_list = method
            .sig
            .inputs
            .iter()
            .filter_map(|argument| match argument {
                syn::FnArg::Typed(argument) => Some(argument.ty.as_ref()),
                syn::FnArg::Receiver(_) => None,
            })
            .collect::<Vec<_>>();
        let receiver_kind = method.sig.receiver().map(|receiver| &receiver.kind);

        match (
            &method.sig.safety,
            receiver_kind,
            kind,
            method.sig.generics.params.len(),
            &method.sig.generics.where_clause,
        ) {
            (syn::Safety::Default, Some(syn::ReceiverKind::Reference(_, _, None)), AccessorKind::Getter, 0, None)
            | (
                syn::Safety::Default,
                Some(syn::ReceiverKind::Reference(_, _, Some(_))),
                AccessorKind::Setter,
                0,
                None,
            ) => Ok(()),
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen accessor `{}` must safely borrow its receiver with the accessor's mutability",
                method.sig.ident
            ))),
        }?;

        let value_type = match (kind, parameter_list.as_slice(), &method.sig.output) {
            (AccessorKind::Setter, [value_type], syn::ReturnType::Default) => *value_type,
            (AccessorKind::Getter, [], syn::ReturnType::Type(_, return_type)) => return_type.as_ref(),
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen accessor `{}` has an unsupported signature",
                method.sig.ident
            )))?,
        };

        match layout_resolver.unsigned_integer_bits(value_type)? {
            Some(scalar_bits) if usize::from(field_width) <= scalar_bits => Ok(scalar_bits),
            _ => Err(AdaptationError::Unsupported(format!(
                "Bindgen accessor `{}` must use an unsigned integer at least as wide as its field",
                method.sig.ident
            ))),
        }
    }
}
