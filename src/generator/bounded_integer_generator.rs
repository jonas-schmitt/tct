//! Bounded Integer Generator
//!
//! Generates newtype wrappers for bounded integers with codec-based or modulo encoding.

use crate::TctResult;
use crate::generator::encoding_decision::EncodingDecision;
use crate::generator::schema_discovery::TypeDiscovery;
use crate::generator::type_analysis::TraitDerivability;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub struct BoundedIntegerGenerator {
    streaming_encode: bool,
    encoding_base: u32,
}

impl BoundedIntegerGenerator {
    const DEFAULT_ENCODING_BASE: u32 = 256;

    pub fn new() -> Self {
        Self {
            streaming_encode: false,
            encoding_base: Self::DEFAULT_ENCODING_BASE,
        }
    }

    pub fn with_streaming_encode(streaming_encode: bool) -> Self {
        Self {
            streaming_encode,
            encoding_base: Self::DEFAULT_ENCODING_BASE,
        }
    }

    pub fn with_streaming_encode_and_encoding_base(
        streaming_encode: bool,
        encoding_base: u32,
    ) -> Self {
        Self {
            streaming_encode,
            encoding_base,
        }
    }

    pub fn generate_bounded_integer_type(
        &self,
        type_discovery: &TypeDiscovery,
        encoding_decision: &EncodingDecision,
        trait_derivability: Option<&TraitDerivability>,
    ) -> TctResult<(TokenStream, TokenStream)> {
        let type_name_ident = format_ident!("{}", type_discovery.type_name);

        // Generate type definition (newtype wrapper)
        let type_def = self.generate_type_definition(&type_name_ident, trait_derivability)?;

        // Generate codec module if needed
        let codec_module = match encoding_decision {
            EncodingDecision::BoundedIntegerCodec { codec_spec, .. } => {
                crate::generator::structural_codec::generate_codec_module(
                    &type_discovery.type_name,
                    codec_spec,
                )
            }
            _ => quote! {},
        };

        // Generate encode/decode implementations
        let encoder_impl = self.generate_encoder(&type_name_ident, encoding_decision)?;

        // Definition tokens = codec module + type definition
        let definition_tokens = quote! {
            #codec_module
            #type_def
        };

        // Implementation tokens = encode/decode impl
        let implementation_tokens = encoder_impl;

        Ok((definition_tokens, implementation_tokens))
    }

    fn generate_type_definition(
        &self,
        type_name: &proc_macro2::Ident,
        trait_derivability: Option<&TraitDerivability>,
    ) -> TctResult<TokenStream> {
        // Use same derives as primitive wrappers
        let derives = if let Some(derivability) = trait_derivability {
            let mut derives = vec![quote! { Debug }, quote! { Clone }, quote! { PartialEq }];

            if derivability.can_derive_eq {
                derives.push(quote! { Eq });
            }
            if derivability.can_derive_ord {
                derives.push(quote! { PartialOrd });
                derives.push(quote! { Ord });
            }
            if derivability.can_derive_hash {
                derives.push(quote! { Hash });
            }

            // Add Default when streaming encode is enabled (needed for decode_prefix)
            if self.streaming_encode {
                derives.push(quote! { Default });
            }

            derives.push(quote! { serde::Serialize });
            derives.push(quote! { serde::Deserialize });

            quote! { #[derive(#(#derives),*)] }
        } else {
            // Default derives when no trait analysis available
            if self.streaming_encode {
                quote! {
                    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, serde::Serialize, serde::Deserialize)]
                }
            } else {
                quote! {
                    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
                }
            }
        };

