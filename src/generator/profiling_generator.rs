//! Profiling code generator for TCT tokenizers
//!
//! This module generates feature-gated profiling infrastructure for TCT-generated types.
//! It creates timing wrappers, benchmark helpers, and optional Criterion benchmark files.

use crate::grammar::ProfilingModule;
use crate::profiling_config::{ProfilingConfig, ProfilingLevel};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generates profiling code for TCT tokenizers
pub struct ProfilingGenerator {
    /// Type name being generated for
    type_name: String,
    /// Profiling configuration
    config: ProfilingConfig,
}

impl ProfilingGenerator {
    /// Create new profiling generator
    pub fn new(type_name: String, config: ProfilingConfig) -> Self {
        Self { type_name, config }
    }

    /// Generate complete profiling module
    pub fn generate(&self) -> ProfilingModule {
        let mut module_code = TokenStream::new();

        // Generate feature-gated profiling module
        let feature_gate = &self.config.feature_gate;
        let type_ident = format_ident!("{}", self.type_name);

        match self.config.level {
            ProfilingLevel::None => {
                // No profiling code generated
            }
            ProfilingLevel::Basic => {
                module_code.extend(self.generate_basic_profiling(&type_ident, feature_gate));
            }
            ProfilingLevel::Detailed => {
                module_code.extend(self.generate_detailed_profiling(&type_ident, feature_gate));
            }
        }

        // Generate Criterion benchmark file if requested
        let benchmark_file = if self.config.generate_criterion {
            Some(self.generate_criterion_benchmark())
        } else {
            None
        };

        ProfilingModule {
            module_code,
            benchmark_file,
        }
    }

