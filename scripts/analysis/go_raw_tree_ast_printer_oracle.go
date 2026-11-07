// go_raw_tree_ast_printer_oracle evaluates whether a decoded Go raw tree can
// be materialized as go/ast and rendered through go/format instead of a manual
// concrete-source renderer. It is sidecar evidence only.
package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"go/ast"
	"go/format"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strconv"
	"strings"
)

const (
	goRawTreeAstPrinterOracleTool          = "go_raw_tree_ast_printer_oracle"
	goRawTreeAstPrinterOracleVersion       = 1
	goRawTreeAstPrinterOracleArtifactKind  = "tct_go_raw_tree_ast_printer_oracle_decision_v1"
	goRawTreeAstPrinterOracleInputKind     = "tct_go_raw_tree_ast_printer_oracle_input_v1"
	goRawTreeAstPrinterOracleStructureMode = "structure-only"
	goRawTreeAstPrinterOracleGrammarMode   = "grammar-mode"

	goRawTreeAstPrinterOracleStatusPassed               = "passed"
	goRawTreeAstPrinterOracleStatusStructuralBlocked    = "structural_materialization_blocked"
	goRawTreeAstPrinterOracleStatusCommentPolicyBlocked = "structural_passed_comment_policy_blocked"
	goRawTreeAstPrinterOracleSuccessorTask              = "Build Go Source-Record Raw-Token Generator Core"
)

var (
	tokenType    = reflect.TypeOf(token.ILLEGAL)
	tokenPosType = reflect.TypeOf(token.NoPos)
	chanDirType  = reflect.TypeOf(ast.SEND)

	identPtrType        = reflect.TypeOf((*ast.Ident)(nil))
	basicLitPtrType     = reflect.TypeOf((*ast.BasicLit)(nil))
	commentPtrType      = reflect.TypeOf((*ast.Comment)(nil))
	commentGroupPtrType = reflect.TypeOf((*ast.CommentGroup)(nil))
)

type goRawTreeAstPrinterOracleOptions struct {
	decodedTreePath  string
	astInventoryPath string
	sourceLabel      string
	mode             string
	outputPath       string
	validatePath     string
}

type goRawTreeAstPrinterOracleInput struct {
	ArtifactKind string                                 `json:"artifact_kind,omitempty"`
	Version      int                                    `json:"version,omitempty"`
	Tool         string                                 `json:"tool,omitempty"`
	Mode         string                                 `json:"mode,omitempty"`
	Records      []goRawTreeAstPrinterOracleInputRecord `json:"records,omitempty"`
	Root         *rawDecodedNode                        `json:"root,omitempty"`
}

type goRawTreeAstPrinterOracleInputRecord struct {
	SourceLabel                    string                         `json:"source_label"`
	Category                       string                         `json:"category,omitempty"`
	ManifestPath                   string                         `json:"manifest_path,omitempty"`
	SourcePath                     string                         `json:"source_path,omitempty"`
	SourceSHA256                   string                         `json:"source_sha256,omitempty"`
	CurrentRustRendererGrammarMode currentRustRendererGrammarMode `json:"current_rust_renderer_grammar_mode"`
	DecodedTree                    rawDecodedTreeEnvelope         `json:"decoded_tree"`
}

type currentRustRendererGrammarMode struct {
	Status                    string  `json:"status"`
	RawTreeGrammarModeReady   bool    `json:"raw_tree_grammar_mode_render_ready"`
	RawTreeCanonicalReady     bool    `json:"raw_tree_canonical_fidelity_ready"`
	GofmtValidation           string  `json:"gofmt_validation,omitempty"`
	GoParserReparseValidation string  `json:"go_parser_reparse_validation,omitempty"`
	RendererDiagnostic        *string `json:"renderer_diagnostic,omitempty"`
}

type rawDecodedTreeEnvelope struct {
	RootNodeKind          string         `json:"root_node_kind,omitempty"`
	NodeStartCount        int            `json:"node_start_count,omitempty"`
	FieldStartCount       int            `json:"field_start_count,omitempty"`
	PayloadEventCount     int            `json:"payload_event_count,omitempty"`
	PayloadTokenByteCount int            `json:"payload_token_byte_count,omitempty"`
	PayloadValueCount     int            `json:"payload_value_count,omitempty"`
	Root                  rawDecodedNode `json:"root"`
}

type rawDecodedNode struct {
	RuleName string            `json:"rule_name"`
	NodeKind string            `json:"node_kind"`
	Fields   []rawDecodedField `json:"fields,omitempty"`
}

type rawDecodedField struct {
	Name          string            `json:"name"`
	Values        []rawDecodedValue `json:"values,omitempty"`
	Missing       bool              `json:"missing,omitempty"`
	RepeatedEmpty bool              `json:"repeated_empty,omitempty"`
	Required      bool              `json:"required,omitempty"`
	Multiple      bool              `json:"multiple,omitempty"`
	Terminal      string            `json:"terminal,omitempty"`
}

type rawDecodedValue struct {
	Kind          string              `json:"kind,omitempty"`
	NodeKind      string              `json:"node_kind,omitempty"`
	Node          *rawDecodedNode     `json:"node,omitempty"`
	Terminal      string              `json:"terminal,omitempty"`
	Payload       string              `json:"payload,omitempty"`
	SerdeNode     *rawDecodedNode     `json:"Node,omitempty"`
	SerdeTerminal *rawDecodedTerminal `json:"Terminal,omitempty"`
}

type rawDecodedTerminal struct {
	NodeKind string `json:"node_kind"`
	Terminal string `json:"terminal"`
	Payload  string `json:"payload"`
}

type goRawTreeAstPrinterOracleReport struct {
	ArtifactKind                 string                                  `json:"artifact_kind"`
	Version                      int                                     `json:"version"`
	Tool                         string                                  `json:"tool"`
	Status                       string                                  `json:"status"`
	Mode                         string                                  `json:"mode"`
	GoVersion                    string                                  `json:"go_version"`
	Input                        oracleFileRef                           `json:"input"`
	AstInventory                 oracleInventorySummary                  `json:"ast_inventory"`
	RecordCount                  int                                     `json:"record_count"`
	StructuralPassedCount        int                                     `json:"structural_passed_count"`
	StructuralBlockedCount       int                                     `json:"structural_blocked_count"`
	CommentDirectiveBlockedCount int                                     `json:"comment_directive_blocked_count"`
	FullAdoptionPossible         bool                                    `json:"full_adoption_possible"`
	StructuralBlockers           []oracleAdoptionFinding                 `json:"structural_blockers"`
	EssentialCommentBlockers     []oracleAdoptionFinding                 `json:"essential_comment_blockers"`
	ParserBlockers               []oracleAdoptionFinding                 `json:"parser_blockers"`
	CanonicalFidelityOnlyGaps    []oracleAdoptionFinding                 `json:"canonical_fidelity_only_gaps"`
	NextConcreteTask             string                                  `json:"next_concrete_task"`
	DiagnosticsByKind            []oracleDiagnosticCount                 `json:"diagnostics_by_kind"`
	Records                      []goRawTreeAstPrinterOracleRecordReport `json:"records"`
	Decision                     oracleDecision                          `json:"decision"`
	ValidationCommands           []string                                `json:"validation_commands"`
	ClaimBoundary                string                                  `json:"claim_boundary"`
	NoStableClaims               []string                                `json:"no_stable_claims"`
}

type oracleFileRef struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int64  `json:"bytes"`
}

type oracleInventorySummary struct {
	Path         string `json:"path"`
	SHA256       string `json:"sha256"`
	Bytes        int64  `json:"bytes"`
	ArtifactKind string `json:"artifact_kind,omitempty"`
	Version      int    `json:"version,omitempty"`
	GoVersion    string `json:"go_version,omitempty"`
}

type goRawTreeAstPrinterOracleRecordReport struct {
	SourceLabel                    string                         `json:"source_label"`
	Category                       string                         `json:"category,omitempty"`
	ManifestPath                   string                         `json:"manifest_path,omitempty"`
	SourcePath                     string                         `json:"source_path,omitempty"`
	SourceSHA256                   string                         `json:"source_sha256,omitempty"`
	CurrentRustRendererGrammarMode currentRustRendererGrammarMode `json:"current_rust_renderer_grammar_mode"`
	OracleStatus                   string                         `json:"oracle_status"`
	GoFormatValidation             string                         `json:"go_format_node_validation"`
	GoParserReparseValidation      string                         `json:"go_parser_reparse_validation"`
	FormattedSourceSHA256          string                         `json:"formatted_source_sha256,omitempty"`
	FormattedSourceByteCount       int                            `json:"formatted_source_byte_count,omitempty"`
	CommentDirectiveReadiness      string                         `json:"comment_directive_readiness"`
	CommentCount                   int                            `json:"comment_count"`
	EssentialCommentCount          int                            `json:"essential_comment_count"`
	EssentialCommentCategories     []string                       `json:"essential_comment_categories,omitempty"`
	Materialization                oracleMaterializationSummary   `json:"materialization"`
	Diagnostics                    []oracleDiagnostic             `json:"diagnostics,omitempty"`
}

type oracleMaterializationSummary struct {
	NodeCount                     int      `json:"node_count"`
	FieldCount                    int      `json:"field_count"`
	TerminalPayloadCount          int      `json:"terminal_payload_count"`
	DistinctNodeKinds             []string `json:"distinct_node_kinds"`
	DistinctFields                []string `json:"distinct_fields"`
	ReflectedFieldAssignments     int      `json:"reflected_field_assignments"`
	TerminalCoercions             int      `json:"terminal_coercions"`
	SynthesizedPositionCount      int      `json:"synthesized_position_count"`
	InferredBasicLitKindCount     int      `json:"inferred_basic_lit_kind_count"`
	SkippedCommentNodeCount       int      `json:"skipped_comment_node_count"`
	UnsupportedStructuralCount    int      `json:"unsupported_structural_count"`
	UnsupportedCommentPolicyCount int      `json:"unsupported_comment_policy_count"`
}

type oracleDiagnostic struct {
	DiagnosticKind string `json:"diagnostic_kind"`
	Category       string `json:"category"`
	NodeKind       string `json:"node_kind,omitempty"`
	Field          string `json:"field,omitempty"`
	Terminal       string `json:"terminal,omitempty"`
	Payload        string `json:"payload,omitempty"`
	Message        string `json:"message"`
}