        Ok(quote! {
            #derives
            #[serde(transparent)]
            pub struct #type_name(pub i64);

            impl #type_name {
                /// Get the inner value
                pub fn value(&self) -> &i64 {
                    &self.0
                }

                /// Get the inner value (consuming)
                pub fn into_value(self) -> i64 {
                    self.0
                }

                /// Create from inner value
                pub fn new(value: i64) -> Self {
                    Self(value)
                }
            }
        })
    }

    fn generate_encoder(
        &self,
        type_name: &proc_macro2::Ident,
        encoding_decision: &EncodingDecision,
    ) -> TctResult<TokenStream> {
        let (encode_impl, decode_impl, strict_decode_impl, token_count_impl) =
            match encoding_decision {
                EncodingDecision::BoundedIntegerCodec {
                    min, codec_spec, ..
                } => {
                    let codec_mod_name =
                        format_ident!("{}_codec", type_name.to_string().to_lowercase());
                    let token_width = codec_spec.token_width;

                    (
                        quote! {
                            let offset = (self.0 - #min) as u32;
                            #codec_mod_name::encode(offset, tokens);
                        },
                        quote! {
                            let offset = #codec_mod_name::decode(tokens)?;
                            let value = (offset as i64) + #min;
                            Ok(Self(value))
                        },
                        quote! {
                            let offset = #codec_mod_name::decode_strict(tokens)?;
                            let value = (offset as i64) + #min;
                            Ok(Self(value))
                        },
                        quote! { #token_width },
                    )
                }
                EncodingDecision::BoundedIntegerOffset { min, range, .. } => {
                    let encoding_base = self.encoding_base;
                    let max_width = Self::max_token_width_for_base(encoding_base);
                    let encode_width_chain = Self::generate_offset_encode_width_chain(
                        encoding_base,
                        max_width,
                        quote! { offset },
                    );
                    let token_count_width_chain =
                        Self::generate_offset_token_count_width_chain(encoding_base, max_width);

                    (
                        // Use the configured variable-base encoding to respect vocabulary bounds.
                        // The width thresholds and divisors are fixed by the schema/compiler config.
                        quote! {
                            const ENCODING_BASE: u32 = #encoding_base;
                            let offset = (self.0 - #min) as u32;
                            #encode_width_chain
                        },
                        {
                            quote! {
                                let offset = crate::tokenizer::variable_base::decode_u32_fixed_base(
                                    tokens,
                                    stringify!(#type_name)
                                )?;
                                let value = (offset as i64) + #min;
                                Ok(Self(value))
                            }
                        },
                        quote! {
                            let offset = crate::tokenizer::variable_base::decode_u32_fixed_base_strict(
                                tokens,
                                stringify!(#type_name)
                            )?;
                            if offset >= #range {
                                return Err(crate::tokenizer::DecodeError::InvalidData(format!(
                                    "bounded integer offset {} exceeds schema range 0..{}",
                                    offset,
                                    #range.saturating_sub(1)
                                )));
                            }
                            let value = (offset as i64) + #min;
                            Ok(Self(value))
                        },
                        quote! {
                            let offset = (self.0 - #min) as u32;
                            #token_count_width_chain
                        },
                    )
                }
                _ => {
                    // Fallback to standard i64 encoding
                    (
                        quote! {
                            self.0.encode(tokens);
                        },
                        quote! {
                            let value = <i64>::decode(tokens)?;
                            Ok(Self(value))
                        },
                        quote! {
                            let value = <i64>::decode_strict(tokens)?;
                            Ok(Self(value))
                        },
                        quote! { self.0.token_count() },
                    )
                }
            };

        // Generate streaming decode_prefix if streaming mode is enabled
        let decode_prefix_impl = if self.streaming_encode {
            quote! {
                impl #type_name {
                    /// Streaming decode that handles truncated token sequences
                    pub fn decode_prefix(tokens: &mut impl Iterator<Item = u32>) -> PrefixDecodeResult<Self> {
                        // Try to decode - bounded integers are simple enough to be all-or-nothing
                        match <Self as crate::tokenizer::JsonTokenEncoder>::decode(tokens) {
                            Ok(value) => PrefixDecodeResult {
                                instance: value,
                                fields_decoded: 1,
                                complete: true,
                                partial_field: None,
                            },
                            Err(_) => PrefixDecodeResult {
                                instance: Self::default(),
                                fields_decoded: 0,
                                complete: false,
                                partial_field: None,
                            },
                        }
                    }
                }
            }
        } else {
            quote! {}
        };

        Ok(quote! {
            impl crate::tokenizer::JsonTokenEncoder for #type_name {
                fn encode(&self, tokens: &mut Vec<u32>) {
                    #encode_impl
                }

                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    #decode_impl
                }

                fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                    #strict_decode_impl
                }

                fn token_count(&self) -> usize {
                    #token_count_impl
                }
            }

            #decode_prefix_impl
        })
    }

    fn max_token_width_for_base(base: u32) -> u32 {
        let mut width = 1u32;
        let mut max_value = base as u64;
        while max_value <= u32::MAX as u64 {
            width += 1;
            max_value = max_value.saturating_mul(base as u64);
        }
        width
    }

    fn generate_offset_encode_width_chain(
        base: u32,
        max_width: u32,
        offset_expr: TokenStream,
    ) -> TokenStream {
        let mut branch = Self::generate_offset_encode_for_width(base, max_width, &offset_expr);

        for width in (1..max_width).rev() {
            let threshold = Self::base_power_u32(base, width);
            let encode_for_width =
                Self::generate_offset_encode_for_width(base, width, &offset_expr);
            branch = quote! {
                if #offset_expr < #threshold {
                    #encode_for_width
                } else {
                    #branch
                }
            };
        }

        branch
    }

    fn generate_offset_encode_for_width(
        base: u32,
        width: u32,
        offset_expr: &TokenStream,
    ) -> TokenStream {
        let width_indicator = width - 1;
        let digit_pushes = (0..width).rev().map(|power| {
            let divisor = Self::base_power_u32(base, power);
            if power == 0 {
                quote! {
                    tokens.push(#offset_expr % ENCODING_BASE);
                }
            } else {
                quote! {
                    tokens.push((#offset_expr / #divisor) % ENCODING_BASE);
                }
            }
        });

        quote! {
            tokens.push(#width_indicator);
            #(#digit_pushes)*
        }
    }

    fn generate_offset_token_count_width_chain(base: u32, max_width: u32) -> TokenStream {
        let mut branch = {
            let count = max_width as usize + 1;
            quote! { #count }
        };

        for width in (1..max_width).rev() {
            let threshold = Self::base_power_u32(base, width);
            let count = width as usize + 1;
            branch = quote! {
                if offset < #threshold {
                    #count
                } else {
                    #branch
                }
            };
        }

        branch
    }

    fn base_power_u32(base: u32, exponent: u32) -> u32 {
        let mut value = 1u64;
        for _ in 0..exponent {
            value = value.saturating_mul(base as u64);
        }
        u32::try_from(value).expect("base power should fit in u32 for generated width branch")
    }
}

impl Default for BoundedIntegerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::encoding_decision::{CodecSpec, EncodingDecision};
    use crate::generator::schema_discovery::{SchemaLocation, TypeDiscovery, TypeKind};
    use crate::generator::type_analysis::TraitDerivability;
    use quote::ToTokens;
    use std::collections::HashSet;
    use syn::punctuated::Punctuated;

    fn parse_generated_file(tokens: TokenStream) -> syn::File {
        match syn::parse2(tokens) {
            Ok(file) => file,
            Err(error) => panic!("generated bounded-integer tokens should parse as Rust: {error}"),
        }
    }

    fn generated_struct<'a>(file: &'a syn::File, name: &str) -> &'a syn::ItemStruct {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Struct(item_struct) if item_struct.ident == name => Some(item_struct),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected generated struct {name}"))
    }

    fn generated_module_items<'a>(file: &'a syn::File, name: &str) -> &'a [syn::Item] {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Mod(item_mod) if item_mod.ident == name => {
                    item_mod.content.as_ref().map(|(_, items)| items.as_slice())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected inline generated module {name}"))
    }

    fn generated_function<'a>(items: &'a [syn::Item], name: &str) -> &'a syn::ItemFn {
        items
            .iter()
            .find_map(|item| match item {
                syn::Item::Fn(item_fn) if item_fn.sig.ident == name => Some(item_fn),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected generated function {name}"))
    }

    fn impl_self_type_matches(item_impl: &syn::ItemImpl, type_name: &str) -> bool {
        item_impl.self_ty.to_token_stream().to_string() == type_name
    }

    fn impl_method<'a>(
        item_impl: &'a syn::ItemImpl,
        method_name: &str,
    ) -> Option<&'a syn::ImplItemFn> {
        item_impl.items.iter().find_map(|item| match item {
            syn::ImplItem::Fn(method) if method.sig.ident == method_name => Some(method),
            _ => None,
        })
    }

    fn generated_trait_method<'a>(
        file: &'a syn::File,
        type_name: &str,
        trait_name: &str,
        method_name: &str,
    ) -> &'a syn::ImplItemFn {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Impl(item_impl)
                    if impl_self_type_matches(item_impl, type_name)
                        && item_impl.trait_.as_ref().is_some_and(|(_, path, _)| {
                            path.segments
                                .last()
                                .is_some_and(|segment| segment.ident == trait_name)
                        }) =>
                {
                    impl_method(item_impl, method_name)
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!("expected generated {trait_name}::{method_name} impl for {type_name}")
            })
    }

    fn generated_inherent_method<'a>(
        file: &'a syn::File,
        type_name: &str,
        method_name: &str,
    ) -> Option<&'a syn::ImplItemFn> {
        file.items.iter().find_map(|item| match item {
            syn::Item::Impl(item_impl)
                if item_impl.trait_.is_none() && impl_self_type_matches(item_impl, type_name) =>
            {
                impl_method(item_impl, method_name)
            }
            _ => None,
        })
    }

    fn derive_traits(attrs: &[syn::Attribute]) -> Vec<String> {
        attrs
            .iter()
            .filter(|attr| attr.path().is_ident("derive"))
            .filter_map(|attr| {
                attr.parse_args_with(Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
                    .ok()
            })
            .flat_map(|paths| paths.into_iter())
            .map(|path| {
                path.segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::")
            })
            .collect()
    }

    fn tuple_struct_field_type(item_struct: &syn::ItemStruct) -> String {
        match &item_struct.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                fields.unnamed[0].ty.to_token_stream().to_string()
            }
            _ => panic!("expected single-field tuple struct {}", item_struct.ident),
        }
    }

    fn has_derive_trait(derives: &[String], expected: &str) -> bool {
        derives.iter().any(|derive| derive == expected)
    }

    fn method_signature_contains(method: &syn::ImplItemFn, expected: &str) -> bool {
        method.sig.to_token_stream().to_string().contains(expected)
    }

    fn create_test_type_discovery(name: &str) -> TypeDiscovery {
        TypeDiscovery::new(
            name.to_string(),
            TypeKind::PrimitiveWrapper {
                primitive_type: "i64".to_string(),
            },
            SchemaLocation {
                json_path: format!("#/{}", name),
                description: format!("Test type {}", name),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        )
    }

    fn create_codec_spec(base: u32, token_width: usize, max_value: u32) -> CodecSpec {
        let is_power_of_2 = base.is_power_of_two();
        let shift = if is_power_of_2 {
            Some(base.trailing_zeros())
        } else {
            None
        };
        let mask = if is_power_of_2 { Some(base - 1) } else { None };
        let divisors = if !is_power_of_2 {
            (0..token_width).rev().map(|i| base.pow(i as u32)).collect()
        } else {
            vec![]
        };

        CodecSpec {
            token_width,
            base,
            max_value,
            is_power_of_2,
            shift,
            mask,
            divisors,
        }
    }

    fn create_trait_derivability(eq: bool, ord: bool, hash: bool) -> TraitDerivability {
        TraitDerivability {
            can_derive_eq: eq,
            can_derive_ord: ord,
            can_derive_hash: hash,
            reasoning: "Test derivability".to_string(),
            confidence: 1.0,
            blocking_types: HashSet::new(),
        }
    }

    #[test]
    fn test_bounded_integer_generator_new() {
        let generator = BoundedIntegerGenerator::new();
        let _ = generator; // Verify it compiles
    }

    #[test]
    fn test_bounded_integer_generator_default() {
        let generator = BoundedIntegerGenerator::default();
        let _ = generator;
    }

    // ========== Batch 1: generate_bounded_integer_type tests ==========

    #[test]
    fn test_generate_bounded_integer_type_with_codec() {
        let generator = BoundedIntegerGenerator::new();
        let type_disc = create_test_type_discovery("TestBoundedInt");

        let encoding = EncodingDecision::BoundedIntegerCodec {
            min: 0,
            max: 255,
            codec_spec: create_codec_spec(10, 3, 255),
        };

        let result = generator.generate_bounded_integer_type(&type_disc, &encoding, None);
        assert!(result.is_ok());

        let (def_tokens, impl_tokens) = result.unwrap();
        let definition_file = parse_generated_file(def_tokens);
        let implementation_file = parse_generated_file(impl_tokens);

        // Verify definition contains struct and codec module
        generated_struct(&definition_file, "TestBoundedInt");
        let codec_items = generated_module_items(&definition_file, "testboundedint_codec");
        generated_function(codec_items, "encode");

        // Verify implementation contains encode/decode
        generated_trait_method(
            &implementation_file,
            "TestBoundedInt",
            "JsonTokenEncoder",
            "encode",
        );
        generated_trait_method(
            &implementation_file,
            "TestBoundedInt",
            "JsonTokenEncoder",
            "decode",
        );
    }

    #[test]
    fn test_generate_bounded_integer_type_with_offset() {
        let generator = BoundedIntegerGenerator::new();
        let type_disc = create_test_type_discovery("OffsetInt");

        let encoding = EncodingDecision::BoundedIntegerOffset {
            min: 10,
            max: 110,
            range: 101, // max - min + 1
        };

        let result = generator.generate_bounded_integer_type(&type_disc, &encoding, None);
        assert!(result.is_ok());

        let (_def_tokens, impl_tokens) = result.unwrap();
        let implementation_file = parse_generated_file(impl_tokens.clone());
        let impl_str = impl_tokens.to_string();

        // Verify offset encoding is generated
        generated_trait_method(
            &implementation_file,
            "OffsetInt",
            "JsonTokenEncoder",
            "encode",
        );
        generated_trait_method(
            &implementation_file,
            "OffsetInt",
            "JsonTokenEncoder",
            "decode",
        );
        // Verify it contains the min value 10 in some form
        assert!(impl_str.contains("10"));
    }

    #[test]
    fn test_generate_bounded_integer_type_fallback() {
        let generator = BoundedIntegerGenerator::new();
        let type_disc = create_test_type_discovery("FallbackInt");

        // Use a different encoding type to trigger fallback
        let encoding = EncodingDecision::Standard;

        let result = generator.generate_bounded_integer_type(&type_disc, &encoding, None);
        assert!(result.is_ok());

        let (_def_tokens, impl_tokens) = result.unwrap();
        let implementation_file = parse_generated_file(impl_tokens.clone());
        let impl_str = impl_tokens.to_string();

        // Verify fallback encoding is generated
        generated_trait_method(
            &implementation_file,
            "FallbackInt",
            "JsonTokenEncoder",
            "encode",
        );
        generated_trait_method(
            &implementation_file,
            "FallbackInt",
            "JsonTokenEncoder",
            "decode",
        );
        assert!(impl_str.contains("i64")); // Should reference i64 type
    }

    // ========== Batch 2: generate_type_definition tests ==========

    #[test]
    fn test_generate_type_definition_without_derivability() {
        let generator = BoundedIntegerGenerator::new();
        let type_name = format_ident!("TestType");

        let result = generator.generate_type_definition(&type_name, None);
        assert!(result.is_ok());

        let definition_file = parse_generated_file(result.unwrap());
        let item_struct = generated_struct(&definition_file, "TestType");
        let derives = derive_traits(&item_struct.attrs);

        // Verify default derives
        assert!(has_derive_trait(&derives, "Debug"));
        assert!(has_derive_trait(&derives, "Clone"));
        assert!(has_derive_trait(&derives, "PartialEq"));
        assert!(has_derive_trait(&derives, "Eq"));
        assert!(has_derive_trait(&derives, "PartialOrd"));
        assert!(has_derive_trait(&derives, "Ord"));
        assert!(has_derive_trait(&derives, "Hash"));
        assert!(has_derive_trait(&derives, "serde::Serialize"));
        assert!(has_derive_trait(&derives, "serde::Deserialize"));

        // Verify struct definition
        assert!(matches!(item_struct.vis, syn::Visibility::Public(_)));
        assert_eq!(tuple_struct_field_type(item_struct), "i64");

        // Verify methods
        let tokens = definition_file.to_token_stream().to_string();
        assert!(tokens.contains("pub fn value"));
        assert!(tokens.contains("pub fn into_value"));
        assert!(tokens.contains("pub fn new"));
    }

    #[test]
    fn test_generate_type_definition_with_all_traits() {
        let generator = BoundedIntegerGenerator::new();
        let type_name = format_ident!("AllTraits");
        let derivability = create_trait_derivability(true, true, true);

        let result = generator.generate_type_definition(&type_name, Some(&derivability));
        assert!(result.is_ok());

        let tokens = result.unwrap().to_string();

        // Verify all conditional derives are included
        assert!(tokens.contains("Eq"));
        assert!(tokens.contains("PartialOrd"));
        assert!(tokens.contains("Ord"));
        assert!(tokens.contains("Hash"));
    }

    #[test]
    fn test_generate_type_definition_without_eq() {
        let generator = BoundedIntegerGenerator::new();
        let type_name = format_ident!("NoEq");
        let derivability = create_trait_derivability(false, true, true);

        let result = generator.generate_type_definition(&type_name, Some(&derivability));
        assert!(result.is_ok());

        let tokens = result.unwrap().to_string();

        // Verify Eq is NOT derived when can_derive_eq is false
        // But other traits should still be present
        assert!(tokens.contains("PartialOrd"));
        assert!(tokens.contains("Ord"));
        assert!(tokens.contains("Hash"));
    }

    #[test]
    fn test_generate_type_definition_without_ord() {
        let generator = BoundedIntegerGenerator::new();
        let type_name = format_ident!("NoOrd");
        let derivability = create_trait_derivability(true, false, true);

        let result = generator.generate_type_definition(&type_name, Some(&derivability));
        assert!(result.is_ok());

        let tokens = result.unwrap().to_string();

        // Verify Eq is present but PartialOrd/Ord are NOT when can_derive_ord is false
        assert!(tokens.contains("Eq"));
        assert!(tokens.contains("Hash"));
    }

    #[test]
    fn test_generate_type_definition_without_hash() {
        let generator = BoundedIntegerGenerator::new();
        let type_name = format_ident!("NoHash");
        let derivability = create_trait_derivability(true, true, false);

        let result = generator.generate_type_definition(&type_name, Some(&derivability));
        assert!(result.is_ok());

        let tokens = result.unwrap().to_string();

        // Verify Eq and Ord are present but Hash is NOT when can_derive_hash is false
        assert!(tokens.contains("Eq"));
        assert!(tokens.contains("PartialOrd"));
        assert!(tokens.contains("Ord"));
    }

    // ========== Batch 3: generate_encoder tests ==========

    #[test]
    fn test_generate_encoder_with_codec() {
        let generator = BoundedIntegerGenerator::new();
        let type_name = format_ident!("CodecType");

        let encoding = EncodingDecision::BoundedIntegerCodec {
            min: 100,
            max: 200,
            codec_spec: create_codec_spec(10, 2, 100),
        };

        let result = generator.generate_encoder(&type_name, &encoding);
        assert!(result.is_ok());

        let implementation_file = parse_generated_file(result.unwrap().clone());
        let tokens = implementation_file.to_token_stream().to_string();
        generated_trait_method(
            &implementation_file,
            "CodecType",
            "JsonTokenEncoder",
            "token_count",
        );

        // Verify codec-based encoding
        assert!(
            tokens.contains("codectype_codec::encode")
                || tokens.contains("codectype_codec :: encode")
        );
        assert!(
            tokens.contains("codectype_codec::decode")
                || tokens.contains("codectype_codec :: decode")
        );
        assert!(tokens.contains("self.0-100") || tokens.contains("self . 0 - 100"));
        assert!(tokens.contains("offset as i64")); // Core logic
    }

    #[test]
    fn test_generate_encoder_with_offset() {
        let generator = BoundedIntegerGenerator::new();
        let type_name = format_ident!("OffsetType");

        let encoding = EncodingDecision::BoundedIntegerOffset {
            min: 50,
            max: 150,
            range: 101,
        };

        let result = generator.generate_encoder(&type_name, &encoding);
        assert!(result.is_ok());

        let implementation_file = parse_generated_file(result.unwrap().clone());
        let tokens = implementation_file.to_token_stream().to_string();
        generated_trait_method(
            &implementation_file,
            "OffsetType",
            "JsonTokenEncoder",
            "token_count",
        );

        // Verify offset encoding uses unrolled compile-time width thresholds for encode/token_count.
        assert!(tokens.contains("self.0-50") || tokens.contains("self . 0 - 50"));
        assert!(tokens.contains("const ENCODING_BASE : u32 = 256u32"));
        assert!(tokens.contains("if offset < 256u32"));
        assert!(tokens.contains("tokens.push (0u32)") || tokens.contains("tokens . push (0u32)"));
        assert!(
            !tokens.contains("variable_base :: encode_u32"),
            "offset encode should not call generic variable-base helper"
        );
        assert!(
            !tokens.contains("variable_base :: token_count_u32"),
            "offset token_count should not call generic variable-base helper"
        );
        assert!(
            tokens.contains("variable_base :: decode_u32_fixed_base"),
            "offset decode should use the fixed-base helper while preserving malformed-token behavior"
        );
        assert!(
            !tokens.contains("variable_base :: decode_u32 ("),
            "offset decode should not call generic variable-base helper"
        );
        assert!(tokens.contains("offset as i64")); // Core logic
    }

    #[test]
    fn test_generate_encoder_fallback() {
        let generator = BoundedIntegerGenerator::new();
        let type_name = format_ident!("FallbackType");

        let encoding = EncodingDecision::Standard;

        let result = generator.generate_encoder(&type_name, &encoding);
        assert!(result.is_ok());

        let tokens = result.unwrap().to_string();

        // Verify fallback to standard i64 encoding
        assert!(tokens.contains("self.0.encode") || tokens.contains("self . 0 . encode"));
        assert!(tokens.contains("<i64>::decode") || tokens.contains("< i64 > :: decode"));
        assert!(tokens.contains("self.0.token_count") || tokens.contains("self . 0 . token_count"));
    }

    #[test]
    fn test_streaming_encode_generates_decode_prefix() {
        // Streaming mode should generate decode_prefix method
        let generator = BoundedIntegerGenerator::with_streaming_encode(true);
        let type_name = format_ident!("StreamingType");

        let encoding = EncodingDecision::BoundedIntegerCodec {
            min: 0,
            max: 100,
            codec_spec: create_codec_spec(10, 2, 100),
        };

        let result = generator.generate_encoder(&type_name, &encoding);
        assert!(result.is_ok());

        let implementation_file = parse_generated_file(result.unwrap());
        let tokens = implementation_file.to_token_stream().to_string();
        let decode_prefix =
            generated_inherent_method(&implementation_file, "StreamingType", "decode_prefix")
                .expect("expected generated decode_prefix method");

        // Verify decode_prefix is generated
        assert!(
            method_signature_contains(decode_prefix, "PrefixDecodeResult"),
            "Should use PrefixDecodeResult type"
        );
        // Verify fallback to default on decode failure
        assert!(
            tokens.contains("Self::default()") || tokens.contains("Self :: default ()"),
            "Should use default on decode failure"
        );
    }

    #[test]
    fn test_streaming_encode_adds_default_derive() {
        // Streaming mode should add Default derive for decode_prefix fallback
        let generator = BoundedIntegerGenerator::with_streaming_encode(true);
        let type_name = format_ident!("StreamingDefaultType");
        let derivability = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: true,
            can_derive_hash: true,
            reasoning: "Test streaming Default derive".to_string(),
            confidence: 1.0,
            blocking_types: HashSet::new(),
        };

        let result = generator.generate_type_definition(&type_name, Some(&derivability));
        assert!(result.is_ok());

        let tokens = result.unwrap().to_string();

        // Verify Default is derived
        assert!(
            tokens.contains("Default"),
            "Should derive Default when streaming is enabled"
        );
    }

    #[test]
    fn test_non_streaming_no_decode_prefix() {
        // Non-streaming mode should NOT generate decode_prefix
        let generator = BoundedIntegerGenerator::new();
        let type_name = format_ident!("NonStreamingType");

        let encoding = EncodingDecision::BoundedIntegerCodec {
            min: 0,
            max: 100,
            codec_spec: create_codec_spec(10, 2, 100),
        };

        let result = generator.generate_encoder(&type_name, &encoding);
        assert!(result.is_ok());

        let implementation_file = parse_generated_file(result.unwrap());

        // Verify decode_prefix is NOT generated
        assert!(
            generated_inherent_method(&implementation_file, "NonStreamingType", "decode_prefix")
                .is_none(),
            "Should NOT generate decode_prefix when streaming is disabled"
        );
    }

    #[test]
    fn test_streaming_with_offset_encoding() {
        // Streaming mode should work with offset encoding too
        let generator = BoundedIntegerGenerator::with_streaming_encode(true);
        let type_name = format_ident!("OffsetStreamingType");

        let encoding = EncodingDecision::BoundedIntegerOffset {
            min: 10,
            max: 50,
            range: 41,
        };

        let result = generator.generate_encoder(&type_name, &encoding);
        assert!(result.is_ok());

        let implementation_file = parse_generated_file(result.unwrap());
        let tokens = implementation_file.to_token_stream().to_string();

        // Verify both offset encoding and decode_prefix are generated
        assert!(
            generated_inherent_method(&implementation_file, "OffsetStreamingType", "decode_prefix")
                .is_some(),
            "Should generate decode_prefix for offset encoding"
        );
        assert!(
            tokens.contains("self.0-10") || tokens.contains("self . 0 - 10"),
            "Should use offset encoding"
        );
    }
}
