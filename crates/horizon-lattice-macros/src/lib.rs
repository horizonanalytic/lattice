//! Procedural macros for Horizon Lattice meta-object system.
//!
//! This crate provides the `#[derive(Object)]` macro and related attribute macros
//! for defining signals, properties, and slots.
//!
//! # Attributes
//!
//! ## `#[property]`
//!
//! Marks a field as a property with optional change notification:
//!
//! ```ignore
//! #[derive(Object)]
//! struct Counter {
//!     base: ObjectBase,
//!
//!     #[property(notify = "value_changed")]
//!     value: Property<i32>,
//!
//!     #[property(read_only)]
//!     computed: i32,
//!
//!     #[signal]
//!     value_changed: Signal<i32>,
//! }
//! ```
//!
//! Property attributes:
//! - `notify = "signal_name"`: Links the property to a change notification signal
//! - `read_only`: Makes the property read-only (no setter generated)
//! - `skip`: Excludes the field from meta-object registration
//!
//! ## `#[signal]`
//!
//! Marks a field as a signal:
//!
//! ```ignore
//! #[signal]
//! clicked: Signal<()>,
//!
//! #[signal]
//! text_changed: Signal<String>,
//! ```
//!
//! ## `#[object]`
//!
//! Struct-level attribute for object configuration:
//!
//! ```ignore
//! #[derive(Object)]
//! #[object(no_factory)]  // Don't generate factory function
//! struct MyWidget {
//!     // ...
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, ExprLit, Field, Fields, Ident, Lit,
    Type,
};

/// Derive the `Object` trait and generate meta-object information.
///
/// This macro generates:
/// - A static `MetaObject` containing type information
/// - Type-erased getter/setter functions for properties
/// - Implementation of the `Object` trait
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::prelude::*;
///
/// #[derive(Object, Default)]
/// struct Button {
///     base: ObjectBase,
///
///     #[property(notify = "text_changed")]
///     text: Property<String>,
///
///     #[signal]
///     clicked: Signal<()>,
///
///     #[signal]
///     text_changed: Signal<String>,
/// }
/// ```
#[proc_macro_derive(Object, attributes(object, property, signal))]
pub fn derive_object(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match impl_derive_object(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Parsed property information.
#[allow(dead_code)]
struct PropertyInfo {
    field_name: Ident,
    field_type: Type,
    inner_type: Type,
    notify_signal: Option<String>,
    read_only: bool,
    is_property_wrapper: bool,
}

/// Parsed signal information.
#[allow(dead_code)]
struct SignalInfo {
    field_name: Ident,
    args_type: Type,
    param_type_names: Vec<String>,
}

/// Parsed struct-level object attributes.
struct ObjectAttrs {
    no_factory: bool,
}

fn impl_derive_object(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let meta_object_name = format_ident!("{}_META", struct_name.to_string().to_uppercase());

    // Parse struct-level attributes
    let object_attrs = parse_object_attrs(&input.attrs)?;

    // Get struct fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "Object derive only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "Object derive only supports structs",
            ))
        }
    };

    // Find ObjectBase field
    let base_field = fields
        .iter()
        .find(|f| f.ident.as_ref().is_some_and(|i| i == "base"));

    if base_field.is_none() {
        return Err(syn::Error::new_spanned(
            input,
            "Object derive requires a `base: ObjectBase` field",
        ));
    }

    // Parse properties and signals from fields
    let mut properties = Vec::new();
    let mut signals = Vec::new();

    for field in fields.iter() {
        if let Some(prop_info) = parse_property_field(field)? {
            properties.push(prop_info);
        }
        if let Some(signal_info) = parse_signal_field(field)? {
            signals.push(signal_info);
        }
    }

    // Generate getter/setter functions
    let getter_setter_fns = generate_getter_setter_fns(struct_name, &properties);

    // Generate property metadata array
    let property_meta = generate_property_meta(struct_name, &properties);

    // Generate signal metadata array
    let signal_meta = generate_signal_meta(&signals);

    // Generate factory function
    let factory = if object_attrs.no_factory {
        quote! { None }
    } else {
        quote! {
            Some(|| Box::new(<#struct_name as Default>::default()) as Box<dyn horizon_lattice_core::Object>)
        }
    };

    // Generate the full implementation
    let expanded = quote! {
        #getter_setter_fns

        /// Static meta-object for this type (generated by #[derive(Object)]).
        #[allow(non_upper_case_globals)]
        static #meta_object_name: horizon_lattice_core::meta::MetaObject = horizon_lattice_core::meta::MetaObject {
            type_id: std::any::TypeId::of::<#struct_name>(),
            type_name: stringify!(#struct_name),
            parent: None,
            properties: &#property_meta,
            signals: &#signal_meta,
            methods: &[],
            create: #factory,
        };

        impl #struct_name {
            /// Reference to the static MetaObject for this type.
            ///
            /// This can be used to access type metadata without an instance:
            /// ```ignore
            /// let meta = MyWidget::META;
            /// println!("Type has {} properties", meta.properties.len());
            /// ```
            pub const META: &'static horizon_lattice_core::meta::MetaObject = &#meta_object_name;

            /// Register this type in the global TypeRegistry.
            ///
            /// Call this during application initialization to enable dynamic
            /// object creation by type name:
            /// ```ignore
            /// MyWidget::register_type();
            ///
            /// // Later, create dynamically:
            /// let obj = TypeRegistry::create("MyWidget");
            /// ```
            pub fn register_type() {
                horizon_lattice_core::TypeRegistry::register(&#meta_object_name);
            }
        }

        impl horizon_lattice_core::Object for #struct_name {
            fn object_id(&self) -> horizon_lattice_core::object::ObjectId {
                self.base.id()
            }

            fn meta_object(&self) -> Option<&'static horizon_lattice_core::meta::MetaObject> {
                Some(&#meta_object_name)
            }
        }
    };

    Ok(expanded)
}

