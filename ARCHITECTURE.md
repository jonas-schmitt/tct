# TCT Multi-Pass Architecture

## Overview

TCT uses a clean multi-pass compilation system with pure separation of concerns. Each pass has a single responsibility and specialized data structures. The four-pass architecture separates encoding decisions from code generation, enabling vocabulary-constrained optimization.

## Design Principles

1. **Single Responsibility**: Each pass does exactly one thing
2. **Pure Functions**: No side effects, clear inputs/outputs
3. **Immutable Data Flow**: Data flows forward, no clone/restore patterns
4. **Extensibility**: Easy to add new passes (optimization, validation, etc.)
5. **Testability**: Each pass is independently testable

## Core Architecture

### Four Specialized Systems

```rust
// 1. PURE DISCOVERY - No code generation
struct SchemaWalker {
    discovery: SchemaDiscovery,  // Pure schema structure analysis
}

// 2. PURE ANALYSIS - No code generation
struct TypeAnalyzer {
    discovery: SchemaDiscovery,     // Input
    analysis: TypeAnalysis,         // Output: dependencies, traits
}

// 3. PURE OPTIMIZATION - Encoding decisions only (no code)
struct OptimizationPhase {
    config: OptimizationConfig,     // Input: constraints
    strategy: EncodingStrategy,     // Output: encoding decisions and explicit generation metadata
}

// 4. PURE GENERATION - Only code generation
struct CodeGenerator {
    discovery: SchemaDiscovery,     // Input
    analysis: TypeAnalysis,         // Input
    encoding_strategy: Option<EncodingStrategy>,  // Input (optional)
    generation: CodeGeneration,     // Output: TokenStreams, definitions
}
```

### Data Flow

```
JSON Schema + OptimizationConfig
    ↓
SchemaWalker::walk() → SchemaDiscovery
    ↓
TypeAnalyzer::analyze() → TypeAnalysis
    ↓
OptimizationPhase::run() → EncodingStrategy
    ↓
CodeGenerator::generate() → CodeGeneration
    ↓
Grammar
```

**Key**: Each pass consumes immutable data and produces new data. Encoding decisions are data, not code.

## Schema Composition Contract

`allOf` is treated as schema intersection. The compiler supports this exactly
for object-oriented shapes it can either merge into generated members or enforce
with generated validation. Unsupported shapes must fail during compilation with
schema-path context rather than compiling an approximate tokenizer.

Supported merge behavior:

- Object subschema properties are merged into the containing generated type.
- Duplicate properties are merged only when their validation-relevant schemas
  are structurally identical; conflicting duplicates are rejected.
- `required` names are unioned across the containing object, mergeable
  subschemas, nested mergeable `allOf` entries, and local `$ref` object
  subschemas. A required name that does not map to a generated member is
  rejected.
- Root or containing-object `additionalProperties` decides whether the
  generated type has a catch-all map. `additionalProperties` constraints inside
  `allOf` subschemas are preserved as generated validation, not silently
  converted into extra catch-all members.
- Local `$ref` object subschemas can participate in the merge when the wrapper
  carries no validation siblings. Vendored external `$ref` entries are enforced
  through generated JSON Schema validation; unvendored external refs are
  rejected.

Supported validation-only `allOf` entries include the currently modeled
`not: { required: [...] }`, the workflow-dispatch style `if`/`then` shape, and
local-ref `anyOf` validation branches. These entries constrain serde
deserialization and generated round trips without introducing generated
members. Generated JSON Schema validators for these validation-only checks are
cached at first use, but each deserialization still validates the incoming JSON
before a typed value can be encoded. Generated decode also revalidates decoded
values for these validation-only constraints before returning, so malformed
token streams cannot bypass constraints that serde would reject. Unsupported
composition or validation keywords inside `allOf`, such as unmodeled `oneOf`,
validation siblings on `$ref`, recursive allOf reference cycles, and non-object
merge subschemas, are explicit compile-time errors.

## Pass-Specific Data Structures

### SchemaDiscovery - Pure Schema Analysis

```rust
pub struct SchemaDiscovery {
    /// All discovered type locations and their schema information
    pub discovered_types: HashMap<String, TypeDiscovery>,
    /// Root schema information
    pub root_info: Option<RootSchemaInfo>,
    /// Definitions section analysis
    pub definitions: HashMap<String, DefinitionInfo>,
    /// Statistics about discovery
    pub stats: DiscoveryStats,
    /// JSON pointer to type name mapping
    pub ref_mappings: HashMap<String, String>,
}

pub struct TypeDiscovery {
    pub type_name: String,
    pub type_kind: TypeKind,                    // Struct, Enum, NestedStruct
    pub schema_location: SchemaLocation,        // Where in schema
    pub schema_object: Map<String, Value>,      // Original schema
    pub members: Vec<MemberInfo>,               // Properties/variants
    pub direct_references: HashSet<String>,     // Types this references
    pub needs_boxing: bool,                     // Set by analysis pass
}
```

**NO TokenStreams, NO code generation, ONLY schema structure analysis**

### TypeAnalysis - Dependency & Trait Analysis