    /// Generate basic profiling with timing wrappers
    fn generate_basic_profiling(
        &self,
        type_ident: &proc_macro2::Ident,
        feature_gate: &str,
    ) -> TokenStream {
        let allocation_tracking = if self.config.track_allocations {
            quote! {
                #[cfg(feature = "dhat-heap")]
                let _profiler = dhat::Profiler::new_heap();
            }
        } else {
            TokenStream::new()
        };

        quote! {
            #[cfg(feature = #feature_gate)]
            pub mod profiling {
                use std::time::{Duration, Instant};
                use super::*;

                /// Trait for timing tokenizer operations
                pub trait TimedEncode: crate::tokenizer::JsonTokenEncoder {
                    /// Encode with timing measurement
                    fn encode_with_timing(&self) -> (Vec<u32>, Duration) {
                        #allocation_tracking
                        let start = Instant::now();
                        let mut tokens = Vec::new();
                        self.encode(&mut tokens);
                        let duration = start.elapsed();
                        (tokens, duration)
                    }

                    /// Decode with timing measurement
                    fn decode_with_timing(tokens: &mut impl Iterator<Item = u32>)
                        -> Result<(Self, Duration), crate::tokenizer::DecodeError>
                    where
                        Self: Sized,
                    {
                        #allocation_tracking
                        let start = Instant::now();
                        let result = Self::decode(tokens)?;
                        let duration = start.elapsed();
                        Ok((result, duration))
                    }

                    /// Get token count with timing
                    fn token_count_with_timing(&self) -> (usize, Duration) {
                        #allocation_tracking
                        let start = Instant::now();
                        let count = self.token_count();
                        let duration = start.elapsed();
                        (count, duration)
                    }
                }

                /// Implementation of TimedEncode for the generated type
                impl TimedEncode for #type_ident {}

                /// Benchmark helper for round-trip operations
                pub fn benchmark_round_trip(instance: &#type_ident) -> (Duration, Duration, bool) {
                    let (tokens, encode_time) = instance.encode_with_timing();
                    let mut token_iter = tokens.into_iter();

                    match #type_ident::decode_with_timing(&mut token_iter) {
                        Ok((decoded, decode_time)) => {
                            let round_trip_valid = instance == &decoded;
                            (encode_time, decode_time, round_trip_valid)
                        }
                        Err(_) => (encode_time, Duration::ZERO, false),
                    }
                }

                /// Quick performance measurement
                pub fn quick_benchmark(instance: &#type_ident, iterations: usize) -> BenchmarkResults {
                    let mut encode_times = Vec::with_capacity(iterations);
                    let mut decode_times = Vec::with_capacity(iterations);
                    let mut token_counts = Vec::with_capacity(iterations);
                    let mut valid_round_trips = 0;

                    for _ in 0..iterations {
                        let (encode_time, decode_time, valid) = benchmark_round_trip(instance);
                        encode_times.push(encode_time);
                        decode_times.push(decode_time);

                        let (count, _) = instance.token_count_with_timing();
                        token_counts.push(count);

                        if valid {
                            valid_round_trips += 1;
                        }
                    }

                    BenchmarkResults {
                        iterations,
                        encode_times,
                        decode_times,
                        token_counts,
                        valid_round_trips,
                    }
                }

                /// Results from benchmark operations
                #[derive(Debug, Clone)]
                pub struct BenchmarkResults {
                    pub iterations: usize,
                    pub encode_times: Vec<Duration>,
                    pub decode_times: Vec<Duration>,
                    pub token_counts: Vec<usize>,
                    pub valid_round_trips: usize,
                }

                impl BenchmarkResults {
                    /// Get average encode time
                    pub fn avg_encode_time(&self) -> Duration {
                        let total: Duration = self.encode_times.iter().sum();
                        total / self.iterations as u32
                    }

                    /// Get average decode time
                    pub fn avg_decode_time(&self) -> Duration {
                        let total: Duration = self.decode_times.iter().sum();
                        total / self.iterations as u32
                    }

                    /// Get average token count
                    pub fn avg_token_count(&self) -> f64 {
                        self.token_counts.iter().sum::<usize>() as f64 / self.iterations as f64
                    }

                    /// Get round-trip success rate
                    pub fn success_rate(&self) -> f64 {
                        self.valid_round_trips as f64 / self.iterations as f64
                    }

                    /// Print summary
                    pub fn print_summary(&self) {
                        println!("Benchmark Results ({} iterations):", self.iterations);
                        println!("  Avg Encode Time: {:?}", self.avg_encode_time());
                        println!("  Avg Decode Time: {:?}", self.avg_decode_time());
                        println!("  Avg Token Count: {:.1}", self.avg_token_count());
                        println!("  Success Rate: {:.1}%", self.success_rate() * 100.0);
                    }
                }
            }
        }
    }

    /// Generate detailed profiling with per-operation breakdowns
    fn generate_detailed_profiling(
        &self,
        type_ident: &proc_macro2::Ident,
        feature_gate: &str,
    ) -> TokenStream {
        let basic_profiling = self.generate_basic_profiling(type_ident, feature_gate);

        let detailed_additions = quote! {
            #[cfg(feature = #feature_gate)]
            pub mod detailed {
                use super::*;
                use std::time::{Duration, Instant};

                /// Detailed profiling results with operation breakdown
                #[derive(Debug, Clone)]
                pub struct DetailedProfilingResults {
                    pub encode_time: Duration,
                    pub decode_time: Duration,
                    pub token_count: usize,
                    /// Generated timing helpers do not collect allocation counts.
                    /// Use tct-measure-tokens --track-allocations for measured
                    /// generated helper subprocess allocation counters.
                    pub memory_allocations: Option<usize>,
                    pub operation_breakdown: Vec<OperationTiming>,
                }

                /// Individual operation timing
                #[derive(Debug, Clone)]
                pub struct OperationTiming {
                    pub operation: String,
                    pub duration: Duration,
                    pub iterations: usize,
                }

                /// Enhanced profiling for detailed analysis
                pub trait DetailedProfiler: TimedEncode {
                    /// Run detailed profiling analysis
                    fn detailed_profile(&self) -> DetailedProfilingResults {
                        let mut breakdown = Vec::new();

                        // Profile encode operation
                        let (tokens, encode_time) = self.encode_with_timing();
                        breakdown.push(OperationTiming {
                            operation: "encode".to_string(),
                            duration: encode_time,
                            iterations: 1,
                        });

                        // Profile token count
                        let (count, count_time) = self.token_count_with_timing();
                        breakdown.push(OperationTiming {
                            operation: "token_count".to_string(),
                            duration: count_time,
                            iterations: 1,
                        });

                        // Profile decode operation
                        let mut token_iter = tokens.into_iter();
                        let decode_time = match Self::decode_with_timing(&mut token_iter) {
                            Ok((_, time)) => time,
                            Err(_) => Duration::ZERO,
                        };
                        breakdown.push(OperationTiming {
                            operation: "decode".to_string(),
                            duration: decode_time,
                            iterations: 1,
                        });

                        DetailedProfilingResults {
                            encode_time,
                            decode_time,
                            token_count: count,
                            memory_allocations: None,
                            operation_breakdown: breakdown,
                        }
                    }
                }

                impl DetailedProfiler for #type_ident {}
            }
        };

        quote! {
            #basic_profiling
            #detailed_additions
        }
    }

    /// Generate enhanced Criterion benchmark file content
    fn generate_criterion_benchmark(&self) -> String {
        let sample_data_function = self.generate_sample_data_function();

        format!(
            r#"//! Comprehensive Criterion benchmarks for {}
//!
//! Generated by TCT profiling system
//! Tests caller-provided schema-valid sample data and optimization scenarios

use criterion::{{black_box, criterion_group, criterion_main, Criterion, BenchmarkId}};

// Import the generated type and profiling module
use crate::profiling::{{TimedEncode, benchmark_round_trip, quick_benchmark}};

{}

fn benchmark_encode_sizes(c: &mut Criterion) {{
    let mut group = c.benchmark_group("{}_encode_by_size");

    for (size_name, sample_data) in get_sample_data() {{
        let instance = serde_json::from_value::<{}>(sample_data)
            .expect("Criterion sample data should be valid for the generated type");

        group.bench_with_input(BenchmarkId::new("encode", size_name), &instance, |b, instance| {{
            b.iter(|| {{
                let (tokens, _) = black_box(instance).encode_with_timing();
                black_box(tokens);
            }})
        }});
    }}

    group.finish();
}}

fn benchmark_decode_sizes(c: &mut Criterion) {{
    let mut group = c.benchmark_group("{}_decode_by_size");

    for (size_name, sample_data) in get_sample_data() {{
        let instance = serde_json::from_value::<{}>(sample_data)
            .expect("Criterion sample data should be valid for the generated type");
        let (tokens, _) = instance.encode_with_timing();

        group.bench_with_input(BenchmarkId::new("decode", size_name), &tokens, |b, tokens| {{
            b.iter(|| {{
                let mut token_iter = black_box(tokens.clone()).into_iter();
                let result = {}::decode_with_timing(black_box(&mut token_iter));
                black_box(result);
            }})
        }});
    }}

    group.finish();
}}

fn benchmark_round_trip_sizes(c: &mut Criterion) {{
    let mut group = c.benchmark_group("{}_round_trip_by_size");

    for (size_name, sample_data) in get_sample_data() {{
        let instance = serde_json::from_value::<{}>(sample_data)
            .expect("Criterion sample data should be valid for the generated type");

        group.bench_with_input(BenchmarkId::new("round_trip", size_name), &instance, |b, instance| {{
            b.iter(|| {{
                let result = benchmark_round_trip(black_box(instance));
                black_box(result);
            }})
        }});
    }}

    group.finish();
}}

fn benchmark_quick_performance(c: &mut Criterion) {{
    let sample_data = get_sample_data()[0].1.clone();
    let instance = serde_json::from_value::<{}>(sample_data)
        .expect("Criterion sample data should be valid for the generated type");

    c.bench_function("{}_quick_benchmark_100", |b| {{
        b.iter(|| {{
            let results = quick_benchmark(black_box(&instance), 100);
            black_box(results);
        }})
    }});
}}

fn benchmark_token_count(c: &mut Criterion) {{
    let mut group = c.benchmark_group("{}_token_count");

    for (size_name, sample_data) in get_sample_data() {{
        let instance = serde_json::from_value::<{}>(sample_data)
            .expect("Criterion sample data should be valid for the generated type");

        group.bench_with_input(BenchmarkId::new("token_count", size_name), &instance, |b, instance| {{
            b.iter(|| {{
                let (count, _) = black_box(instance).token_count_with_timing();
                black_box(count);
            }})
        }});
    }}

    group.finish();
}}

criterion_group!(
    benches,
    benchmark_encode_sizes,
    benchmark_decode_sizes,
    benchmark_round_trip_sizes,
    benchmark_quick_performance,
    benchmark_token_count
);
criterion_main!(benches);
"#,
            self.type_name,
            sample_data_function,
            self.type_name,
            self.type_name,
            self.type_name,
            self.type_name,
            self.type_name,
            self.type_name,
            self.type_name,
            self.type_name,
            self.type_name,
            self.type_name,
            self.type_name
        )
    }

    /// Generate caller-provided sample data for Criterion benchmarks
    fn generate_sample_data_function(&self) -> String {
        if self.config.criterion_samples.is_empty() {
            return r#"/// Schema-valid sample data for benchmarking.
///
/// Add samples with ProfilingConfig::with_criterion_sample before running this
/// generated Criterion benchmark. Placeholder data is deliberately not emitted
/// because invalid samples would make benchmark results misleading.
fn get_sample_data() -> Vec<(&'static str, serde_json::Value)> {
    panic!(
        "generated Criterion benchmarks require caller-provided schema-valid samples; \
         configure ProfilingConfig::with_criterion_sample"
    )
}"#
            .to_string();
        }

        let entries = self
            .config
            .criterion_samples
            .iter()
            .map(|sample| {
                let name = format!("{:?}", sample.name.as_str());
                let json_text = serde_json::to_string(&sample.value)
                    .expect("serde_json::Value samples should serialize");
                let json_literal = format!("{:?}", json_text);
                format!(
                    "        ({name}, serde_json::from_str::<serde_json::Value>({json_literal}).expect(\"valid embedded Criterion sample JSON\"))"
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            r#"/// Schema-valid sample data for benchmarking.
fn get_sample_data() -> Vec<(&'static str, serde_json::Value)> {{
    vec![
{entries}
    ]
}}"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiling_config::ProfilingConfig;
    use quote::ToTokens;

    fn parse_generated_file(code: &str) -> syn::File {
        syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
    }

    fn collect_token_texts(stream: proc_macro2::TokenStream, output: &mut Vec<String>) {
        for token in stream {
            match token {
                proc_macro2::TokenTree::Group(group) => collect_token_texts(group.stream(), output),
                proc_macro2::TokenTree::Ident(ident) => output.push(ident.to_string()),
                proc_macro2::TokenTree::Punct(punct) => output.push(punct.as_char().to_string()),
                proc_macro2::TokenTree::Literal(literal) => output.push(literal.to_string()),
            }
        }
    }

    fn generated_token_texts(file: &syn::File) -> Vec<String> {
        let mut output = Vec::new();
        collect_token_texts(file.to_token_stream(), &mut output);
        output
    }

    fn has_string_literal(tokens: &[String], expected: &str) -> bool {
        tokens.iter().any(|token| {
            syn::parse_str::<syn::LitStr>(token).is_ok_and(|literal| literal.value() == expected)
        })
    }

    fn contains_ordered_tokens(tokens: &[String], expected: &[&str]) -> bool {
        let mut expected = expected.iter();
        let Some(mut next) = expected.next() else {
            return true;
        };
        for token in tokens {
            if token == next {
                let Some(after_match) = expected.next() else {
                    return true;
                };
                next = after_match;
            }
        }
        false
    }

    #[test]
    fn test_basic_profiling_generation() {
        let generator = ProfilingGenerator::new("TestType".to_string(), ProfilingConfig::basic());

        let module = generator.generate();
        assert!(!module.module_code.is_empty());
        assert!(module.benchmark_file.is_none());
    }

    #[test]
    fn test_detailed_profiling_generation() {
        let generator =
            ProfilingGenerator::new("TestType".to_string(), ProfilingConfig::detailed());

        let module = generator.generate();
        assert!(!module.module_code.is_empty());
        assert!(module.benchmark_file.is_none());
    }

    #[test]
    fn test_criterion_benchmark_generation() {
        let generator =
            ProfilingGenerator::new("TestType".to_string(), ProfilingConfig::benchmarking());

        let module = generator.generate();
        assert!(!module.module_code.is_empty());
        assert!(module.benchmark_file.is_some());

        let benchmark = module.benchmark_file.unwrap();
        assert!(benchmark.contains("criterion_group!"));
        assert!(benchmark.contains("TestType"));
        assert!(benchmark.contains("with_criterion_sample"));
        assert!(!benchmark.contains("json!(\"example\")"));
    }

    #[test]
    fn test_criterion_benchmark_uses_caller_samples() {
        let config = ProfilingConfig::benchmarking()
            .with_criterion_sample("green", serde_json::json!("green"))
            .with_criterion_sample("object", serde_json::json!({"status": "ready"}));
        let generator = ProfilingGenerator::new("TestType".to_string(), config);

        let module = generator.generate();
        let benchmark = module.benchmark_file.unwrap();
        let generated_file = parse_generated_file(&benchmark);
        let tokens = generated_token_texts(&generated_file);
        assert!(has_string_literal(&tokens, "green"));
        assert!(has_string_literal(&tokens, "\"green\""));
        assert!(has_string_literal(&tokens, "object"));
        assert!(has_string_literal(&tokens, "{\"status\":\"ready\"}"));
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.as_str() == "from_str")
                .count(),
            2
        );
        assert!(contains_ordered_tokens(
            &tokens,
            &["serde_json", "from_value", "TestType"]
        ));
        assert!(benchmark.contains("encode_with_timing"));
        assert!(!benchmark.contains("try_from(sample_data)"));
    }

    #[test]
    fn test_no_profiling_generation() {
        let generator = ProfilingGenerator::new(
            "TestType".to_string(),
            ProfilingConfig::default(), // ProfilingLevel::None
        );

        let module = generator.generate();
        assert!(module.module_code.is_empty());
        assert!(module.benchmark_file.is_none());
    }
}