type oracleDiagnosticCount struct {
	DiagnosticKind string `json:"diagnostic_kind"`
	Count          int    `json:"count"`
}

type oracleAdoptionFinding struct {
	SourceLabel    string `json:"source_label"`
	Category       string `json:"category,omitempty"`
	ManifestPath   string `json:"manifest_path,omitempty"`
	SourcePath     string `json:"source_path,omitempty"`
	DiagnosticKind string `json:"diagnostic_kind"`
	NodeKind       string `json:"node_kind,omitempty"`
	Field          string `json:"field,omitempty"`
	Terminal       string `json:"terminal,omitempty"`
	Payload        string `json:"payload,omitempty"`
	Message        string `json:"message"`
}

type oracleDecision struct {
	Recommendation                string   `json:"recommendation"`
	StructuralOraclePassed        bool     `json:"structural_oracle_passed"`
	CommentsOrDirectivesBlocked   bool     `json:"comments_or_directives_blocked"`
	MateriallyReducesManualPolicy bool     `json:"materially_reduces_manual_renderer_policy"`
	ReusableDiagnosticCategories  []string `json:"reusable_diagnostic_categories"`
	NextConcreteTask              string   `json:"next_concrete_task"`
	Rationale                     string   `json:"rationale"`
}

type astMaterializer struct {
	mode                  string
	diagnostics           []oracleDiagnostic
	nodeCount             int
	fieldCount            int
	terminalCount         int
	nodeKinds             map[string]struct{}
	fields                map[string]struct{}
	assignments           int
	terminalCoercions     int
	synthesizedPositions  int
	inferredBasicLitKinds int
	skippedCommentNodes   int
	nextSyntheticPos      token.Pos
}

