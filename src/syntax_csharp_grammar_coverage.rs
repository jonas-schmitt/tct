//! C# syntax grammar coverage over registered parser-backed codec entries.
//!
//! This gate stays at the syntax boundary: checked C# metadata lowers through
//! GrammarIR, StructuralModel, token layout, and the raw syntax state-machine
//! core. It deliberately does not use C# arena IR, source-program type
//! correctness, route-handoff provenance, or BPE artifacts as validity
//! authority.

mod coverage;
mod report;

use std::error::Error;
use std::fmt;

pub use report::{
    CSharpSyntaxGrammarCoverageBlocker, CSharpSyntaxGrammarCoverageEntry,
    CSharpSyntaxGrammarCoverageRawStateMachine, CSharpSyntaxGrammarCoverageReport,
    CSharpSyntaxGrammarCoverageTokenLayout, CSharpSyntaxGrammarCoverageTotals,
};

pub const CSHARP_SYNTAX_GRAMMAR_COVERAGE_ARTIFACT_KIND: &str =
    "tct_csharp_syntax_grammar_coverage_v1";
pub const CSHARP_SYNTAX_GRAMMAR_COVERAGE_VERSION: u32 = 1;
pub const CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_READY: &str =
    "csharp_syntax_registry_raw_state_coverage_ready";
pub const CSHARP_SYNTAX_GRAMMAR_COVERAGE_STATUS_BLOCKED: &str =
    "csharp_syntax_registry_raw_state_coverage_blocked";
pub const CSHARP_SYNTAX_GRAMMAR_COVERAGE_SCOPE: &str = "registered_csharp_syntax_codec_entries";
pub const CSHARP_SYNTAX_GRAMMAR_COVERAGE_NEXT_READY_TASK: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;
pub const CSHARP_SYNTAX_GRAMMAR_COVERAGE_BLOCKER_NEXT_READY_TASK: &str =
    "fix the first C# syntax grammar coverage blocker before expanding registry breadth";
pub const CSHARP_SYNTAX_GRAMMAR_COVERAGE_CROSS_TARGET_FRAMING: &str = "C# grammar coverage is compared with the Go AST grammar coverage gate at the shared raw state-machine boundary: parser-backed metadata lowers through GrammarIR -> StructuralModel -> token layout, then the compiled raw state exposes empty-prefix admissible node-start state. Go AST covers the pinned full go/ast inventory; this C# gate covers the currently registered C# syntax codec entries and records the first precise syntax-level blocker before any broader C# registry claim.";
pub const CSHARP_SYNTAX_GRAMMAR_COVERAGE_CLAIM_BOUNDARY: &str = "C# syntax grammar coverage gate: checked Roslyn-style metadata for registered syntax codec entries lowers through GrammarIR -> StructuralModel -> token layout and compiles the raw syntax state-machine boundary. This is not C# arena IR, not source-program type correctness, not route-handoff provenance, not BPE validity authority, and not stable-backend promotion.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpSyntaxGrammarCoverageError {
    phase: &'static str,
    path: Option<String>,
    message: String,
}

impl CSharpSyntaxGrammarCoverageError {
    pub(crate) fn new(
        phase: &'static str,
        path: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            phase,
            path,
            message: message.into(),
        }
    }
}

impl fmt::Display for CSharpSyntaxGrammarCoverageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(
                formatter,
                "{} error at {}: {}",
                self.phase, path, self.message
            ),
            None => write!(formatter, "{} error: {}", self.phase, self.message),
        }
    }
}

impl Error for CSharpSyntaxGrammarCoverageError {}