```rust
pub struct TypeAnalysis {
    /// Complete dependency graph with cycle detection
    pub dependency_graph: TypeDependencyGraph,
    /// Trait derivability analysis for each type
    pub trait_derivability: HashMap<String, TraitDerivability>,
    /// Types requiring Box wrapping (from cycle detection)
    pub boxed_types: HashSet<String>,
    /// Type generation order (topological sort)
    pub generation_order: Vec<String>,
    /// Decode strategies for schema-aware generated decoding
    pub decode_strategies: HashMap<String, DecodeStrategy>,
    /// Analysis statistics
    pub stats: AnalysisStats,
}

pub struct TraitDerivability {
    pub can_derive_eq: bool,
    pub can_derive_ord: bool,
    pub can_derive_hash: bool,
    pub reasoning: String,
    pub confidence: f64,
}
```

**NO code generation, ONLY mathematical analysis of discovered types**

### EncodingStrategy - Encoding Decisions (Pure Data)

```rust
pub struct EncodingStrategy {
    /// Encoding decision for each type
    pub decisions: HashMap<TypePath, EncodingDecision>,
    /// Explicit member type refinements consumed by generation
    pub member_type_refinements: HashMap<MemberPath, MemberTypeRefinement>,
    /// Optional structural vocabulary cap (`max_vocab`) the strategy was planned with
    pub max_vocabulary_size: Option<usize>,
    /// Optimization statistics
    pub stats: OptimizationStats,
}

pub enum EncodingDecision {
    /// Standard encoding (no optimization decision for this type). Enums still
    /// emit their variant index as one token.
    Standard,

    // Enum encodings
    /// Enum under the enum pass: one-token index, emitted exactly like Standard
    EnumIndex { variants: Vec<String> },
    /// Enum with codec encoding (lossless when the variant count exceeds `max_vocab`)
    EnumCodec { variants: Vec<String>, codec_spec: CodecSpec },
    /// Single-variant enum: a constant, zero tokens
    SingleVariantEnum { variant: String },

    // Integer encodings
    /// Bounded integer with standard offset encoding
    BoundedIntegerOffset { min: i64, max: i64, range: u32 },
    /// Bounded integer with codec encoding (lossless when the range exceeds `max_vocab`)
    BoundedIntegerCodec { min: i64, max: i64, codec_spec: CodecSpec },
    /// Integer with minimum == maximum: a constant, zero tokens
    ConstantInteger { value: i64 },

    // String encodings
    /// Format-specific string encoding (O3 only); malformed values fall back to
    /// the string encoding
    FormatOptimized { format: StringFormat, fallback_to_string: bool },

    // Struct encodings
    /// Bit-packed presence flags for optional `Option<T>` members
    BitPackedStruct { optional_field_count: usize, presence_tokens_needed: usize },

    // Pattern encodings
    /// Pattern-optimized encoding (future)
    PatternOptimized { pattern: String, strategy: PatternEncodingStrategy },

    // BoundedInteger and FixedSizeArray are also defined; OptimizationPhase does
    // not produce them
}

pub struct CodecSpec {
    pub token_width: usize,      // Number of tokens needed
    pub base: u32,                // Codec base (`max_vocab` for enum/integer codecs)
    pub max_value: u32,           // Maximum value to encode
    pub is_power_of_2: bool,      // Enables bit-shift optimization
    pub divisors: Vec<u32>,       // Precomputed divisors for each position
}
```

**NO code generation, ONLY encoding decisions (pure data)**

### CodeGeneration - Pure Code Generation

```rust
pub struct CodeGeneration {
    /// Generated TokenStreams for each type
    pub type_definitions: HashMap<String, TypeCodeGeneration>,
    /// Final definitions structure
    pub definitions: Definitions,
    /// Root struct generation
    pub root_generation: Option<RootCodeGeneration>,
    /// Generation statistics
    pub stats: GenerationStats,
}

pub struct TypeCodeGeneration {
    pub type_name: String,
    pub definition_tokens: TokenStream,
    pub implementation_tokens: TokenStream,
    pub dependencies: Vec<String>,
    pub generated_item_kind: GeneratedItemKind,
    pub has_token_encoder_impl: bool,
}
```

`generated_item_kind` is code-generation metadata, not raw schema kind:
newtype wrappers are routed as structs, root/simple composition can be routed as
enums, and references can be routed as type aliases. Final `Definitions`
assembly must use this metadata instead of classifying generated token text.

**ONLY code generation, consumes discovery and analysis data**

## Pass Implementations

### 1. SchemaWalker (Pure Discovery)

```rust
impl SchemaWalker {
    pub fn walk(schema: &Value, root_name: &str) -> Result<SchemaDiscovery> {
        let mut walker = Self::new();

        // Walk root
        walker.discover_root(schema, root_name)?;

        // Walk definitions
        if let Some(definitions) = schema.get("definitions") {
            walker.discover_definitions(definitions)?;
        }

        Ok(walker.discovery)
    }

    fn discover_root(&mut self, schema: &Value, name: &str) -> Result<()> {
        // Analyze root schema structure
        // Register root type in discovery
        // NO code generation
    }

    fn discover_property(&mut self, prop_schema: &Map<String, Value>,
                        parent: &str, prop_name: &str) -> Result<()> {
        // Recursive schema analysis
        // Identify: objects → structs, oneOf → enums, etc.
        // Register discovered types
        // NO TokenStreams generated
    }
}
```