/// Parse struct-level #[object(...)] attributes.
fn parse_object_attrs(attrs: &[Attribute]) -> syn::Result<ObjectAttrs> {
    let mut result = ObjectAttrs { no_factory: false };

    for attr in attrs {
        if !attr.path().is_ident("object") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("no_factory") {
                result.no_factory = true;
            }
            Ok(())
        })?;
    }

    Ok(result)
}

/// Parse a field with #[property] attribute.
fn parse_property_field(field: &Field) -> syn::Result<Option<PropertyInfo>> {
    let field_name = match &field.ident {
        Some(name) => name.clone(),
        None => return Ok(None),
    };

    // Skip base field
    if field_name == "base" {
        return Ok(None);
    }

    let mut notify_signal = None;
    let mut read_only = false;
    let mut has_property_attr = false;
    let mut skip = false;

    for attr in &field.attrs {
        if attr.path().is_ident("property") {
            has_property_attr = true;

            // Parse property attributes using syn 2.0 API
            // Handle both #[property] and #[property(...)]
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("notify") {
                    // Parse notify = "signal_name"
                    let value: Expr = meta.value()?.parse()?;
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit_str),
                        ..
                    }) = value
                    {
                        notify_signal = Some(lit_str.value());
                    }
                } else if meta.path.is_ident("read_only") {
                    read_only = true;
                } else if meta.path.is_ident("skip") {
                    skip = true;
                }
                Ok(())
            });
            // Note: We ignore parse errors for attributes like #[property] (no args)
        }

        // Also skip fields with #[signal] attribute
        if attr.path().is_ident("signal") {
            return Ok(None);
        }
    }

    if skip {
        return Ok(None);
    }

    // If no explicit #[property] attribute, treat all non-signal fields as properties
    // (except base and other special fields)
    if !has_property_attr {
        // Skip common non-property field patterns
        let name_str = field_name.to_string();
        if name_str.starts_with('_') {
            return Ok(None);
        }
    }

    // Detect if this is a Property<T> wrapper
    let (inner_type, is_property_wrapper) = extract_inner_type(&field.ty);

    Ok(Some(PropertyInfo {
        field_name,
        field_type: field.ty.clone(),
        inner_type,
        notify_signal,
        read_only,
        is_property_wrapper,
    }))
}

/// Parse a field with #[signal] attribute.
fn parse_signal_field(field: &Field) -> syn::Result<Option<SignalInfo>> {
    let field_name = match &field.ident {
        Some(name) => name.clone(),
        None => return Ok(None),
    };

    let has_signal_attr = field.attrs.iter().any(|attr| attr.path().is_ident("signal"));

    if !has_signal_attr {
        return Ok(None);
    }

    // Extract Signal<Args> type parameter
    let (args_type, param_type_names) = extract_signal_args(&field.ty)?;

    Ok(Some(SignalInfo {
        field_name,
        args_type,
        param_type_names,
    }))
}

/// Extract inner type from Property<T> or return the original type.
fn extract_inner_type(ty: &Type) -> (Type, bool) {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Property" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return (inner.clone(), true);
                    }
                }
            }
        }
    }
    (ty.clone(), false)
}

/// Extract Signal<Args> type parameter and convert to type names.
fn extract_signal_args(ty: &Type) -> syn::Result<(Type, Vec<String>)> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Signal" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(args_type)) = args.args.first() {
                        let param_names = extract_param_type_names(args_type);
                        return Ok((args_type.clone(), param_names));
                    }
                }
            }
        }
    }

    // Default to unit type if we can't parse
    let unit_type: Type = syn::parse_quote!(());
    Ok((unit_type, vec![]))
}

/// Extract parameter type names from a signal argument type.
fn extract_param_type_names(ty: &Type) -> Vec<String> {
    match ty {
        Type::Tuple(tuple) if tuple.elems.is_empty() => {
            // () - no parameters
            vec![]
        }
        Type::Tuple(tuple) => {
            // Multiple parameters in tuple
            tuple.elems.iter().map(type_to_string).collect()
        }
        _ => {
            // Single parameter
            vec![type_to_string(ty)]
        }
    }
}