func main() {
	if err := runGoRawTreeAstPrinterOracle(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func runGoRawTreeAstPrinterOracle(args []string) error {
	var options goRawTreeAstPrinterOracleOptions
	flags := flag.NewFlagSet(goRawTreeAstPrinterOracleTool, flag.ContinueOnError)
	flags.StringVar(&options.decodedTreePath, "decoded-tree", "", "decoded raw-tree JSON input path")
	flags.StringVar(&options.astInventoryPath, "ast-inventory", "", "Go AST schema inventory JSON path")
	flags.StringVar(&options.sourceLabel, "source-label", "", "optional source label for a single-root input")
	flags.StringVar(&options.mode, "mode", goRawTreeAstPrinterOracleStructureMode, "oracle mode")
	flags.StringVar(&options.outputPath, "output", "", "output report path")
	flags.StringVar(&options.validatePath, "validate-report", "", "validate an existing oracle report")
	if err := flags.Parse(args); err != nil {
		return err
	}

	if options.validatePath != "" {
		report, err := readOracleReport(options.validatePath)
		if err != nil {
			return err
		}
		return validateOracleReport(report)
	}
	if !isSupportedOracleMode(options.mode) {
		return fmt.Errorf("unsupported oracle mode %q; supported modes: %q, %q", options.mode, goRawTreeAstPrinterOracleStructureMode, goRawTreeAstPrinterOracleGrammarMode)
	}
	if options.decodedTreePath == "" {
		return errors.New("--decoded-tree is required")
	}
	if options.astInventoryPath == "" {
		return errors.New("--ast-inventory is required")
	}
	if options.outputPath == "" {
		return errors.New("--output is required")
	}

	report, err := buildGoRawTreeAstPrinterOracleReport(options)
	if err != nil {
		return err
	}
	if err := validateOracleReport(report); err != nil {
		return err
	}
	return writeJSON(options.outputPath, report)
}

func buildGoRawTreeAstPrinterOracleReport(options goRawTreeAstPrinterOracleOptions) (*goRawTreeAstPrinterOracleReport, error) {
	inputBytes, err := os.ReadFile(options.decodedTreePath)
	if err != nil {
		return nil, fmt.Errorf("read decoded tree input: %w", err)
	}
	var input goRawTreeAstPrinterOracleInput
	if err := json.Unmarshal(inputBytes, &input); err != nil {
		return nil, fmt.Errorf("parse decoded tree input: %w", err)
	}
	if len(input.Records) == 0 && input.Root != nil {
		label := options.sourceLabel
		if label == "" {
			label = filepath.Base(options.decodedTreePath)
		}
		input.Records = []goRawTreeAstPrinterOracleInputRecord{{
			SourceLabel: label,
			DecodedTree: rawDecodedTreeEnvelope{
				RootNodeKind: input.Root.NodeKind,
				Root:         *input.Root,
			},
		}}
	}
	if len(input.Records) == 0 {
		return nil, errors.New("decoded tree input must contain records or a root node")
	}

	inputRef, err := fileRef(options.decodedTreePath)
	if err != nil {
		return nil, err
	}
	inventory, err := inventorySummary(options.astInventoryPath)
	if err != nil {
		return nil, err
	}

	report := &goRawTreeAstPrinterOracleReport{
		ArtifactKind:              goRawTreeAstPrinterOracleArtifactKind,
		Version:                   goRawTreeAstPrinterOracleVersion,
		Tool:                      goRawTreeAstPrinterOracleTool,
		Mode:                      options.mode,
		GoVersion:                 runtime.Version(),
		Input:                     inputRef,
		AstInventory:              inventory,
		StructuralBlockers:        []oracleAdoptionFinding{},
		EssentialCommentBlockers:  []oracleAdoptionFinding{},
		ParserBlockers:            []oracleAdoptionFinding{},
		CanonicalFidelityOnlyGaps: []oracleAdoptionFinding{},
		DiagnosticsByKind:         []oracleDiagnosticCount{},
		Records:                   []goRawTreeAstPrinterOracleRecordReport{},
		ValidationCommands:        []string{"go test scripts/analysis/go_raw_tree_ast_printer_oracle.go scripts/analysis/go_raw_tree_ast_printer_oracle_test.go -run TestGoRawTreeAstPrinterOracle", "cargo test --test syntax_go_source_instance_projection_test go_raw_tree_ast_printer_oracle_exports_current_plus_decision_report -- --exact --nocapture", "cargo test --test syntax_go_source_instance_projection_test go_raw_tree_ast_printer_oracle_exports_current_plus_full_adoption_decision_report -- --exact --nocapture"},
		ClaimBoundary:             "Go raw-tree AST printer oracle sidecar evidence only; go/format and go/parser validate rendering from decoded raw trees in the selected oracle mode. A clean grammar-mode report is the cutover authority for grammar-mode formatting/parsing policy; canonical source fidelity remains diagnostic. The handwritten Rust raw-tree renderer is historical/baseline diagnostic evidence after cutover and is not grammar-mode formatting/parsing authority. This is not generated-hot-path code, not a production Rust API, not source-program validity authority, not canonical comment fidelity, not BPE authority, not Arena IR, not paper-facing evidence, and not stable-backend promotion.",
		NoStableClaims: []string{
			"no_arena_ir_claim",
			"no_bpe_validity_authority",
			"no_canonical_comment_fidelity_claim",
			"no_generated_hot_path_claim",
			"no_paper_facing_claim",
			"no_source_program_validity_claim",
			"no_stable_backend_claim",
			"no_type_correctness_claim",
		},
	}

	diagnosticCounts := map[string]int{}
	for _, record := range input.Records {
		recordReport := evaluateOracleRecord(record, options.mode)
		report.Records = append(report.Records, recordReport)
		if recordReport.OracleStatus == "passed" {
			report.StructuralPassedCount++
		} else {
			report.StructuralBlockedCount++
		}
		if commentDirectiveBlocked(recordReport.CommentDirectiveReadiness) {
			report.CommentDirectiveBlockedCount++
		}
		for _, diagnostic := range recordReport.Diagnostics {
			diagnosticCounts[diagnostic.DiagnosticKind]++
		}
	}
	report.RecordCount = len(report.Records)
	for kind, count := range diagnosticCounts {
		report.DiagnosticsByKind = append(report.DiagnosticsByKind, oracleDiagnosticCount{
			DiagnosticKind: kind,
			Count:          count,
		})
	}
	sort.Slice(report.DiagnosticsByKind, func(i, j int) bool {
		return report.DiagnosticsByKind[i].DiagnosticKind < report.DiagnosticsByKind[j].DiagnosticKind
	})
	report.Status = oracleStatus(report)
	populateAdoptionDecisionFields(report)
	report.Decision = oracleDecisionFor(report)
	return report, nil
}

func evaluateOracleRecord(record goRawTreeAstPrinterOracleInputRecord, mode string) goRawTreeAstPrinterOracleRecordReport {
	materializer := newAstMaterializer(mode)
	commentTexts := collectRawTreeComments(&record.DecodedTree.Root)
	essentialComments, categories := essentialGoComments(commentTexts)

	report := goRawTreeAstPrinterOracleRecordReport{
		SourceLabel:                    record.SourceLabel,
		Category:                       record.Category,
		ManifestPath:                   record.ManifestPath,
		SourcePath:                     record.SourcePath,
		SourceSHA256:                   record.SourceSHA256,
		CurrentRustRendererGrammarMode: record.CurrentRustRendererGrammarMode,
		CommentCount:                   len(commentTexts),
		EssentialCommentCount:          len(essentialComments),
		EssentialCommentCategories:     categories,
	}
	if report.SourceLabel == "" {
		report.SourceLabel = record.SourcePath
	}
	if report.SourceLabel == "" {
		report.SourceLabel = record.ManifestPath
	}

	file, ok := materializer.materializeFile(&record.DecodedTree.Root)
	report.Diagnostics = append(report.Diagnostics, materializer.diagnostics...)
	if mode == goRawTreeAstPrinterOracleStructureMode && len(essentialComments) > 0 {
		for _, category := range categories {
			report.Diagnostics = append(report.Diagnostics, oracleDiagnostic{
				DiagnosticKind: "comment_directive_unsupported",
				Category:       "comment_directive_policy",
				Message:        "structure-only oracle drops essential Go comments/directives before go/format rendering",
				Payload:        category,
			})
		}
	}
	if !ok {
		report.OracleStatus = "blocked"
		report.GoFormatValidation = "not_run"
		report.GoParserReparseValidation = "not_run"
		report.CommentDirectiveReadiness = commentDirectiveReadiness(mode, len(commentTexts), len(essentialComments), nil)
		report.Materialization = materializer.summary()
		return report
	}
	if mode == goRawTreeAstPrinterOracleGrammarMode {
		prepareGrammarModeAstFile(file)
	}

	formatted, err := safeFormatGoAstFile(file)
	if err != nil {
		report.Diagnostics = append(report.Diagnostics, oracleDiagnostic{
			DiagnosticKind: "formatter_error",
			Category:       "formatter",
			NodeKind:       record.DecodedTree.Root.NodeKind,
			Message:        err.Error(),
		})
		report.OracleStatus = "blocked"
		report.GoFormatValidation = "failed"
		report.GoParserReparseValidation = "not_run"
		report.CommentDirectiveReadiness = commentDirectiveReadiness(mode, len(commentTexts), len(essentialComments), nil)
		report.Materialization = materializer.summary()
		return report
	}
	report.GoFormatValidation = "passed"
	report.FormattedSourceByteCount = len(formatted)
	report.FormattedSourceSHA256 = sha256Bytes([]byte(formatted))
	parsed, err := parser.ParseFile(token.NewFileSet(), report.SourceLabel, []byte(formatted), parser.ParseComments)
	if err != nil {
		report.Diagnostics = append(report.Diagnostics, oracleDiagnostic{
			DiagnosticKind: "parser_error",
			Category:       "parser",
			NodeKind:       record.DecodedTree.Root.NodeKind,
			Message:        err.Error(),
		})
		report.OracleStatus = "blocked"
		report.GoParserReparseValidation = "failed"
	} else {
		report.GoParserReparseValidation = "passed"
		report.OracleStatus = "passed"
	}
	var missingEssential []string
	if mode == goRawTreeAstPrinterOracleGrammarMode && parsed != nil {
		missingEssential = missingEssentialComments(essentialComments, collectAstFileCommentTexts(parsed))
		for _, missing := range missingEssential {
			report.Diagnostics = append(report.Diagnostics, oracleDiagnostic{
				DiagnosticKind: "essential_comment_not_preserved",
				Category:       "comment_directive_policy",
				Payload:        missing,
				Message:        "grammar-mode oracle formatted output is missing an essential Go comment/directive from the decoded raw tree",
			})
		}
	}
	report.CommentDirectiveReadiness = commentDirectiveReadiness(mode, len(commentTexts), len(essentialComments), missingEssential)
	report.Materialization = materializer.summary()
	if report.Materialization.UnsupportedStructuralCount > 0 {
		report.OracleStatus = "blocked"
	}
	return report
}

func newAstMaterializer(mode string) *astMaterializer {
	return &astMaterializer{
		nodeKinds:        map[string]struct{}{},
		fields:           map[string]struct{}{},
		nextSyntheticPos: 1,
		mode:             mode,
	}
}

func (m *astMaterializer) materializeFile(root *rawDecodedNode) (*ast.File, bool) {
	value, ok := m.materializeNode(root)
	if !ok || !value.IsValid() {
		return nil, false
	}
	file, ok := value.Interface().(*ast.File)
	if !ok {
		m.addDiagnostic("unsupported_root_node", "structural", root.NodeKind, "", "", "", "root decoded node must materialize to *ast.File")
		return nil, false
	}
	if m.mode == goRawTreeAstPrinterOracleStructureMode {
		file.Comments = nil
	} else {
		file.Comments = grammarModeEssentialFileCommentGroups(file)
	}
	return file, !m.hasStructuralDiagnostics()
}

func (m *astMaterializer) materializeNode(node *rawDecodedNode) (reflect.Value, bool) {
	if node == nil {
		return reflect.Value{}, false
	}
	if m.mode == goRawTreeAstPrinterOracleStructureMode && (node.NodeKind == "Comment" || node.NodeKind == "CommentGroup") {
		m.skippedCommentNodes++
		return reflect.Value{}, true
	}
	m.nodeCount++
	m.nodeKinds[node.NodeKind] = struct{}{}
	typ, ok := astNodeTypeByName(node.NodeKind)
	if !ok {
		m.addDiagnostic("unsupported_node_kind", "structural", node.NodeKind, "", "", "", "decoded node kind has no go/ast struct mapping")
		return reflect.Value{}, false
	}
	ptr := reflect.New(typ)
	elem := ptr.Elem()
	for _, field := range node.Fields {
		m.fieldCount++
		fieldKey := node.NodeKind + "." + field.Name
		m.fields[fieldKey] = struct{}{}
		if len(field.Values) == 0 {
			continue
		}
		structField, ok := elem.Type().FieldByName(field.Name)
		if !ok {
			if isCommentPolicyField(field.Name) {
				m.addDiagnostic("comment_policy_field_unsupported", "comment_directive_policy", node.NodeKind, field.Name, "", "", "structure-only oracle drops raw-tree comment/layout policy fields")
			} else {
				m.addDiagnostic("unsupported_field", "structural", node.NodeKind, field.Name, "", "", "decoded field has no go/ast struct field")
			}
			continue
		}
		target := elem.FieldByIndex(structField.Index)
		if !target.CanSet() {
			m.addDiagnostic("unsupported_field", "structural", node.NodeKind, field.Name, "", "", "go/ast struct field cannot be set by reflection")
			continue
		}
		m.assignField(target, field, node.NodeKind)
	}
	m.finalizeNode(node.NodeKind, ptr)
	return ptr, true
}

func (m *astMaterializer) assignField(target reflect.Value, field rawDecodedField, ownerNodeKind string) {
	if target.Kind() == reflect.Slice {
		for _, raw := range field.Values {
			value, skip := m.valueForType(target.Type().Elem(), raw, ownerNodeKind, field.Name)
			if skip {
				continue
			}
			if !value.IsValid() {
				continue
			}
			coerced, ok := m.coerceValue(target.Type().Elem(), value, ownerNodeKind, field.Name)
			if !ok {
				continue
			}
			target.Set(reflect.Append(target, coerced))
			m.assignments++
		}
		return
	}
	if len(field.Values) > 1 {
		m.addDiagnostic("unsupported_field_cardinality", "structural", ownerNodeKind, field.Name, "", "", "non-slice go/ast field received multiple decoded values")
		return
	}
	value, skip := m.valueForType(target.Type(), field.Values[0], ownerNodeKind, field.Name)
	if skip || !value.IsValid() {
		return
	}
	coerced, ok := m.coerceValue(target.Type(), value, ownerNodeKind, field.Name)
	if !ok {
		return
	}
	target.Set(coerced)
	m.assignments++
}

func (m *astMaterializer) valueForType(targetType reflect.Type, raw rawDecodedValue, ownerNodeKind string, fieldName string) (reflect.Value, bool) {
	if node := raw.nodeValue(); node != nil {
		value, ok := m.materializeNode(node)
		if !ok {
			return reflect.Value{}, false
		}
		if !value.IsValid() {
			return reflect.Value{}, true
		}
		return value, false
	}
	if terminal := raw.terminalValue(); terminal != nil {
		m.terminalCount++
		value, ok := m.valueFromTerminal(targetType, *terminal, ownerNodeKind, fieldName)
		if !ok {
			return reflect.Value{}, false
		}
		m.terminalCoercions++
		return value, false
	}
	m.addDiagnostic("unsupported_decoded_value", "structural", ownerNodeKind, fieldName, "", "", "decoded value contained neither node nor terminal payload")
	return reflect.Value{}, false
}

func (m *astMaterializer) valueFromTerminal(targetType reflect.Type, terminal rawDecodedTerminal, ownerNodeKind string, fieldName string) (reflect.Value, bool) {
	payload := terminal.Payload
	if targetType == tokenType {
		tok, ok := tokenFromPayload(payload)
		if !ok {
			m.addDiagnostic("invalid_finite_domain_token", "structural", ownerNodeKind, fieldName, terminal.Terminal, payload, "terminal payload cannot be mapped to go/token.Token")
			return reflect.Value{}, false
		}
		return reflect.ValueOf(tok), true
	}
	if targetType == tokenPosType {
		if isFalsePayload(payload) {
			return reflect.ValueOf(token.NoPos), true
		}
		m.synthesizedPositions++
		return reflect.ValueOf(m.syntheticPos()), true
	}
	if targetType == chanDirType {
		dir, ok := chanDirFromPayload(payload)
		if !ok {
			m.addDiagnostic("invalid_finite_domain_token", "structural", ownerNodeKind, fieldName, terminal.Terminal, payload, "terminal payload cannot be mapped to ast.ChanDir")
			return reflect.Value{}, false
		}
		return reflect.ValueOf(dir), true
	}
	if targetType.Kind() == reflect.String {
		return reflect.ValueOf(payload).Convert(targetType), true
	}
	if targetType.Kind() == reflect.Bool {
		parsed, err := strconv.ParseBool(payload)
		if err != nil {
			m.addDiagnostic("unsupported_terminal_payload", "structural", ownerNodeKind, fieldName, terminal.Terminal, payload, "terminal payload cannot be parsed as bool")
			return reflect.Value{}, false
		}
		return reflect.ValueOf(parsed).Convert(targetType), true
	}
	if targetType.Kind() >= reflect.Int && targetType.Kind() <= reflect.Int64 {
		parsed, err := strconv.ParseInt(payload, 10, targetType.Bits())
		if err != nil {
			m.addDiagnostic("unsupported_terminal_payload", "structural", ownerNodeKind, fieldName, terminal.Terminal, payload, "terminal payload cannot be parsed as integer")
			return reflect.Value{}, false
		}
		value := reflect.New(targetType).Elem()
		value.SetInt(parsed)
		return value, true
	}
	if targetType == identPtrType {
		return reflect.ValueOf(&ast.Ident{Name: payload}), true
	}
	if targetType == basicLitPtrType {
		return reflect.ValueOf(&ast.BasicLit{Kind: inferBasicLitKind(payload), Value: payload}), true
	}
	if targetType.Kind() == reflect.Interface {
		if ident := terminalIdent(payload, terminal); ident != nil && reflect.TypeOf(ident).Implements(targetType) {
			return reflect.ValueOf(ident), true
		}
		lit := &ast.BasicLit{Kind: inferBasicLitKind(payload), Value: payload}
		if reflect.TypeOf(lit).Implements(targetType) {
			return reflect.ValueOf(lit), true
		}
	}
	m.addDiagnostic("unsupported_terminal_payload", "structural", ownerNodeKind, fieldName, terminal.Terminal, payload, fmt.Sprintf("terminal payload cannot populate %s", targetType))
	return reflect.Value{}, false
}

func (m *astMaterializer) coerceValue(targetType reflect.Type, value reflect.Value, ownerNodeKind string, fieldName string) (reflect.Value, bool) {
	if !value.IsValid() {
		return reflect.Value{}, false
	}
	if value.Type().AssignableTo(targetType) {
		return value, true
	}
	if value.Type().ConvertibleTo(targetType) {
		return value.Convert(targetType), true
	}
	if targetType.Kind() == reflect.Interface && value.Type().Implements(targetType) {
		return value, true
	}
	if targetType == commentPtrType || targetType == commentGroupPtrType {
		return reflect.Value{}, false
	}
	m.addDiagnostic("unsupported_field_type", "structural", ownerNodeKind, fieldName, "", "", fmt.Sprintf("materialized value %s cannot populate go/ast field %s", value.Type(), targetType))
	return reflect.Value{}, false
}

func (m *astMaterializer) finalizeNode(nodeKind string, ptr reflect.Value) {
	switch nodeKind {
	case "BasicLit":
		lit := ptr.Interface().(*ast.BasicLit)
		if lit.Kind == token.ILLEGAL && lit.Value != "" {
			lit.Kind = inferBasicLitKind(lit.Value)
			m.inferredBasicLitKinds++
		}
	case "FuncType":
		funcType := ptr.Interface().(*ast.FuncType)
		if funcType.Params == nil {
			funcType.Params = &ast.FieldList{}
		}
	case "File":
		file := ptr.Interface().(*ast.File)
		if file.Name == nil {
			m.addDiagnostic("missing_required_field", "structural", nodeKind, "Name", "", "", "go/ast File.Name is required for formatting")
		}
	}
}

func (m *astMaterializer) summary() oracleMaterializationSummary {
	diagnosticStructural := 0
	diagnosticComments := 0
	for _, diagnostic := range m.diagnostics {
		if diagnostic.Category == "comment_directive_policy" {
			diagnosticComments++
		} else {
			diagnosticStructural++
		}
	}
	return oracleMaterializationSummary{
		NodeCount:                     m.nodeCount,
		FieldCount:                    m.fieldCount,
		TerminalPayloadCount:          m.terminalCount,
		DistinctNodeKinds:             sortedKeys(m.nodeKinds),
		DistinctFields:                sortedKeys(m.fields),
		ReflectedFieldAssignments:     m.assignments,
		TerminalCoercions:             m.terminalCoercions,
		SynthesizedPositionCount:      m.synthesizedPositions,
		InferredBasicLitKindCount:     m.inferredBasicLitKinds,
		SkippedCommentNodeCount:       m.skippedCommentNodes,
		UnsupportedStructuralCount:    diagnosticStructural,
		UnsupportedCommentPolicyCount: diagnosticComments,
	}
}

func (m *astMaterializer) syntheticPos() token.Pos {
	pos := m.nextSyntheticPos
	m.nextSyntheticPos++
	return pos
}

func (m *astMaterializer) addDiagnostic(kind string, category string, nodeKind string, field string, terminal string, payload string, message string) {
	m.diagnostics = append(m.diagnostics, oracleDiagnostic{
		DiagnosticKind: kind,
		Category:       category,
		NodeKind:       nodeKind,
		Field:          field,
		Terminal:       terminal,
		Payload:        payload,
		Message:        message,
	})
}

func (m *astMaterializer) hasStructuralDiagnostics() bool {
	for _, diagnostic := range m.diagnostics {
		if diagnostic.Category != "comment_directive_policy" {
			return true
		}
	}
	return false
}

func (v rawDecodedValue) nodeValue() *rawDecodedNode {
	if v.Node != nil {
		return v.Node
	}
	return v.SerdeNode
}

func (v rawDecodedValue) terminalValue() *rawDecodedTerminal {
	if v.SerdeTerminal != nil {
		return v.SerdeTerminal
	}
	if v.Kind == "terminal" || v.Terminal != "" {
		return &rawDecodedTerminal{
			NodeKind: v.NodeKind,
			Terminal: v.Terminal,
			Payload:  v.Payload,
		}
	}
	return nil
}

func astNodeTypeByName(name string) (reflect.Type, bool) {
	typ, ok := map[string]reflect.Type{
		"ArrayType":      reflect.TypeOf(ast.ArrayType{}),
		"AssignStmt":     reflect.TypeOf(ast.AssignStmt{}),
		"BadDecl":        reflect.TypeOf(ast.BadDecl{}),
		"BadExpr":        reflect.TypeOf(ast.BadExpr{}),
		"BadStmt":        reflect.TypeOf(ast.BadStmt{}),
		"BasicLit":       reflect.TypeOf(ast.BasicLit{}),
		"BinaryExpr":     reflect.TypeOf(ast.BinaryExpr{}),
		"BlockStmt":      reflect.TypeOf(ast.BlockStmt{}),
		"BranchStmt":     reflect.TypeOf(ast.BranchStmt{}),
		"CallExpr":       reflect.TypeOf(ast.CallExpr{}),
		"CaseClause":     reflect.TypeOf(ast.CaseClause{}),
		"ChanType":       reflect.TypeOf(ast.ChanType{}),
		"CommClause":     reflect.TypeOf(ast.CommClause{}),
		"Comment":        reflect.TypeOf(ast.Comment{}),
		"CommentGroup":   reflect.TypeOf(ast.CommentGroup{}),
		"CompositeLit":   reflect.TypeOf(ast.CompositeLit{}),
		"DeclStmt":       reflect.TypeOf(ast.DeclStmt{}),
		"DeferStmt":      reflect.TypeOf(ast.DeferStmt{}),
		"Ellipsis":       reflect.TypeOf(ast.Ellipsis{}),
		"EmptyStmt":      reflect.TypeOf(ast.EmptyStmt{}),
		"ExprStmt":       reflect.TypeOf(ast.ExprStmt{}),
		"Field":          reflect.TypeOf(ast.Field{}),
		"FieldList":      reflect.TypeOf(ast.FieldList{}),
		"File":           reflect.TypeOf(ast.File{}),
		"ForStmt":        reflect.TypeOf(ast.ForStmt{}),
		"FuncDecl":       reflect.TypeOf(ast.FuncDecl{}),
		"FuncLit":        reflect.TypeOf(ast.FuncLit{}),
		"FuncType":       reflect.TypeOf(ast.FuncType{}),
		"GenDecl":        reflect.TypeOf(ast.GenDecl{}),
		"GoStmt":         reflect.TypeOf(ast.GoStmt{}),
		"Ident":          reflect.TypeOf(ast.Ident{}),
		"IfStmt":         reflect.TypeOf(ast.IfStmt{}),
		"ImportSpec":     reflect.TypeOf(ast.ImportSpec{}),
		"IncDecStmt":     reflect.TypeOf(ast.IncDecStmt{}),
		"IndexExpr":      reflect.TypeOf(ast.IndexExpr{}),
		"IndexListExpr":  reflect.TypeOf(ast.IndexListExpr{}),
		"InterfaceType":  reflect.TypeOf(ast.InterfaceType{}),
		"KeyValueExpr":   reflect.TypeOf(ast.KeyValueExpr{}),
		"LabeledStmt":    reflect.TypeOf(ast.LabeledStmt{}),
		"MapType":        reflect.TypeOf(ast.MapType{}),
		"Package":        reflect.TypeOf(ast.Package{}),
		"ParenExpr":      reflect.TypeOf(ast.ParenExpr{}),
		"RangeStmt":      reflect.TypeOf(ast.RangeStmt{}),
		"ReturnStmt":     reflect.TypeOf(ast.ReturnStmt{}),
		"SelectStmt":     reflect.TypeOf(ast.SelectStmt{}),
		"SelectorExpr":   reflect.TypeOf(ast.SelectorExpr{}),
		"SendStmt":       reflect.TypeOf(ast.SendStmt{}),
		"SliceExpr":      reflect.TypeOf(ast.SliceExpr{}),
		"StarExpr":       reflect.TypeOf(ast.StarExpr{}),
		"StructType":     reflect.TypeOf(ast.StructType{}),
		"SwitchStmt":     reflect.TypeOf(ast.SwitchStmt{}),
		"TypeAssertExpr": reflect.TypeOf(ast.TypeAssertExpr{}),
		"TypeSpec":       reflect.TypeOf(ast.TypeSpec{}),
		"TypeSwitchStmt": reflect.TypeOf(ast.TypeSwitchStmt{}),
		"UnaryExpr":      reflect.TypeOf(ast.UnaryExpr{}),
		"ValueSpec":      reflect.TypeOf(ast.ValueSpec{}),
	}[name]
	return typ, ok
}

func safeFormatGoAstFile(file *ast.File) (formatted string, err error) {
	defer func() {
		if recovered := recover(); recovered != nil {
			err = fmt.Errorf("go/format.Node panicked: %v", recovered)
		}
	}()
	var buf bytes.Buffer
	if err := format.Node(&buf, syntheticFileSetForAstFile(file), file); err != nil {
		return "", err
	}
	return buf.String(), nil
}

func syntheticFileSetForAstFile(file *ast.File) *token.FileSet {
	fset := token.NewFileSet()
	maxPos := maxAstTokenPos(file)
	if maxPos == token.NoPos {
		return fset
	}
	tokenFile := fset.AddFile("go-raw-tree-ast-printer-oracle.go", 1, int(maxPos)+1024)
	lineStarts := astFileLineStartPositions(file)
	sort.Slice(lineStarts, func(i, j int) bool {
		return lineStarts[i] < lineStarts[j]
	})
	var previous token.Pos
	for _, pos := range lineStarts {
		if pos == token.NoPos || pos == previous {
			continue
		}
		previous = pos
		offset := int(pos) - tokenFile.Base()
		if offset <= 0 || offset >= tokenFile.Size() {
			continue
		}
		tokenFile.AddLine(offset)
	}
	return fset
}

func maxAstTokenPos(file *ast.File) token.Pos {
	if file == nil {
		return token.NoPos
	}
	var maxPos token.Pos
	add := func(pos token.Pos) {
		if pos > maxPos {
			maxPos = pos
		}
	}
	ast.Inspect(file, func(node ast.Node) bool {
		if node == nil {
			return true
		}
		add(node.Pos())
		add(node.End())
		return true
	})
	for _, group := range file.Comments {
		if group == nil {
			continue
		}
		add(group.Pos())
		add(group.End())
		for _, comment := range group.List {
			if comment == nil {
				continue
			}
			add(comment.Pos())
			add(comment.End())
		}
	}
	return maxPos
}

func astFileLineStartPositions(file *ast.File) []token.Pos {
	if file == nil {
		return nil
	}
	var positions []token.Pos
	add := func(pos token.Pos) {
		if pos != token.NoPos {
			positions = append(positions, pos)
		}
	}
	add(file.Package)
	for _, group := range file.Comments {
		if group == nil {
			continue
		}
		for _, comment := range group.List {
			if comment != nil {
				add(comment.Slash)
			}
		}
	}
	for _, decl := range file.Decls {
		if decl != nil {
			add(decl.Pos())
		}
	}
	return positions
}

func grammarModeEssentialFileCommentGroups(file *ast.File) []*ast.CommentGroup {
	attachedGroups := collectAttachedEssentialCommentGroups(file)
	attachedByText := map[string][]*ast.CommentGroup{}
	for _, group := range attachedGroups {
		key := commentGroupTextKey(group)
		if key == "" {
			continue
		}
		attachedByText[key] = append(attachedByText[key], group)
	}

	var retained []*ast.CommentGroup
	usedAttached := map[*ast.CommentGroup]struct{}{}
	for _, group := range file.Comments {
		if group == nil {
			continue
		}
		if !commentGroupHasEssentialCategory(group) {
			continue
		}
		key := commentGroupTextKey(group)
		if candidates := attachedByText[key]; len(candidates) > 0 {
			attached := candidates[0]
			attachedByText[key] = candidates[1:]
			retained = append(retained, attached)
			usedAttached[attached] = struct{}{}
		} else {
			retained = append(retained, group)
		}
	}
	for _, group := range attachedGroups {
		if _, ok := usedAttached[group]; ok {
			continue
		}
		retained = append(retained, group)
	}
	return retained
}

func commentGroupHasCategory(group *ast.CommentGroup, category string) bool {
	for _, comment := range group.List {
		for _, current := range essentialGoCommentCategories(comment.Text) {
			if current == category {
				return true
			}
		}
	}
	return false
}

func commentGroupHasEssentialCategory(group *ast.CommentGroup) bool {
	if group == nil {
		return false
	}
	for _, comment := range group.List {
		if comment == nil {
			continue
		}
		if len(essentialGoCommentCategories(comment.Text)) > 0 {
			return true
		}
	}
	return false
}

func collectAttachedEssentialCommentGroups(file *ast.File) []*ast.CommentGroup {
	if file == nil {
		return nil
	}
	var groups []*ast.CommentGroup
	add := func(group *ast.CommentGroup) {
		if commentGroupHasEssentialCategory(group) {
			groups = append(groups, group)
		}
	}
	add(file.Doc)
	for _, decl := range file.Decls {
		ast.Inspect(decl, func(node ast.Node) bool {
			switch current := node.(type) {
			case *ast.GenDecl:
				add(current.Doc)
			case *ast.FuncDecl:
				add(current.Doc)
			case *ast.ImportSpec:
				add(current.Doc)
				add(current.Comment)
			case *ast.ValueSpec:
				add(current.Doc)
				add(current.Comment)
			case *ast.TypeSpec:
				add(current.Doc)
				add(current.Comment)
			case *ast.Field:
				add(current.Doc)
				add(current.Comment)
			}
			return true
		})
	}
	return groups
}

func commentGroupTextKey(group *ast.CommentGroup) string {
	if group == nil {
		return ""
	}
	parts := make([]string, 0, len(group.List))
	for _, comment := range group.List {
		if comment == nil {
			continue
		}
		parts = append(parts, comment.Text)
	}
	return strings.Join(parts, "\x00")
}

func prepareGrammarModeAstFile(file *ast.File) {
	assigner := grammarModePositionAssigner{next: 1}
	for _, group := range file.Comments {
		if commentGroupHasCategory(group, "go_build_constraint") || commentGroupHasCategory(group, "legacy_build_constraint") {
			assigner.assignCommentGroup(group)
		}
	}
	assigner.assignCommentGroup(file.Doc)
	assigner.assignFilePackage(file)
	for _, decl := range file.Decls {
		assigner.assignDecl(decl)
	}
	for _, group := range file.Comments {
		assigner.assignCommentGroup(group)
	}
	sort.SliceStable(file.Comments, func(i, j int) bool {
		return file.Comments[i].Pos() < file.Comments[j].Pos()
	})
}

type grammarModePositionAssigner struct {
	next token.Pos
}

func (p *grammarModePositionAssigner) pos() token.Pos {
	if p.next == token.NoPos {
		p.next = 1
	}
	current := p.next
	p.next += 8
	return current
}

func (p *grammarModePositionAssigner) assignFilePackage(file *ast.File) {
	if file.Package == token.NoPos {
		file.Package = p.pos()
	}
	p.assignIdent(file.Name)
}

func (p *grammarModePositionAssigner) assignCommentGroup(group *ast.CommentGroup) {
	if group == nil {
		return
	}
	for _, comment := range group.List {
		if comment == nil {
			continue
		}
		if comment.Slash == token.NoPos {
			comment.Slash = p.next
			if comment.Slash == token.NoPos {
				comment.Slash = 1
			}
		}
		p.reservePast(comment.End())
	}
}

func (p *grammarModePositionAssigner) reservePast(pos token.Pos) {
	if pos >= p.next {
		p.next = pos + 8
	}
}

func (p *grammarModePositionAssigner) assignDecl(decl ast.Decl) {
	switch current := decl.(type) {
	case *ast.GenDecl:
		p.assignCommentGroup(current.Doc)
		if current.TokPos == token.NoPos {
			current.TokPos = p.pos()
		}
		for _, spec := range current.Specs {
			p.assignSpec(spec)
		}
	case *ast.FuncDecl:
		p.assignCommentGroup(current.Doc)
		p.assignFuncToken(current.Type)
		p.assignFieldList(current.Recv)
		p.assignIdent(current.Name)
		p.assignFuncType(current.Type)
		p.assignBlockStmt(current.Body)
	case *ast.BadDecl:
		if current.From == token.NoPos {
			current.From = p.pos()
		}
		if current.To == token.NoPos {
			current.To = p.pos()
		}
	}
}

func (p *grammarModePositionAssigner) assignSpec(spec ast.Spec) {
	switch current := spec.(type) {
	case *ast.ImportSpec:
		p.assignCommentGroup(current.Doc)
		p.assignIdent(current.Name)
		p.assignExpr(current.Path)
		p.assignCommentGroup(current.Comment)
	case *ast.ValueSpec:
		p.assignCommentGroup(current.Doc)
		for _, name := range current.Names {
			p.assignIdent(name)
		}
		p.assignExpr(current.Type)
		for _, value := range current.Values {
			p.assignExpr(value)
		}
		p.assignCommentGroup(current.Comment)
	case *ast.TypeSpec:
		p.assignCommentGroup(current.Doc)
		p.assignIdent(current.Name)
		p.assignFieldList(current.TypeParams)
		p.assignExpr(current.Type)
		p.assignCommentGroup(current.Comment)
	}
}

func (p *grammarModePositionAssigner) assignFuncType(funcType *ast.FuncType) {
	if funcType == nil {
		return
	}
	p.assignFuncToken(funcType)
	p.assignFieldList(funcType.TypeParams)
	p.assignFieldList(funcType.Params)
	p.assignFieldList(funcType.Results)
}

func (p *grammarModePositionAssigner) assignFuncToken(funcType *ast.FuncType) {
	if funcType == nil {
		return
	}
	if funcType.Func == token.NoPos {
		funcType.Func = p.pos()
	}
}

func (p *grammarModePositionAssigner) assignFieldList(list *ast.FieldList) {
	if list == nil {
		return
	}
	if list.Opening == token.NoPos {
		list.Opening = p.pos()
	}
	for _, field := range list.List {
		p.assignCommentGroup(field.Doc)
		for _, name := range field.Names {
			p.assignIdent(name)
		}
		p.assignExpr(field.Type)
		p.assignCommentGroup(field.Comment)
	}
	if list.Closing == token.NoPos {
		list.Closing = p.pos()
	}
}

func (p *grammarModePositionAssigner) assignBlockStmt(block *ast.BlockStmt) {
	if block == nil {
		return
	}
	if block.Lbrace == token.NoPos {
		block.Lbrace = p.pos()
	}
	for _, stmt := range block.List {
		p.assignStmt(stmt)
	}
	if block.Rbrace == token.NoPos {
		block.Rbrace = p.pos()
	}
}

func (p *grammarModePositionAssigner) assignStmt(stmt ast.Stmt) {
	switch current := stmt.(type) {
	case *ast.AssignStmt:
		for _, expr := range current.Lhs {
			p.assignExpr(expr)
		}
		if current.TokPos == token.NoPos {
			current.TokPos = p.pos()
		}
		for _, expr := range current.Rhs {
			p.assignExpr(expr)
		}
	case *ast.DeclStmt:
		p.assignDecl(current.Decl)
	case *ast.ExprStmt:
		p.assignExpr(current.X)
	case *ast.ReturnStmt:
		if current.Return == token.NoPos {
			current.Return = p.pos()
		}
		for _, expr := range current.Results {
			p.assignExpr(expr)
		}
	case *ast.BlockStmt:
		p.assignBlockStmt(current)
	case *ast.IfStmt:
		if current.If == token.NoPos {
			current.If = p.pos()
		}
		p.assignStmt(current.Init)
		p.assignExpr(current.Cond)
		p.assignBlockStmt(current.Body)
		p.assignStmt(current.Else)
	case *ast.ForStmt:
		if current.For == token.NoPos {
			current.For = p.pos()
		}
		p.assignStmt(current.Init)
		p.assignExpr(current.Cond)
		p.assignStmt(current.Post)
		p.assignBlockStmt(current.Body)
	case *ast.RangeStmt:
		if current.For == token.NoPos {
			current.For = p.pos()
		}
		p.assignExpr(current.Key)
		p.assignExpr(current.Value)
		p.assignExpr(current.X)
		p.assignBlockStmt(current.Body)
	case *ast.SwitchStmt:
		if current.Switch == token.NoPos {
			current.Switch = p.pos()
		}
		p.assignStmt(current.Init)
		p.assignExpr(current.Tag)
		p.assignBlockStmt(current.Body)
	case *ast.TypeSwitchStmt:
		if current.Switch == token.NoPos {
			current.Switch = p.pos()
		}
		p.assignStmt(current.Init)
		p.assignStmt(current.Assign)
		p.assignBlockStmt(current.Body)
	case *ast.CaseClause:
		if current.Case == token.NoPos {
			current.Case = p.pos()
		}
		for _, expr := range current.List {
			p.assignExpr(expr)
		}
		for _, stmt := range current.Body {
			p.assignStmt(stmt)
		}
	case *ast.BranchStmt:
		if current.TokPos == token.NoPos {
			current.TokPos = p.pos()
		}
		p.assignIdent(current.Label)
	case *ast.IncDecStmt:
		p.assignExpr(current.X)
		if current.TokPos == token.NoPos {
			current.TokPos = p.pos()
		}
	case *ast.GoStmt:
		if current.Go == token.NoPos {
			current.Go = p.pos()
		}
		p.assignExpr(current.Call)
	case *ast.DeferStmt:
		if current.Defer == token.NoPos {
			current.Defer = p.pos()
		}
		p.assignExpr(current.Call)
	case *ast.LabeledStmt:
		p.assignIdent(current.Label)
		p.assignStmt(current.Stmt)
	case *ast.SendStmt:
		p.assignExpr(current.Chan)
		p.assignExpr(current.Value)
	case *ast.SelectStmt:
		if current.Select == token.NoPos {
			current.Select = p.pos()
		}
		p.assignBlockStmt(current.Body)
	case *ast.CommClause:
		if current.Case == token.NoPos {
			current.Case = p.pos()
		}
		p.assignStmt(current.Comm)
		for _, stmt := range current.Body {
			p.assignStmt(stmt)
		}
	}
}

func (p *grammarModePositionAssigner) assignExpr(expr ast.Expr) {
	switch current := expr.(type) {
	case *ast.Ident:
		p.assignIdent(current)
	case *ast.BasicLit:
		if current.ValuePos == token.NoPos {
			current.ValuePos = p.pos()
		}
	case *ast.BinaryExpr:
		p.assignExpr(current.X)
		if current.OpPos == token.NoPos {
			current.OpPos = p.pos()
		}
		p.assignExpr(current.Y)
	case *ast.CallExpr:
		p.assignExpr(current.Fun)
		if current.Lparen == token.NoPos {
			current.Lparen = p.pos()
		}
		for _, arg := range current.Args {
			p.assignExpr(arg)
		}
		if current.Rparen == token.NoPos {
			current.Rparen = p.pos()
		}
	case *ast.CompositeLit:
		p.assignExpr(current.Type)
		if current.Lbrace == token.NoPos {
			current.Lbrace = p.pos()
		}
		for _, elt := range current.Elts {
			p.assignExpr(elt)
		}
		if current.Rbrace == token.NoPos {
			current.Rbrace = p.pos()
		}
	case *ast.FuncLit:
		p.assignFuncType(current.Type)
		p.assignBlockStmt(current.Body)
	case *ast.FuncType:
		p.assignFuncType(current)
	case *ast.IndexExpr:
		p.assignExpr(current.X)
		if current.Lbrack == token.NoPos {
			current.Lbrack = p.pos()
		}
		p.assignExpr(current.Index)
		if current.Rbrack == token.NoPos {
			current.Rbrack = p.pos()
		}
	case *ast.IndexListExpr:
		p.assignExpr(current.X)
		if current.Lbrack == token.NoPos {
			current.Lbrack = p.pos()
		}
		for _, index := range current.Indices {
			p.assignExpr(index)
		}
		if current.Rbrack == token.NoPos {
			current.Rbrack = p.pos()
		}
	case *ast.KeyValueExpr:
		p.assignExpr(current.Key)
		if current.Colon == token.NoPos {
			current.Colon = p.pos()
		}
		p.assignExpr(current.Value)
	case *ast.ParenExpr:
		if current.Lparen == token.NoPos {
			current.Lparen = p.pos()
		}
		p.assignExpr(current.X)
		if current.Rparen == token.NoPos {
			current.Rparen = p.pos()
		}
	case *ast.SelectorExpr:
		p.assignExpr(current.X)
		p.assignIdent(current.Sel)
	case *ast.SliceExpr:
		p.assignExpr(current.X)
		if current.Lbrack == token.NoPos {
			current.Lbrack = p.pos()
		}
		p.assignExpr(current.Low)
		p.assignExpr(current.High)
		p.assignExpr(current.Max)
		if current.Rbrack == token.NoPos {
			current.Rbrack = p.pos()
		}
	case *ast.StarExpr:
		if current.Star == token.NoPos {
			current.Star = p.pos()
		}
		p.assignExpr(current.X)
	case *ast.UnaryExpr:
		if current.OpPos == token.NoPos {
			current.OpPos = p.pos()
		}
		p.assignExpr(current.X)
	case *ast.ArrayType:
		if current.Lbrack == token.NoPos {
			current.Lbrack = p.pos()
		}
		p.assignExpr(current.Len)
		p.assignExpr(current.Elt)
	case *ast.Ellipsis:
		if current.Ellipsis == token.NoPos {
			current.Ellipsis = p.pos()
		}
		p.assignExpr(current.Elt)
	case *ast.MapType:
		if current.Map == token.NoPos {
			current.Map = p.pos()
		}
		p.assignExpr(current.Key)
		p.assignExpr(current.Value)
	case *ast.StructType:
		if current.Struct == token.NoPos {
			current.Struct = p.pos()
		}
		p.assignFieldList(current.Fields)
	case *ast.InterfaceType:
		if current.Interface == token.NoPos {
			current.Interface = p.pos()
		}
		p.assignFieldList(current.Methods)
	case *ast.ChanType:
		if current.Begin == token.NoPos {
			current.Begin = p.pos()
		}
		p.assignExpr(current.Value)
	case *ast.TypeAssertExpr:
		p.assignExpr(current.X)
		if current.Lparen == token.NoPos {
			current.Lparen = p.pos()
		}
		p.assignExpr(current.Type)
		if current.Rparen == token.NoPos {
			current.Rparen = p.pos()
		}
	}
}

func (p *grammarModePositionAssigner) assignIdent(ident *ast.Ident) {
	if ident == nil {
		return
	}
	if ident.NamePos == token.NoPos {
		ident.NamePos = p.pos()
	}
}

func collectRawTreeComments(node *rawDecodedNode) []string {
	var comments []string
	var walk func(*rawDecodedNode)
	walk = func(current *rawDecodedNode) {
		if current == nil {
			return
		}
		if current.NodeKind == "Comment" {
			for _, field := range current.Fields {
				if field.Name != "Text" {
					continue
				}
				for _, value := range field.Values {
					if terminal := value.terminalValue(); terminal != nil {
						comments = append(comments, terminal.Payload)
					}
				}
			}
		}
		for _, field := range current.Fields {
			for _, value := range field.Values {
				walk(value.nodeValue())
			}
		}
	}
	walk(node)
	return comments
}

func essentialGoComments(comments []string) ([]string, []string) {
	categorySet := map[string]struct{}{}
	var essential []string
	for _, comment := range comments {
		categories := essentialGoCommentCategories(comment)
		if len(categories) == 0 {
			continue
		}
		essential = append(essential, comment)
		for _, category := range categories {
			categorySet[category] = struct{}{}
		}
	}
	return essential, sortedKeys(categorySet)
}

func collectAstFileCommentTexts(file *ast.File) []string {
	var comments []string
	if file == nil {
		return comments
	}
	for _, group := range file.Comments {
		if group == nil {
			continue
		}
		for _, comment := range group.List {
			if comment != nil {
				comments = append(comments, comment.Text)
			}
		}
	}
	return comments
}

func missingEssentialComments(rawEssential []string, renderedComments []string) []string {
	rendered := map[string]struct{}{}
	for _, comment := range renderedComments {
		rendered[comment] = struct{}{}
	}
	missingSet := map[string]struct{}{}
	for _, comment := range rawEssential {
		if _, ok := rendered[comment]; !ok {
			missingSet[comment] = struct{}{}
		}
	}
	return sortedKeys(missingSet)
}

func essentialGoCommentCategories(comment string) []string {
	trimmed := strings.TrimLeft(comment, " \t")
	var categories []string
	if strings.HasPrefix(trimmed, "//go:build") {
		categories = append(categories, "go_build_constraint")
	}
	if strings.HasPrefix(trimmed, "// +build") {
		categories = append(categories, "legacy_build_constraint")
	}
	if strings.HasPrefix(trimmed, "//go:") && !strings.HasPrefix(trimmed, "//go:build") {
		categories = append(categories, "go_directive")
	}
	if strings.HasPrefix(trimmed, "//line ") || strings.HasPrefix(trimmed, "/*line ") {
		categories = append(categories, "line_directive")
	}
	if isCgoPreambleComment(trimmed) {
		categories = append(categories, "cgo_preamble")
	}
	return categories
}

func isCgoPreambleComment(trimmed string) bool {
	return strings.HasPrefix(trimmed, "// #cgo") ||
		strings.HasPrefix(trimmed, "//#cgo") ||
		strings.HasPrefix(trimmed, "// import \"C\"") ||
		strings.HasPrefix(trimmed, "//import \"C\"") ||
		strings.HasPrefix(trimmed, "// #include") ||
		strings.HasPrefix(trimmed, "//#include") ||
		strings.HasPrefix(trimmed, "// #define") ||
		strings.HasPrefix(trimmed, "//#define") ||
		(strings.HasPrefix(trimmed, "/*") &&
			(strings.Contains(trimmed, "#cgo") ||
				strings.Contains(trimmed, "import \"C\"") ||
				strings.Contains(trimmed, "#include") ||
				strings.Contains(trimmed, "#define")))
}

func commentDirectiveReadiness(mode string, commentCount int, essentialCount int, missingEssential []string) string {
	if mode == goRawTreeAstPrinterOracleStructureMode && essentialCount > 0 {
		return "essential_comments_or_directives_unsupported_structure_only"
	}
	if mode == goRawTreeAstPrinterOracleGrammarMode && len(missingEssential) > 0 {
		return "essential_comments_or_directives_missing_grammar_mode"
	}
	if mode == goRawTreeAstPrinterOracleGrammarMode && essentialCount > 0 {
		return "essential_comments_preserved_or_absent"
	}
	if commentCount > 0 {
		return "canonical_fidelity_only_nonessential_comments_dropped_or_reflowed"
	}
	return "comments_absent"
}

func commentDirectiveBlocked(readiness string) bool {
	return readiness == "essential_comments_or_directives_unsupported_structure_only" ||
		readiness == "essential_comments_or_directives_missing_grammar_mode"
}

func isCommentPolicyField(name string) bool {
	return strings.Contains(name, "Comment") ||
		strings.Contains(name, "BlankLine") ||
		strings.Contains(name, "LineBreak") ||
		strings.Contains(name, "Doc")
}

func tokenFromPayload(payload string) (token.Token, bool) {
	normalized := strings.TrimSpace(payload)
	if normalized == "" {
		return token.ILLEGAL, false
	}
	if normalized == "ILLEGAL" {
		// token.ILLEGAL is a legal finite-domain value for RangeStmt.Tok: a
		// bare `for range x` has no Key/Value and records Tok as ILLEGAL.
		return token.ILLEGAL, true
	}
	if tok := token.Lookup(normalized); tok != token.IDENT {
		return tok, true
	}
	if tok, ok := map[string]token.Token{
		"IDENT":  token.IDENT,
		"INT":    token.INT,
		"FLOAT":  token.FLOAT,
		"IMAG":   token.IMAG,
		"CHAR":   token.CHAR,
		"STRING": token.STRING,
		"VAR":    token.VAR,
		"CONST":  token.CONST,
		"TYPE":   token.TYPE,
		"IMPORT": token.IMPORT,
		"FUNC":   token.FUNC,
		"=":      token.ASSIGN,
		":=":     token.DEFINE,
		"+":      token.ADD,
		"-":      token.SUB,
		"*":      token.MUL,
		"/":      token.QUO,
		"%":      token.REM,
		"&":      token.AND,
		"|":      token.OR,
		"^":      token.XOR,
		"<<":     token.SHL,
		">>":     token.SHR,
		"&^":     token.AND_NOT,
		"+=":     token.ADD_ASSIGN,
		"-=":     token.SUB_ASSIGN,
		"*=":     token.MUL_ASSIGN,
		"/=":     token.QUO_ASSIGN,
		"%=":     token.REM_ASSIGN,
		"&=":     token.AND_ASSIGN,
		"|=":     token.OR_ASSIGN,
		"^=":     token.XOR_ASSIGN,
		"<<=":    token.SHL_ASSIGN,
		">>=":    token.SHR_ASSIGN,
		"&^=":    token.AND_NOT_ASSIGN,
		"&&":     token.LAND,
		"||":     token.LOR,
		"<-":     token.ARROW,
		"++":     token.INC,
		"--":     token.DEC,
		"==":     token.EQL,
		"<":      token.LSS,
		">":      token.GTR,
		"!":      token.NOT,
		"!=":     token.NEQ,
		"<=":     token.LEQ,
		">=":     token.GEQ,
		"...":    token.ELLIPSIS,
		"(":      token.LPAREN,
		")":      token.RPAREN,
		"[":      token.LBRACK,
		"]":      token.RBRACK,
		"{":      token.LBRACE,
		"}":      token.RBRACE,
		",":      token.COMMA,
		".":      token.PERIOD,
		";":      token.SEMICOLON,
		":":      token.COLON,
	}[normalized]; ok {
		return tok, true
	}
	if tok, ok := map[string]token.Token{
		"identifier": token.IDENT,
		"integer":    token.INT,
		"float":      token.FLOAT,
		"imaginary":  token.IMAG,
		"rune":       token.CHAR,
		"string":     token.STRING,
	}[strings.ToLower(normalized)]; ok {
		return tok, true
	}
	return token.ILLEGAL, false
}

func chanDirFromPayload(payload string) (ast.ChanDir, bool) {
	switch strings.TrimSpace(strings.ToLower(payload)) {
	case "send", "sendonly", "send_only", "1":
		return ast.SEND, true
	case "recv", "recvonly", "recv_only", "receive", "2":
		return ast.RECV, true
	case "both", "sendrecv", "send_recv", "3":
		return ast.SEND | ast.RECV, true
	default:
		return 0, false
	}
}

func inferBasicLitKind(value string) token.Token {
	trimmed := strings.TrimSpace(value)
	if trimmed == "" {
		return token.STRING
	}
	if strings.HasPrefix(trimmed, "\"") || strings.HasPrefix(trimmed, "`") {
		return token.STRING
	}
	if strings.HasPrefix(trimmed, "'") {
		return token.CHAR
	}
	if strings.HasSuffix(trimmed, "i") {
		return token.IMAG
	}
	if strings.ContainsAny(trimmed, ".eEpP") {
		return token.FLOAT
	}
	return token.INT
}

func terminalIdent(payload string, terminal rawDecodedTerminal) *ast.Ident {
	if terminal.Terminal == "identifier_payload" || strings.Contains(terminal.NodeKind, "identifier") {
		return &ast.Ident{Name: payload}
	}
	return nil
}

func isFalsePayload(payload string) bool {
	normalized := strings.TrimSpace(strings.ToLower(payload))
	return normalized == "" || normalized == "false" || normalized == "0" || normalized == "missing"
}

func oracleStatus(report *goRawTreeAstPrinterOracleReport) string {
	if report.StructuralBlockedCount > 0 {
		return goRawTreeAstPrinterOracleStatusStructuralBlocked
	}
	if report.CommentDirectiveBlockedCount > 0 {
		return goRawTreeAstPrinterOracleStatusCommentPolicyBlocked
	}
	return goRawTreeAstPrinterOracleStatusPassed
}

func populateAdoptionDecisionFields(report *goRawTreeAstPrinterOracleReport) {
	for _, record := range report.Records {
		if record.CommentDirectiveReadiness == "essential_comments_or_directives_unsupported_structure_only" {
			report.EssentialCommentBlockers = append(report.EssentialCommentBlockers, oracleAdoptionFinding{
				SourceLabel:    record.SourceLabel,
				Category:       record.Category,
				ManifestPath:   record.ManifestPath,
				SourcePath:     record.SourcePath,
				DiagnosticKind: "essential_comment_policy_not_enabled",
				Message:        "structure-only oracle mode does not preserve essential Go comments/directives",
			})
		}
		if record.CommentCount > record.EssentialCommentCount {
			report.CanonicalFidelityOnlyGaps = append(report.CanonicalFidelityOnlyGaps, oracleAdoptionFinding{
				SourceLabel:    record.SourceLabel,
				Category:       record.Category,
				ManifestPath:   record.ManifestPath,
				SourcePath:     record.SourcePath,
				DiagnosticKind: "nonessential_comment_or_layout_fidelity",
				Message:        "nonessential comments, exact inline placement, blank lines, or source layout remain canonical-fidelity-only diagnostics",
			})
		}
		if !record.CurrentRustRendererGrammarMode.RawTreeCanonicalReady {
			report.CanonicalFidelityOnlyGaps = append(report.CanonicalFidelityOnlyGaps, oracleAdoptionFinding{
				SourceLabel:    record.SourceLabel,
				Category:       record.Category,
				ManifestPath:   record.ManifestPath,
				SourcePath:     record.SourcePath,
				DiagnosticKind: "current_rust_renderer_canonical_fidelity_gap",
				Message:        "current Rust renderer grammar-mode evidence is ready but exact canonical source fidelity is not ready for this record",
			})
		}
		for _, diagnostic := range record.Diagnostics {
			finding := adoptionFindingFor(record, diagnostic)
			switch diagnostic.Category {
			case "parser", "formatter":
				report.ParserBlockers = append(report.ParserBlockers, finding)
			case "comment_directive_policy":
				if diagnostic.DiagnosticKind == "essential_comment_not_preserved" {
					report.EssentialCommentBlockers = append(report.EssentialCommentBlockers, finding)
				} else {
					report.CanonicalFidelityOnlyGaps = append(report.CanonicalFidelityOnlyGaps, finding)
				}
			default:
				report.StructuralBlockers = append(report.StructuralBlockers, finding)
			}
		}
	}

	report.FullAdoptionPossible = report.Mode == goRawTreeAstPrinterOracleGrammarMode &&
		report.RecordCount > 0 &&
		report.StructuralBlockedCount == 0 &&
		report.CommentDirectiveBlockedCount == 0 &&
		len(report.StructuralBlockers) == 0 &&
		len(report.EssentialCommentBlockers) == 0 &&
		len(report.ParserBlockers) == 0
	report.NextConcreteTask = nextConcreteTaskForAdoption(report)
}

func adoptionFindingFor(record goRawTreeAstPrinterOracleRecordReport, diagnostic oracleDiagnostic) oracleAdoptionFinding {
	return oracleAdoptionFinding{
		SourceLabel:    record.SourceLabel,
		Category:       record.Category,
		ManifestPath:   record.ManifestPath,
		SourcePath:     record.SourcePath,
		DiagnosticKind: diagnostic.DiagnosticKind,
		NodeKind:       diagnostic.NodeKind,
		Field:          diagnostic.Field,
		Terminal:       diagnostic.Terminal,
		Payload:        diagnostic.Payload,
		Message:        diagnostic.Message,
	}
}

func nextConcreteTaskForAdoption(report *goRawTreeAstPrinterOracleReport) string {
	if report.FullAdoptionPossible {
		return goRawTreeAstPrinterOracleSuccessorTask
	}
	if len(report.StructuralBlockers) > 0 {
		blocker := report.StructuralBlockers[0]
		return fmt.Sprintf("Fix the first reusable Go AST printer oracle structural blocker (%s on %s.%s) before accepting the grammar-mode oracle cutover.", blocker.DiagnosticKind, emptyAsUnknown(blocker.NodeKind), emptyAsUnknown(blocker.Field))
	}
	if len(report.EssentialCommentBlockers) > 0 {
		blocker := report.EssentialCommentBlockers[0]
		return fmt.Sprintf("Fix the first Go AST printer oracle essential-comment blocker (%s for %s) before accepting the grammar-mode oracle cutover.", blocker.DiagnosticKind, emptyAsUnknown(blocker.SourceLabel))
	}
	if len(report.ParserBlockers) > 0 {
		blocker := report.ParserBlockers[0]
		return fmt.Sprintf("Fix the first Go AST printer oracle formatter/parser blocker (%s for %s) before accepting the grammar-mode oracle cutover.", blocker.DiagnosticKind, emptyAsUnknown(blocker.SourceLabel))
	}
	if report.Mode != goRawTreeAstPrinterOracleGrammarMode {
		return "Run the Go AST printer oracle in grammar-mode across every current-plus raw-tree render attempt before making a full renderer adoption decision."
	}
	return "Do not accept the grammar-mode oracle cutover until the report records full_adoption_possible=true."
}

func emptyAsUnknown(value string) string {
	if value == "" {
		return "unknown"
	}
	return value
}

func oracleDecisionFor(report *goRawTreeAstPrinterOracleReport) oracleDecision {
	categories := map[string]struct{}{}
	for _, record := range report.Records {
		for _, diagnostic := range record.Diagnostics {
			categories[diagnostic.Category] = struct{}{}
		}
	}
	if report.StructuralBlockedCount > 0 {
		return oracleDecision{
			Recommendation:                "block_go_ast_printer_oracle_cutover",
			StructuralOraclePassed:        false,
			CommentsOrDirectivesBlocked:   report.CommentDirectiveBlockedCount > 0,
			MateriallyReducesManualPolicy: false,
			ReusableDiagnosticCategories:  sortedKeys(categories),
			NextConcreteTask:              report.NextConcreteTask,
			Rationale:                     "At least one representative decoded raw tree could not be materialized, formatted, and reparsed through go/ast and go/format.",
		}
	}
	if report.CommentDirectiveBlockedCount > 0 {
		if report.Mode == goRawTreeAstPrinterOracleGrammarMode {
			return oracleDecision{
				Recommendation:                "block_go_ast_printer_oracle_cutover",
				StructuralOraclePassed:        true,
				CommentsOrDirectivesBlocked:   true,
				MateriallyReducesManualPolicy: true,
				ReusableDiagnosticCategories:  sortedKeys(categories),
				NextConcreteTask:              report.NextConcreteTask,
				Rationale:                     "Every selected decoded raw tree rendered and reparsed structurally through go/format, but grammar-mode still dropped at least one distinct essential Go comment/directive.",
			}
		}
		return oracleDecision{
			Recommendation:                "add_essential_comment_policy_before_oracle_cutover",
			StructuralOraclePassed:        true,
			CommentsOrDirectivesBlocked:   true,
			MateriallyReducesManualPolicy: true,
			ReusableDiagnosticCategories:  sortedKeys(categories),
			NextConcreteTask:              report.NextConcreteTask,
			Rationale:                     "Every representative decoded raw tree rendered and reparsed structurally through go/format, but structure-only mode deliberately drops essential comments/directives.",
		}
	}
	if report.FullAdoptionPossible {
		return oracleDecision{
			Recommendation:                "accept_go_ast_printer_oracle_grammar_mode_cutover",
			StructuralOraclePassed:        true,
			CommentsOrDirectivesBlocked:   false,
			MateriallyReducesManualPolicy: true,
			ReusableDiagnosticCategories:  sortedKeys(categories),
			NextConcreteTask:              report.NextConcreteTask,
			Rationale:                     "Every selected current-plus decoded raw tree rendered and reparsed through go/ast, go/format, and go/parser in grammar-mode, and every distinct essential Go comment/directive text was preserved. Remaining comment/layout differences are canonical-fidelity-only diagnostics.",
		}
	}
	return oracleDecision{
		Recommendation:                "oracle_structural_probe_ready_but_cutover_unproven",
		StructuralOraclePassed:        true,
		CommentsOrDirectivesBlocked:   false,
		MateriallyReducesManualPolicy: true,
		ReusableDiagnosticCategories:  sortedKeys(categories),
		NextConcreteTask:              report.NextConcreteTask,
		Rationale:                     "Every representative decoded raw tree rendered and reparsed through go/ast, go/format, and go/parser without structural diagnostics.",
	}
}

func readOracleReport(path string) (*goRawTreeAstPrinterOracleReport, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read oracle report: %w", err)
	}
	var report goRawTreeAstPrinterOracleReport
	if err := json.Unmarshal(data, &report); err != nil {
		return nil, fmt.Errorf("parse oracle report: %w", err)
	}
	return &report, nil
}

func validateOracleReport(report *goRawTreeAstPrinterOracleReport) error {
	if report.ArtifactKind != goRawTreeAstPrinterOracleArtifactKind {
		return fmt.Errorf("artifact_kind = %q, want %q", report.ArtifactKind, goRawTreeAstPrinterOracleArtifactKind)
	}
	if report.Version != goRawTreeAstPrinterOracleVersion {
		return fmt.Errorf("version = %d, want %d", report.Version, goRawTreeAstPrinterOracleVersion)
	}
	if report.Tool != goRawTreeAstPrinterOracleTool {
		return fmt.Errorf("tool = %q, want %q", report.Tool, goRawTreeAstPrinterOracleTool)
	}
	if !isSupportedOracleMode(report.Mode) {
		return fmt.Errorf("unsupported report mode %q", report.Mode)
	}
	if report.RecordCount != len(report.Records) {
		return fmt.Errorf("record_count = %d, want %d", report.RecordCount, len(report.Records))
	}
	passed := 0
	blocked := 0
	commentBlocked := 0
	for _, record := range report.Records {
		if record.OracleStatus == "passed" {
			passed++
			if record.GoFormatValidation != "passed" || record.GoParserReparseValidation != "passed" {
				return fmt.Errorf("%s passed without format/parser validation", record.SourceLabel)
			}
		} else {
			blocked++
		}
		if commentDirectiveBlocked(record.CommentDirectiveReadiness) {
			commentBlocked++
		}
	}
	if report.StructuralPassedCount != passed {
		return fmt.Errorf("structural_passed_count = %d, want %d", report.StructuralPassedCount, passed)
	}
	if report.StructuralBlockedCount != blocked {
		return fmt.Errorf("structural_blocked_count = %d, want %d", report.StructuralBlockedCount, blocked)
	}
	if report.CommentDirectiveBlockedCount != commentBlocked {
		return fmt.Errorf("comment_directive_blocked_count = %d, want %d", report.CommentDirectiveBlockedCount, commentBlocked)
	}
	if report.Status != oracleStatus(report) {
		return fmt.Errorf("status = %q, want %q", report.Status, oracleStatus(report))
	}
	expectedFullAdoptionPossible := report.Mode == goRawTreeAstPrinterOracleGrammarMode &&
		report.RecordCount > 0 &&
		report.StructuralBlockedCount == 0 &&
		report.CommentDirectiveBlockedCount == 0 &&
		len(report.StructuralBlockers) == 0 &&
		len(report.EssentialCommentBlockers) == 0 &&
		len(report.ParserBlockers) == 0
	if report.FullAdoptionPossible != expectedFullAdoptionPossible {
		return fmt.Errorf("full_adoption_possible = %t, want %t", report.FullAdoptionPossible, expectedFullAdoptionPossible)
	}
	if report.NextConcreteTask == "" {
		return errors.New("next_concrete_task must be populated")
	}
	if report.Decision.NextConcreteTask == "" {
		return errors.New("decision.next_concrete_task must be populated")
	}
	if report.Decision.NextConcreteTask != report.NextConcreteTask {
		return errors.New("decision.next_concrete_task must match top-level next_concrete_task")
	}
	return nil
}

func isSupportedOracleMode(mode string) bool {
	return mode == goRawTreeAstPrinterOracleStructureMode || mode == goRawTreeAstPrinterOracleGrammarMode
}

func inventorySummary(path string) (oracleInventorySummary, error) {
	ref, err := fileRef(path)
	if err != nil {
		return oracleInventorySummary{}, err
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return oracleInventorySummary{}, err
	}
	var inventory struct {
		ArtifactKind string `json:"artifact_kind"`
		Version      int    `json:"version"`
		Go           struct {
			Version string `json:"version"`
		} `json:"go"`
	}
	if err := json.Unmarshal(data, &inventory); err != nil {
		return oracleInventorySummary{}, fmt.Errorf("parse AST inventory: %w", err)
	}
	return oracleInventorySummary{
		Path:         ref.Path,
		SHA256:       ref.SHA256,
		Bytes:        ref.Bytes,
		ArtifactKind: inventory.ArtifactKind,
		Version:      inventory.Version,
		GoVersion:    inventory.Go.Version,
	}, nil
}

func fileRef(path string) (oracleFileRef, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return oracleFileRef{}, fmt.Errorf("read %s: %w", path, err)
	}
	info, err := os.Stat(path)
	if err != nil {
		return oracleFileRef{}, fmt.Errorf("stat %s: %w", path, err)
	}
	return oracleFileRef{Path: path, SHA256: sha256Bytes(data), Bytes: info.Size()}, nil
}

func writeJSON(path string, value any) error {
	if dir := filepath.Dir(path); dir != "." && dir != "" {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			return err
		}
	}
	data, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		return err
	}
	data = append(data, '\n')
	return os.WriteFile(path, data, 0o644)
}

func sha256Bytes(data []byte) string {
	sum := sha256.Sum256(data)
	return hex.EncodeToString(sum[:])
}

func sortedKeys(keys map[string]struct{}) []string {
	values := make([]string, 0, len(keys))
	for key := range keys {
		values = append(values, key)
	}
	sort.Strings(values)
	return values
}