### 2. TypeAnalyzer (Pure Analysis)

```rust
impl TypeAnalyzer {
    pub fn analyze(discovery: SchemaDiscovery,
                   optimization_config: OptimizationConfig) -> Result<TypeAnalysis> {
        let mut analyzer = Self::new(discovery, optimization_config);

        // Build dependency graph
        analyzer.build_dependency_graph()?;

        // Detect cycles, mark types for Box wrapping
        analyzer.detect_cycles()?;

        // Run trait derivability analysis
        analyzer.analyze_trait_derivability()?;

        // Compute generation order
        analyzer.compute_generation_order()?;

        // Compute decode strategies for schema-aware decoding
        analyzer.compute_decode_strategies()?;

        analyzer.finalize_statistics();

        Ok(analyzer.analysis)
    }
}
```

### 3. CodeGenerator (Pure Generation)

```rust
impl CodeGenerator {
    pub(crate) fn generate(discovery: SchemaDiscovery,
                           analysis: TypeAnalysis,
                           encoding_strategy: Option<EncodingStrategy>,
                           options: CodeGeneratorOptions) -> Result<CodeGeneration> {
        // Apply the member type refinements recorded by the optimization phase
        let discovery = Self::apply_member_type_refinements(discovery, encoding_strategy.as_ref());
        let mut generator = Self::new(discovery, analysis, encoding_strategy,
                                      /* constraints, streaming, encoding base, ... from options */);

        // Generate types in dependency order
        for type_name in &generator.analysis.generation_order.clone() {
            generator.generate_type(type_name)?;
        }

        // Generate pattern validators and the root
        generator.generate_pattern_validators()?;
        generator.generate_root()?;

        // Assemble the definitions and emit the BASE_VOCAB_SIZE constant
        generator.generation.build_definitions();
        /* ... */

        Ok(generator.generation)
    }

    fn generate_type(&mut self, type_name: &str) -> Result<()> {
        let discovery = self.discovery.get_type(type_name).unwrap();

        match discovery.type_kind {
            TypeKind::Struct => self.generate_struct(discovery),
            TypeKind::Enum => self.generate_enum(discovery),
            // ... other types
        }
    }
}
```

## Structural Codec Generation

### Overview

TCT implements **lossless multi-token encoding** for vocabulary-constrained types through compile-time specialized codec modules. This system resolves everything at compile time and generates brutally efficient code with zero runtime overhead.

### Problem: Encoding-Base Constraints