/// Convert a type to a string representation.
fn type_to_string(ty: &Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}

/// Generate getter and setter functions for properties.
fn generate_getter_setter_fns(struct_name: &Ident, properties: &[PropertyInfo]) -> TokenStream2 {
    let fns: Vec<TokenStream2> = properties
        .iter()
        .map(|prop| {
            let field_name = &prop.field_name;
            let inner_type = &prop.inner_type;
            let getter_name = format_ident!(
                "__{}_{}_getter",
                struct_name.to_string().to_lowercase(),
                field_name
            );
            let setter_name = format_ident!(
                "__{}_{}_setter",
                struct_name.to_string().to_lowercase(),
                field_name
            );
            let type_name_str = type_to_string(inner_type);

            let getter_body = if prop.is_property_wrapper {
                quote! {
                    let typed = horizon_lattice_core::object_cast::<#struct_name>(obj)
                        .expect("object_cast failed in generated getter");
                    Box::new(typed.#field_name.get())
                }
            } else {
                quote! {
                    let typed = horizon_lattice_core::object_cast::<#struct_name>(obj)
                        .expect("object_cast failed in generated getter");
                    Box::new(typed.#field_name.clone())
                }
            };

            let setter_fn = if prop.read_only {
                quote! {}
            } else {
                let setter_body = if prop.is_property_wrapper {
                    quote! {
                        let typed = horizon_lattice_core::object_cast_mut::<#struct_name>(obj)
                            .expect("object_cast_mut failed in generated setter");
                        let val = value.downcast::<#inner_type>().map_err(|_| {
                            horizon_lattice_core::meta::MetaError::PropertyTypeMismatch {
                                expected: #type_name_str,
                                got: "unknown",
                            }
                        })?;
                        typed.#field_name.set_silent(*val);
                        Ok(())
                    }
                } else {
                    quote! {
                        let typed = horizon_lattice_core::object_cast_mut::<#struct_name>(obj)
                            .expect("object_cast_mut failed in generated setter");
                        let val = value.downcast::<#inner_type>().map_err(|_| {
                            horizon_lattice_core::meta::MetaError::PropertyTypeMismatch {
                                expected: #type_name_str,
                                got: "unknown",
                            }
                        })?;
                        typed.#field_name = *val;
                        Ok(())
                    }
                };

                quote! {
                    #[allow(non_snake_case)]
                    fn #setter_name(
                        obj: &mut dyn horizon_lattice_core::Object,
                        value: Box<dyn std::any::Any>,
                    ) -> horizon_lattice_core::meta::MetaResult<()> {
                        #setter_body
                    }
                }
            };

            quote! {
                #[allow(non_snake_case)]
                fn #getter_name(obj: &dyn horizon_lattice_core::Object) -> Box<dyn std::any::Any> {
                    #getter_body
                }

                #setter_fn
            }
        })
        .collect();

    quote! { #(#fns)* }
}

/// Generate property metadata array.
fn generate_property_meta(struct_name: &Ident, properties: &[PropertyInfo]) -> TokenStream2 {
    let meta_entries: Vec<TokenStream2> = properties
        .iter()
        .map(|prop| {
            let field_name = &prop.field_name;
            let inner_type = &prop.inner_type;
            let field_name_str = field_name.to_string();
            let type_name_str = type_to_string(inner_type);
            let read_only = prop.read_only;
            let getter_name = format_ident!(
                "__{}_{}_getter",
                struct_name.to_string().to_lowercase(),
                field_name
            );
            let setter_name = format_ident!(
                "__{}_{}_setter",
                struct_name.to_string().to_lowercase(),
                field_name
            );

            let notify_signal = match &prop.notify_signal {
                Some(name) => quote! { Some(#name) },
                None => quote! { None },
            };

            let setter = if prop.read_only {
                quote! { None }
            } else {
                quote! { Some(#setter_name) }
            };

            quote! {
                horizon_lattice_core::meta::MetaProperty {
                    name: #field_name_str,
                    type_name: #type_name_str,
                    type_id: std::any::TypeId::of::<#inner_type>(),
                    read_only: #read_only,
                    notify_signal: #notify_signal,
                    getter: #getter_name,
                    setter: #setter,
                }
            }
        })
        .collect();

    quote! { [#(#meta_entries),*] }
}

/// Generate signal metadata array.
fn generate_signal_meta(signals: &[SignalInfo]) -> TokenStream2 {
    let meta_entries: Vec<TokenStream2> = signals
        .iter()
        .enumerate()
        .map(|(index, signal)| {
            let signal_name_str = signal.field_name.to_string();
            let param_types: Vec<TokenStream2> = signal
                .param_type_names
                .iter()
                .map(|name| {
                    quote! { #name }
                })
                .collect();

            quote! {
                horizon_lattice_core::meta::SignalMeta {
                    name: #signal_name_str,
                    param_types: &[#(#param_types),*],
                    index: #index,
                }
            }
        })
        .collect();

    quote! { [#(#meta_entries),*] }
}