Generated tokenizers must keep every emitted token ID below `VOCAB_SIZE` while
preserving the represented JSON value. When a finite structural range (an enum's
variant count or a bounded integer's range) is larger than the structural
vocabulary cap, TCT uses a fixed-width multi-token codec rather than colliding
values into one token.

Two settings are involved, and they are not interchangeable:

- `max_vocab` (`TctCompiler::with_max_vocabulary()`, CLI `--max-vocab`, at least
  257) selects the codecs. The enum and bounded-integer passes encode a type with
  a codec when its variant count or range exceeds `max_vocab`, and `max_vocab`
  itself is the codec base (it need not be a power of two). Without `max_vocab`,
  or when those passes do not run, enums and bounded integers never use a codec.
- `max_base_vocab` (`TctCompiler::with_max_base_vocab()`, test environment
  variable `MAX_BASE_VOCAB`) sets the encoding base of the generated fixed-width
  multi-token integer encodings (for example collection lengths and
  bounded-integer offsets): the largest power of two not exceeding it, never
  below 256, so the default base remains 256. It does not cap anything.
  `BASE_VOCAB_SIZE` is the maximum of the string-token capacity (257 for UTF-8),
  the structural token requirement, and the encoding base, so a larger
  `max_base_vocab` raises the base vocabulary, which is never below 257 tokens
  with UTF-8 strings (256 with ASCII).

**Key distinction**:
- `max_vocab=257` with 200 enum variants → one-token index.
- `max_vocab=257` with 385 enum variants → lossless two-token codec in base 257.
- `max_base_vocab=257` → encoding base 256, identical to the default; it does
  not turn an enum with more than 256 variants into a codec.
- BPE target vocabulary controls how many trained merge IDs are available above
  the base/structural range; it must not change schema semantics.

### Codec Architecture

#### Core Principle: Compile-Time Specialization

Each vocabulary-constrained type gets a specialized codec module generated at compile time:

```rust
// Generated for Status enum (1,000 variants, codec base = max_vocab = 512)
mod status_codec {
    // Precomputed constants (compile-time)
    pub const TOKEN_WIDTH: usize = 2;         // ceil(log_512(1000)) = 2
    pub const BASE: u32 = 512;
    pub const MAX_VALUE: u32 = 999;
    pub const SHIFT: u32 = 9;
    pub const MASK: u32 = 0x1FF;

    #[inline(always)]
    pub fn encode(value: u32, tokens: &mut Vec<u32>) {
        // UNROLLED - no loops, all decisions at compile time
        tokens.push((value >> 9) & MASK);  // High digit
        tokens.push(value & MASK);         // Low digit
    }

    #[inline(always)]
    pub fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<u32, DecodeError> {
        // UNROLLED - exactly 2 reads, defensive modulo for LLM errors
        let d1 = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream)? % 512;
        let d0 = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream)? % 512;
        Ok((d1 * 512 + d0) % 1000)  // Reconstruct value with bounds check
    }
}
```

#### Power-of-2 Optimization

When `BASE` is a power of 2, generated encode paths can use bit operations
instead of division/modulo. Decode still uses modulo normalization for
consistent recovery semantics:

```rust
// Generated for Priority (600 variants, codec base = max_vocab = 512)
mod priority_codec {
    pub const TOKEN_WIDTH: usize = 2;
    pub const BASE: u32 = 512;
    pub const SHIFT: u32 = 9;      // log₂(512)
    pub const MASK: u32 = 0x1FF;   // 512 - 1

    #[inline(always)]
    pub fn encode(value: u32, tokens: &mut Vec<u32>) {
        tokens.push((value >> 9) & MASK);  // Bit shift replaces division
        tokens.push(value & MASK);          // Bit AND replaces modulo
    }

    #[inline(always)]
    pub fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<u32, DecodeError> {
        // Decode stays arithmetic: each digit is normalized with % BASE and
        // the reconstructed value with % 600.
        let d1 = tokens.next()? % BASE;
        let d0 = tokens.next()? % BASE;
        Ok((d1 * BASE + d0) % 600)
    }
}
```

### CodecSpec: Precomputed Codec Metadata

```rust
pub struct CodecSpec {
    /// Number of tokens needed to encode max_value
    /// Calculated as: ceil(log_base(max_value + 1))
    pub token_width: usize,

    /// Codec base: the `max_vocab` cap for enum and bounded-integer codecs
    /// (not necessarily a power of 2)
    pub base: u32,

    /// Maximum value to encode (e.g., variant_count - 1)
    pub max_value: u32,

    /// Whether base is power of 2 (enables bit-shift optimization)
    pub is_power_of_2: bool,

    /// Bit shift amount (if power of 2)
    pub shift: Option<u32>,

    /// Bit mask (if power of 2)
    pub mask: Option<u32>,

    /// Divisors for each digit position (non-power-of-2)
    /// Example: base=10, width=3 → [100, 10, 1]
    pub divisors: Vec<u32>,
}
```

### Optimization Priority System

The optimization pass applies encoding decisions based on priority. For enums,
when the enum pass is active (a one-variant enum is decided first, as
`SingleVariantEnum` with zero tokens):

**Priority 1: Finite Range Exceeds `max_vocab`** (lossless multi-token)
```rust
if let Some(max_vocab) = max_vocab && variant_count > max_vocab {
    return EncodingDecision::EnumCodec {
        variants,
        codec_spec: CodecSpec::calculate(variant_count - 1, Some(max_vocab))
    };
}
```

**Priority 2: No Constraint** (one-token index)
```rust
return EncodingDecision::EnumIndex { variants };
```

Without the enum pass the decision is `Standard`, which emits the same one-token
index.

### Type Coverage

Structural codec generation applies to:

1. **Enums**: `EnumCodec` for variant counts above `max_vocab`
2. **Bounded Integers**: `BoundedIntegerCodec` for ranges above `max_vocab`
3. **Patterns**: Reserved future work; `patternProperties` validation is
   schema-driven and not a direct optimization-pass decision.

### Benefits

- **Compile-time decisions**: Schema-derived codec choices are made before
  generated runtime execution.
- **Loop-free fixed-width codecs**: Generated code can unroll known-width base
  encoding operations.
- **Predictable token counts**: Codec width is derived from the schema range or
  variant count.
- **LLVM-friendly arithmetic**: Generated hot paths use simple constants and
  arithmetic operations.
- **Lossless structural encoding**: The base-N representation preserves the
  supported schema value.
- **Recovery-oriented decode**: Modulo operations map out-of-range model tokens
  back into valid bounded choices.
- **Type-safe generated code**: Rust types encode the supported schema shape.

### Generated Code Example

For enum `Status` with 1,000 variants and `max_vocab=512`:

```rust
// 1. Codec module generation
mod status_codec { /* ... shown above ... */ }

// 2. Enum encoding implementation
impl Status {
    pub fn encode(&self, tokens: &mut Vec<u32>) {
        let variant_index = match self {
            Self::Variant0 => 0,
            Self::Variant1 => 1,
            // ... 998 more variants
            Self::Variant999 => 999,
        };
        status_codec::encode(variant_index, tokens);
    }

    pub fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let variant_index = status_codec::decode(tokens)?;
        match variant_index {
            0 => Ok(Self::Variant0),
            1 => Ok(Self::Variant1),
            // ... 998 more variants
            999 => Ok(Self::Variant999),
            // Recovery decode: modulo keeps the index in range. The strict
            // decoder returns DecodeError::InvalidTokenInContext instead.
            _ => unreachable!("modulo keeps the index in range"),
        }
    }
}
```

### Compile-Time vs Runtime Trade-offs

**Compile-Time Work** (FREE - done once):
- Calculate optimal token width
- Precompute divisors and masks
- Generate specialized codec for each type
- Unroll all loops in generated code

**Runtime Work** (EXPENSIVE - done per value):
- Simple arithmetic (add, multiply, modulo)
- Array access (push/next)
- Pattern matching (enum variants)

**Philosophy**: Spend unlimited time at compile time to achieve minimal runtime cost. Generated code should be as efficient as hand-optimized assembly.

### Defensive Decoding Strategy

**Design Choice**: TCT uses **consistent modulo-based decoding** for maximum LLM robustness.

#### Why Modulo Everywhere?

LLM-generated tokens may fall outside expected ranges due to:
- Model sampling errors
- Temperature/top-k/top-p variations
- Fine-tuning on different token distributions
- Out-of-distribution generation scenarios

TCT handles this gracefully through **layered modulo defense**:

```rust
// 1. PER-DIGIT MODULO - Normalize each token as it's read
let digit = tokens.next()? % BASE;  // Ensure digit ∈ [0, BASE)

// 2. FINAL MODULO - Clamp reconstructed value to valid range
let value = (d1 * BASE + d0) % (MAX_VALUE + 1);  // Ensure value ∈ [0, MAX_VALUE]

// 3. ENUM WRAPAROUND - Wrap variant indices
let variant_index = discriminator % variant_count;  // Map to valid variant

// 4. PRIMITIVE MODULO - Normalize boolean tokens
let boolean = token % 2;  // Map to {0, 1}
```

#### Power-of-2 Base Handling

Even for power-of-2 bases (such as 256 or 512), TCT prioritizes
**consistent modulo decode** over bitwise-only decode:

**Bitwise AND Path** (disabled by default):
```rust
let digit = token & MASK;           // Single-layer defense (masking)
let value = (d1 << SHIFT) | d0;
Ok(value % (MAX_VALUE + 1))         // Final modulo only
```

**Modulo Path** (always used):
```rust
let digit = token % BASE;           // First-layer defense (per-digit)
let value = d1 * BASE + d0;
Ok(value % (MAX_VALUE + 1))         // Second-layer defense (final)
```

**Decision Rationale**:
- **Bitwise AND**: Single defensive layer for power-of-two ranges
- **Modulo**: Slightly more general and keeps the same recovery semantics across
  power-of-two and non-power-of-two ranges
- **Priority**: LLM robustness > micro-optimization

For a 1,000-variant enum with base 512:
- Bitwise: `token & 0x1FF` then final modulo (1 layer)
- Modulo: `token % 512` + final modulo (2 layers)

**Result**: Any out-of-range token gracefully wraps to valid value, enabling reliable LLM generation without "Invalid token" errors.

## Public API

### TctCompiler Pipeline

```rust
impl TctCompiler {
    pub fn compile(&self) -> Result<Grammar> {
        let optimization_config = /* derive config from compiler settings */;

        let mut orchestrator = PassOrchestrator::new();
        orchestrator.add_pass(DiscoveryPass::with_root_name(self.type_name.clone()));
        orchestrator.add_pass(AnalysisPass::new());
        orchestrator.add_pass(OptimizationPassWrapper::new(optimization_config.clone()));
        orchestrator.add_pass(GenerationPass::new());

        let context = orchestrator.run_all_with_config(&self.schema, optimization_config)?;
        /* convert pass context into Grammar */
    }
}
```

**Clean API**: `schema -> discovery -> analysis -> optimization -> generation -> grammar`

## Token Encoding and Decode Contract

Generated tokenizers encode JSON values as schema-local `u32` streams. Tokens
are untyped at the base layer: the generated code supplies the schema context
that gives each token position its meaning. This avoids global type tags in the
common path while keeping encode/decode logic specialized to the compiled
schema.

Core rules:

- `encode()` emits canonical token sequences. It must not use modulo recovery or
  lossy normalization.
- `decode()` is recovery-oriented for token values where the schema has a finite
  choice, such as booleans, enum discriminators, optional-field states, and BPE
  recovery. It still errors on malformed strings, EOF, collection caps, schema
  validation failures, and other cases where the stream cannot represent the
  expected value.
- `decode_full()` uses the same recovery semantics as `decode()` for one value,
  then rejects any unconsumed trailing tokens.
- `decode_prefix()` is truncation-tolerant and may return partial/defaulted
  values. It is not a strict validator.
- Lower-level callers that manage their own iterator may still check token
  consumption directly.
- Base string encoding uses UTF-8 byte tokens followed by an end marker. The
  default base vocabulary reserves byte tokens `0..=255` and
  `END_STRING_MARKER`; ASCII restricted-input mode uses the analogous ASCII
  byte range and rejects non-ASCII input during encoding.
- Generated `oneOf` and `anyOf` data variants use schema-specific
  discriminator tokens before variant payloads. Those discriminators are part of
  the schema-local structural vocabulary.
- Vocabulary accounting is invariant: emitted token IDs must be below
  `VOCAB_SIZE`, and the base range (shared by byte, end-marker, and structural
  tokens), BPE merge IDs, and `PAD_TOKEN` must not overlap.

Modulo normalization belongs only in decode/recovery paths. Canonical JSON
round trips must preserve semantic data: `JSON -> tokens -> JSON` should produce
an equivalent JSON value for supported schemas.

### Strict Decode API

The strict token-value API is additive. Existing `decode()`, `decode_full()`,
and `decode_prefix()` remain recovery APIs so model-generation workflows keep
their current behavior. Generated tokenizers expose these separate entry points
for callers that need malformed finite choices rejected:

- `decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError>`:
  decode one value while rejecting out-of-range token values instead of applying
  modulo recovery.
- `decode_strict_full(tokens: impl IntoIterator<Item = u32>) -> Result<Self, DecodeError>`:
  combine strict token-value semantics with whole-stream consumption.

Strict token-value decoding must reject, rather than normalize, invalid finite
choices: boolean tokens outside `0..=1`, enum or union discriminators outside
their variant range, optional-field state tokens outside the generated state
range, bounded-integer codec digits outside the generated base, and BPE tokens
that are neither base tokens nor known merge IDs. String byte ranges, EOF,
collection caps, UTF-8 validation, schema validation, and generated
`additionalProperties`/`allOf` validators keep the same rejection behavior as
the recovery API. Keep strict coverage incremental and tested with
generated-crate contracts whenever a new generated path is made strict.

### Generated Struct Field Order Contract

Schema-local struct token streams encode fields in the generated member order,
except for bit-packed structs (below). Full decode must consume fields in that
same order. `DecodeStrategy.field_order` records that order for generation and
compatibility evidence; it is the natural discovered member order, not an
independent decode-reordering optimization hint. If a future optimization
intentionally changes member order, encode order, full decode order,
`decode_prefix()` field progress, token-diff structural metadata, and schema
evolution compatibility policy must be updated together.

Bit-packed structs (the `BitPackedStruct` decision) are the exception. They emit
the presence-mask group(s) first, then every member that is not bit-packed
(required members and three-state `OptionalField<T>` members) in member order,
then the values of the present bit-packed optional members in declaration order.
For them `field_order` stays the member order rather than the emitted order, and
the generator's check that `field_order` equals the encode order does not cover
them.

### Prefix And Gap Completion Contracts

`decode_prefix()` is the generated prefix-completion API. Rust generated types
receive `decode_prefix(tokens: &mut impl Iterator<Item = u32>) ->
PrefixDecodeResult<Self>` only when streaming generation is enabled. Python
wheels expose `decode_prefix(partial_tokens)` only when built with
`--streaming`.

The prefix API accepts a truncation of a canonical schema-local token sequence.
It decodes in generated order, returns defaults for fields that cannot yet be
decoded, and reports:

- `instance`: the partial value with generated defaults for undecoded fields.
- `fields_decoded`: how many fields or variants were completed before
  truncation stopped progress.
- `complete`: whether the value was fully decoded from the supplied prefix.
- `partial_field`: metadata for a truncated string or collection field when
  that partial field can be identified.

`decode_prefix()` is not a strict validator and is not a suffix-constrained
infill API. It may use the same recovery behavior as `decode()` for bounded
choices, and callers must not treat `complete=true` as proof that no trailing
tokens exist unless they also control or check the input iterator.

Generated gap metadata has a separate, narrower meaning. `encode_partial()`,
`GapInfo`, `PartialEncoding`, and `decode_to_first_gap()` describe a complete
token sequence where missing fields were filled with defaults, plus metadata
about those default-filled ranges. They do not carry the parser state needed to
decode a left prefix, constrain an arbitrary right suffix, and generate a valid
middle span.

Future suffix-constrained gapped completion needs an explicit parser-state
contract rather than reusing `decode_prefix()` metadata. At minimum that state
must identify the owner type and field or variant path, generated-order cursor,
presence-mask or optional-state progress, collection/string decoder progress,
token cursor, any suffix constraints that must still be satisfied, and BPE
boundary state when compressed token streams are involved.

A future infill API should therefore introduce a versioned, structured parser
state rather than extending `GapInfo`. The state should be derived from
pass-owned compilation metadata and should include schema/type identifiers,
tokenizer layout/config ID, raw-TCT versus BPE mode, left-prefix and
right-suffix token cursors, enum discriminator progress, object-member order,
optional or bit-packed presence state, collection lengths and item indices,
string byte/UTF-8 progress, pending suffix validators, compression or BPE
boundary state, and the validation mode used to accept or reject candidate
middle tokens. Candidate APIs should make these boundaries explicit, for
example by constructing state from `(left_prefix, right_suffix)`, enumerating
or validating next allowed raw tokens, advancing one token at a time, and
finalizing only when the middle span plus suffix forms a valid canonical token
sequence. The implementation contract must define field-aligned versus
intra-field gaps, strict versus recovery behavior, suffix validation,
trailing-token handling, and whether BPE is supported or rejected. Until that
exists, held-out gap fixtures are design fixtures only and must not be counted
as implemented gapped completion.

Future suffix-infill support should use this success/error contract:

- State construction accepts a left raw-TCT prefix and right raw-TCT suffix for
  the same tokenizer layout. BPE-compressed inputs are rejected unless the
  request includes enough merge-table provenance and boundary state to map
  unambiguously back to raw TCT token boundaries.
- The first supported gap class should be field-aligned. Intra-field gaps in
  strings, byte payloads, variable-width integers, arrays, maps, or recursive
  variants require explicit decoder-progress state before they are accepted.
- The default construction and finalization path is strict: invalid prefix
  tokens, invalid suffix tokens, malformed UTF-8, impossible enum variants,
  inconsistent presence or bit-packed state, and suffixes that cannot be
  reached from the prefix are errors. Any recovery or normalization behavior
  must be an explicitly named mode and must report the normalized token stream.
- Advancing a candidate middle token succeeds only if it is valid for the
  current parser state and still leaves at least one valid completion path to
  the supplied suffix. Rejected tokens should report token position, expected
  range or domain, and the owner type/path.
- Finalization succeeds only when `left_prefix + middle + right_suffix` is a
  canonical complete token sequence for the generated tokenizer, strict decode
  consumes the whole stream with no trailing tokens, and schema validation still
  passes. Otherwise the API reports whether the middle span is incomplete, the
  suffix is unreachable, trailing tokens remain, or validation failed.

Whole-stream decoding should be explicit about padding. The Rust whole-stream
entry points (`decode_full()`, `decode_strict_full()`) reject any trailing
token, including `PAD_TOKEN`. The Python raw `decode()` does not reject: it
returns `(json, tokens_consumed, total_tokens)` and leaves the surplus check to
the caller, while `decode_sequence()` rejects a payload with surplus tokens.
Batch or model wrappers that carry sequence lengths may strip padding before
calling the generated tokenizer. This avoids silently treating padding as data
while still supporting padded tensors at the wrapper layer.

### Sequence Boundary Wrappers

Raw schema-local APIs remain content-only. They do not emit BOS, EOS, schema
selection, or padding tokens, and their vocabulary contract remains
`VOCAB_SIZE = raw content range + PAD_TOKEN`. Model-facing wrappers add those
boundaries outside the raw stream.

The generated single-schema wrapper is the degenerate bundle form:

```text
[SEQUENCE_BOS_TOKEN, SEQUENCE_SCHEMA_TOKEN, offset payload..., SEQUENCE_EOS_TOKEN]
```

`SEQUENCE_BOUNDARY_VERSION` identifies this contract. `SEQUENCE_PAD_TOKEN`,
`SEQUENCE_BOS_TOKEN`, `SEQUENCE_EOS_TOKEN`, and `SEQUENCE_SCHEMA_TOKEN` occupy
the wrapper-special range, and raw payload tokens in `[0, PAD_TOKEN)` are mapped
by `SEQUENCE_PAYLOAD_OFFSET` before they appear in the wrapper sequence. Raw
`PAD_TOKEN` is not a valid payload token. This keeps wrapper specials from
colliding with schema-local bytes, structural tokens, or BPE merge IDs.

BPE remains payload-local. Generated wrappers apply the sequence boundary after
local BPE encode and remove plus validate it before local BPE decode. Wrapper
decode accepts trailing `SEQUENCE_PAD_TOKEN`s after EOS, rejects wrapper padding
before EOS, rejects non-padding tokens after EOS, rejects invalid schema tokens,
and verifies that raw decode consumed the complete unwrapped payload. Trailing
padding is not counted as consumed: the returned consumed count is the payload
length plus BOS, schema token, and EOS, while the total is the whole input
length.

Generated wrappers expose a serializable single-schema boundary manifest through
`sequence::metadata()` and Python `sequence_metadata_json()`. The manifest
records the artifact kind/version, boundary version, schema id and schema
token, raw `BASE_VOCAB_SIZE`, `VOCAB_SIZE`, and `PAD_TOKEN`, base-token and BPE
token ranges, raw payload range, payload offset, sequence vocabulary size,
sequence special-token reservations, whether `decode_prefix` is generated, and
BPE merge metadata when a merge table is embedded.

Multi-schema bundles generalize the same layout through the
`sequence_bundle` manifest contract: one wrapper PAD/BOS/EOS policy, one
schema-decision token per bundled schema, per-schema raw and global payload
ranges, optional schema sentinels and generated-artifact references, and
explicit metadata that maps each schema token to the generated tokenizer
artifact and raw local vocabulary. Payload ranges may be disjoint or shared by
schema, but every manifest must state the selected schema's global payload range
so generation masks can allow that range plus EOS after schema selection.

The bundle masking contract is intentionally explicit. Prompt, BOS, schema, and
PAD tokens are masked from labels; EOS is not masked, so the model can learn to
terminate; generation requires schema selection and then restricts logits to the
selected schema's global payload range plus EOS. This mirrors the downstream
Hugging Face/TCT direction without making model training part of this
repository.

Bundle payload wrapping is also contract-level: `wrap_payload_for_schema_id()`
and `wrap_payload_for_schema_token()` map raw schema-local payload IDs into the
selected schema's global payload range, while `unwrap_payload()` validates BOS,
schema token, EOS, trailing PAD placement, and selected-schema ranges before
returning local payload IDs for generated-tokenizer dispatch.

The first bundle implementation path assembles a pure-Python wrapper around
already generated schema wheels. This avoids combining multiple generated Rust
tokenizers in one crate while generated code still owns crate-root constants and
modules. The wrapper calls each schema wheel's raw content-only `encode` and
`decode`, then applies the bundle manifest at the model-facing sequence
boundary.

## Extensibility and Testing Strategy

New passes should be added through `PassOrchestrator` and should declare their
inputs and outputs explicitly. `PassOrchestrator` validates the whole pipeline
before executing any pass: duplicate pass names, missing dependencies,
self-dependencies, and out-of-order dependencies fail before context mutation.
Callers must provide a dependency-safe pipeline order. Keep the same boundary
rules:

- Discovery records schema structure and references.
- Analysis computes dependency, trait, and decode-strategy metadata.
- Optimization computes encoding decisions and explicit generation options, but
  does not replace or mutate `SchemaDiscovery`.
- Generation emits Rust tokens and generated-crate metadata.

Test the passes at their natural boundary. Discovery tests should assert
discovered structure without generated source. Analysis tests should assert
dependency order, boxing, trait derivability, and decode strategy. Optimization
tests should assert encoding decisions. Generation and end-to-end tests should
assert semantic generated-crate behavior instead of generated-source shape
unless the source shape is the intended public contract.

The public `tct::optimization_pass` module is compatibility-only. It remains for
`OptimizationLevel`, `OptimizationConfig`, named built-in pass selection, and
older callers, but it is not the extension model for new compiler work. Do not
add new custom `TokenStream`-returning passes there. New optimization behavior
should produce structured metadata or `EncodingStrategy` decisions that the
generation pass consumes explicitly.

`tests/architecture_boundary_test.rs` guards this separation by checking that
discovery, analysis, and optimization modules do not depend on generated
`TokenStream` data, quote/syntax generation APIs, or generated-source
classifiers. When adding new boundary modules, include them in that test rather
than relying on review alone.

## Engineering Priorities

The architecture follows a fixed priority order when making tradeoffs:

- **Correctness**: pure passes with typed errors and no silent fallbacks for
  unsupported schema features.
- **Robustness**: invalid schemas, invalid instances, and malformed token
  streams fail with useful context; recovery-oriented decode paths document
  their normalization and consumed-token accounting.
- **Consistency**: implementation, tests, and documentation describe the same
  contract; immutable data flows between passes.
- **Simplicity**: each pass has one purpose and a clean interface.
- **Performance**: generated hot paths are specialized at compile time so LLVM
  can optimize them; runtime schema introspection is avoided.

## Custom Serde and Null Preservation

Generated structs use derived serde implementations when ordinary field mapping
is sufficient. The generator switches to custom serde when it must preserve
schema-specific routing that derives cannot express.

Custom serde is used for pattern validators and additional-property routing. A
generated struct can contain regular fields, one or more pattern-validator
fields, and an `additional_properties` map. Deserialization checks regular fields
first, routes pattern-matching keys to the matching validator, and falls back to
`additionalProperties` when the schema allows it.

`OptionalField<T>` preserves the three JSON states that `Option<T>` cannot
represent by itself:

```rust
pub enum OptionalField<T> {
    Absent,
    Null,
    Present(T),
}
```

Generated code uses `OptionalField<T>` when a field can be absent, explicitly
`null`, or present with a value. The token stream uses a state prefix for
absent/null/present, followed by the encoded value only for the present case.
This keeps explicit JSON nulls distinct from omitted fields during semantic
round trips.

## String Encoding

Strings use end-marker encoding rather than length-prefix encoding. The encoder
emits the configured byte tokens and then an end marker; the decoder reads until
that marker, rejects invalid byte tokens for the configured mode, and validates
UTF-8 for default string encoding.

This design avoids requiring a model to predict the string length before
emitting content, while keeping token count comparable to a length-prefixed
format. Missing end markers are EOF errors. Extra tokens after a decoded value
are a whole-stream validation concern for the caller or generated harness.

## Compression Strategy

TCT supports optional BPE compression on top of plain TCT token sequences.
Legacy dictionary compression has been removed; BPE tables with nonzero
`dict_size` metadata are rejected because their token layout is incompatible
with the current generated tokenizer.

### Byte Pair Encoding (BPE)

**Purpose**: Learn recurring token-pair patterns from a corpus for general
sequence compression.

**Architecture**: Merge table embedded at compile-time.
```rust
// Training phase (offline)
tct-train-bpe \
  --schema schemas/popular/github-workflow.json \
  --encoded target/tct-examples/github-workflow-encoded/all.jsonl \
  --target-vocab 4096 \
  --output target/tct-examples/github-workflow-bpe.json

// Compilation phase
let grammar = TctCompiler::new(schema, "Type")?
    .with_bpe_merges("target/tct-examples/github-workflow-bpe.json")?
    .compile()?;
```

**Compression Benefits**:
- Learns byte-level and structural token patterns.
- Applies uniformly across all encoded fields.
- Keeps generated `VOCAB_SIZE` accounting simple: `base_vocab + merges + PAD`.

The generated encoder first emits base TCT tokens, then applies BPE merges. The
decoder expands BPE tokens back to base tokens before schema-specific decoding.

## Future Extensions

New passes should follow the same pattern as the current pipeline: consume immutable input IR, produce new
immutable output IR, and keep code generation in the generation pass.
