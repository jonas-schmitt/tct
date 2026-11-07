// go_syntax_generated_codec_round_trip_gate ties a checker-ready Go source
// selection to generated-codec round-trip evidence, or records the first
// generated-codec blocker for the selected corpus.
package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strings"
)

const (
	roundTripGateTool               = "go_syntax_generated_codec_round_trip_gate"
	roundTripGateVersion            = 1
	roundTripGateArtifactKind       = "tct_go_syntax_generated_codec_round_trip_gate_v1"
	roundTripGateStatusPassed       = "passed"
	roundTripGateStatusBlocked      = "blocked"
	selectionArtifactKind           = "tct_go_syntax_corpus_selection_v1"
	checkArtifactKind               = "tct_go_syntax_corpus_check_v1"
	sourceInstanceReportReadyStatus = "source_instance_generated_codec_report_ready_manifest_promotion_blocked"
)

var roundTripNoStableClaims = []string{
	"no_broad_go_language_claim",
	"no_source_validity_by_construction_claim",
	"no_type_correctness_claim",
	"no_model_quality_claim",
	"no_paper_facing_claim",
	"no_generated_hot_path_claim",
	"no_stable_backend_claim",
	"no_arena_ir_promotion",
	"no_bpe_validity_authority",
}

type roundTripGateOptions struct {
	selectionReportPath     string
	checkerReportPath       string
	reportIngestionManifest string
	outputPath              string
	validateReportPath      string
}

type corpusSelectionReport struct {
	ArtifactKind       string                     `json:"artifact_kind"`
	Version            int                        `json:"version"`
	Tool               string                     `json:"tool"`
	Status             string                     `json:"status"`
	SourceRoot         string                     `json:"source_root"`
	SourceRevision     string                     `json:"source_revision"`
	SelectedCount      int                        `json:"selected_count"`
	SelectedBytes      int                        `json:"selected_bytes"`
	SelectedListSHA256 string                     `json:"selected_list_sha256"`
	ExistingManifest   *selectionExistingManifest `json:"existing_manifest,omitempty"`
	SelectedCandidates []roundTripCandidate       `json:"selected_candidates"`
}

type selectionExistingManifest struct {
	Path       string `json:"path"`
	SHA256     string `json:"sha256"`
	EntryCount int    `json:"entry_count"`
}

type roundTripCandidate struct {
	ManifestPath     string               `json:"manifest_path"`
	SourcePath       string               `json:"source_path"`
	SourceURI        string               `json:"source_uri"`
	File             roundTripFileRef     `json:"file"`
	PackageName      string               `json:"package"`
	UniqueConstructs []string             `json:"unique_constructs,omitempty"`
	Constructs       []roundTripConstruct `json:"constructs,omitempty"`
	ShapeSHA256      string               `json:"shape_sha256"`
	SelectionOrder   int                  `json:"selection_order"`
	SelectionScore   int                  `json:"selection_score"`
	Selected         bool                 `json:"selected"`
	SelectionReason  string               `json:"selection_reason"`
}

type roundTripFileRef struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int    `json:"bytes"`
}

type roundTripConstruct struct {
	Construct string `json:"construct"`
	Count     int    `json:"count"`
}

type corpusCheckReport struct {
	ArtifactKind            string                  `json:"artifact_kind"`
	Version                 int                     `json:"version"`
	Tool                    string                  `json:"tool"`
	Status                  string                  `json:"status"`
	SelectionReport         *roundTripReportFile    `json:"selection_report,omitempty"`
	SourceRoot              string                  `json:"source_root"`
	EntryCount              int                     `json:"entry_count"`
	UnsupportedConstructs   []roundTripConstructGap `json:"unsupported_constructs"`
	UnsupportedGrammarGaps  []roundTripGrammarGap   `json:"unsupported_grammar_gaps"`
	NextMissingConstruct    *roundTripConstructGap  `json:"next_missing_construct"`
	NextMissingGrammarGap   *roundTripGrammarGap    `json:"next_missing_grammar_gap"`
	Files                   []checkerFileSummary    `json:"files"`
	StablePromotionBlockers []string                `json:"stable_promotion_blockers"`
}

type roundTripReportFile struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int64  `json:"bytes"`
}

type roundTripConstructGap struct {
	Construct string   `json:"construct"`
	Count     int      `json:"count"`
	Files     []string `json:"files,omitempty"`
}

type roundTripGrammarGap struct {
	Category string   `json:"category"`
	Feature  string   `json:"feature"`
	Count    int      `json:"count"`
	Files    []string `json:"files,omitempty"`
}

type checkerFileSummary struct {
	Path                   string                `json:"path"`
	SHA256                 string                `json:"sha256"`
	Bytes                  int                   `json:"bytes"`
	PackageName            string                `json:"package"`
	Constructs             []string              `json:"constructs"`
	UnsupportedConstructs  []string              `json:"unsupported_constructs"`
	UnsupportedGrammarGaps []roundTripGrammarGap `json:"unsupported_grammar_gaps"`
}

type reportIngestionManifest struct {
	CorpusID string                  `json:"corpus_id"`
	Records  []reportIngestionRecord `json:"records"`
}

type reportIngestionRecord struct {
	Role                             string `json:"role,omitempty"`
	EvidenceScope                    string `json:"evidence_scope,omitempty"`
	ManifestPath                     string `json:"manifest_path"`
	SourcePath                       string `json:"source_path"`
	SourceSHA256                     string `json:"source_sha256,omitempty"`
	GeneratedCodecRoute              string `json:"generated_codec_route"`
	ReportPath                       string `json:"report_path"`
	GeneratedCodecReportArtifactKind string `json:"generated_codec_report_artifact_kind,omitempty"`
	ExpectedTCTTokenCount            int    `json:"expected_tct_token_count,omitempty"`
	ExpectedTCTTokensSHA256          string `json:"expected_tct_tokens_sha256,omitempty"`
}

type roundTripGateReport struct {
	ArtifactKind               string                    `json:"artifact_kind"`
	Version                    int                       `json:"version"`
	Tool                       string                    `json:"tool"`
	Status                     string                    `json:"status"`
	GoVersion                  string                    `json:"go_version"`
	SelectionReport            roundTripSelectionSummary `json:"selection_report"`
	CheckerReport              roundTripCheckerSummary   `json:"checker_report"`
	ReportIngestionManifest    roundTripManifestSummary  `json:"report_ingestion_manifest"`
	SelectedRecordCount        int                       `json:"selected_record_count"`
	CheckerReady               bool                      `json:"checker_ready"`
	SupportedRecordCount       int                       `json:"supported_record_count"`
	BlockedRecordCount         int                       `json:"blocked_record_count"`
	FirstGeneratedCodecBlocker *roundTripBlocker         `json:"first_generated_codec_blocker,omitempty"`
	Records                    []roundTripGateRecord     `json:"records"`
	NextConcreteTask           string                    `json:"next_concrete_task"`
	ValidationCommands         []string                  `json:"validation_commands"`
	ClaimBoundary              string                    `json:"claim_boundary"`
	NoStableClaims             []string                  `json:"no_stable_claims"`
}

type roundTripSelectionSummary struct {
	Path                 string `json:"path"`
	SHA256               string `json:"sha256"`
	ArtifactKind         string `json:"artifact_kind"`
	Version              int    `json:"version"`
	Tool                 string `json:"tool"`
	Status               string `json:"status"`
	SourceRoot           string `json:"source_root"`
	SourceRevision       string `json:"source_revision"`
	SelectedCount        int    `json:"selected_count"`
	SelectedBytes        int    `json:"selected_bytes"`
	SelectedListSHA256   string `json:"selected_list_sha256"`
	ExistingManifestPath string `json:"existing_manifest_path,omitempty"`
}

type roundTripCheckerSummary struct {
	Path                       string   `json:"path"`
	SHA256                     string   `json:"sha256"`
	ArtifactKind               string   `json:"artifact_kind"`
	Version                    int      `json:"version"`
	Tool                       string   `json:"tool"`
	Status                     string   `json:"status"`
	EntryCount                 int      `json:"entry_count"`
	UnsupportedConstructCount  int      `json:"unsupported_construct_count"`
	UnsupportedGrammarGapCount int      `json:"unsupported_grammar_gap_count"`
	StablePromotionBlockers    []string `json:"stable_promotion_blockers"`
	SelectionReportHashMatched bool     `json:"selection_report_hash_matched"`
}

type roundTripManifestSummary struct {
	Path        string `json:"path"`
	SHA256      string `json:"sha256"`
	CorpusID    string `json:"corpus_id"`
	RecordCount int    `json:"record_count"`
}

type roundTripGateRecord struct {
	ManifestPath         string             `json:"manifest_path"`
	SourcePath           string             `json:"source_path"`
	SourceSHA256         string             `json:"source_sha256"`
	Bytes                int                `json:"bytes"`
	PackageName          string             `json:"package"`
	SelectionOrder       int                `json:"selection_order"`
	CheckerStatus        string             `json:"checker_status"`
	GeneratedCodecStatus string             `json:"generated_codec_status"`
	GeneratedCodecRoute  string             `json:"generated_codec_route,omitempty"`
	ReportPath           string             `json:"report_path,omitempty"`
	Evidence             roundTripEvidence  `json:"evidence"`
	Blockers             []roundTripBlocker `json:"blockers,omitempty"`
}

type roundTripEvidence struct {
	ReportLoaded            bool   `json:"report_loaded"`
	ArtifactKind            string `json:"artifact_kind,omitempty"`
	EncodeDecodeRoundTrip   bool   `json:"encode_decode_round_trip"`
	RenderedCanonicalSource bool   `json:"rendered_canonical_source"`
	GofmtPassed             bool   `json:"gofmt_passed"`
	GoParserPassed          bool   `json:"go_parser_passed"`
	TokenCount              int    `json:"token_count,omitempty"`
	TokensSHA256            string `json:"tokens_sha256,omitempty"`
}

type roundTripBlocker struct {
	Kind   string `json:"kind"`
	Path   string `json:"path"`
	Reason string `json:"reason"`
}

type generatedCodecReport struct {
	ArtifactKind            string                `json:"artifact_kind"`
	Status                  string                `json:"status"`
	Route                   string                `json:"route"`
	SourcePath              string                `json:"source_path"`
	SourceSHA256            string                `json:"source_sha256"`
	Source                  *generatedCodecSource `json:"source,omitempty"`
	ParserValidation        map[string]string     `json:"parser_validation,omitempty"`
	CanonicalRender         map[string]any        `json:"canonical_render,omitempty"`
	TCTTokenCount           int                   `json:"tct_token_count"`
	RawTokenCount           int                   `json:"raw_token_count"`
	TCTTokensSHA256         string                `json:"tct_tokens_sha256"`
	RawTokenIDsSHA256       string                `json:"raw_token_ids_sha256"`
	TCTTokens               []int                 `json:"tct_tokens,omitempty"`
	RawTokensBelowVocabSize *bool                 `json:"raw_tokens_below_vocab_size,omitempty"`
	PadTokenAbsent          *bool                 `json:"pad_token_absent,omitempty"`
	Extra                   map[string]any        `json:"-"`
}

type generatedCodecSource struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int    `json:"bytes"`
}

func main() {
	if err := runRoundTripGate(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func runRoundTripGate(args []string) error {
	options := roundTripGateOptions{}
	flags := flag.NewFlagSet(roundTripGateTool, flag.ContinueOnError)
	flags.StringVar(&options.selectionReportPath, "selection-report", "", "checker-ready Go source-selection report JSON")
	flags.StringVar(&options.checkerReportPath, "checker-report", "", "matching Go syntax corpus checker report JSON")
	flags.StringVar(&options.reportIngestionManifest, "report-ingestion-manifest", "tests/fixtures/go_syntax_corpus/training-report-ingestion-current-plus-v1.json", "generated-codec report-ingestion manifest to search for retained evidence")
	flags.StringVar(&options.outputPath, "output", "", "write generated-codec round-trip gate report JSON")
	flags.StringVar(&options.validateReportPath, "validate-report", "", "validate an existing generated-codec round-trip gate report against current inputs")
	if err := flags.Parse(args); err != nil {
		return err
	}
	if options.selectionReportPath == "" {
		return errors.New("--selection-report is required")
	}
	if options.checkerReportPath == "" {
		return errors.New("--checker-report is required")
	}
	if options.outputPath == "" && options.validateReportPath == "" {
		return errors.New("one of --output or --validate-report is required")
	}

	expected, err := buildRoundTripGateReport(options)
	if err != nil {
		return err
	}
	if options.outputPath != "" {
		if err := writeRoundTripJSON(options.outputPath, expected); err != nil {
			return err
		}
	}
	if options.validateReportPath != "" {
		actual, err := readRoundTripJSON[roundTripGateReport](options.validateReportPath)
		if err != nil {
			return err
		}
		if !reflect.DeepEqual(actual, expected) {
			return errors.New("round-trip gate report does not match current inputs")
		}
	}
	return nil
}

func buildRoundTripGateReport(options roundTripGateOptions) (roundTripGateReport, error) {
	selection, err := readRoundTripJSON[corpusSelectionReport](options.selectionReportPath)
	if err != nil {
		return roundTripGateReport{}, fmt.Errorf("read selection report: %w", err)
	}
	checker, err := readRoundTripJSON[corpusCheckReport](options.checkerReportPath)
	if err != nil {
		return roundTripGateReport{}, fmt.Errorf("read checker report: %w", err)
	}
	ingestion, err := readRoundTripJSON[reportIngestionManifest](options.reportIngestionManifestPath())
	if err != nil {
		return roundTripGateReport{}, fmt.Errorf("read report-ingestion manifest: %w", err)
	}
	if err := validateSelectionAndChecker(options, selection, checker); err != nil {
		return roundTripGateReport{}, err
	}

	records := make([]roundTripGateRecord, 0, len(selection.SelectedCandidates))
	supported := 0
	blocked := 0
	var firstBlocker *roundTripBlocker
	for _, candidate := range selectedCandidatesInOrder(selection.SelectedCandidates) {
		record := buildGateRecord(candidate, checker, ingestion, options.reportIngestionManifestPath())
		if record.GeneratedCodecStatus == roundTripGateStatusPassed {
			supported++
		} else {
			blocked++
			if firstBlocker == nil && len(record.Blockers) > 0 {
				blocker := record.Blockers[0]
				firstBlocker = &blocker
			}
		}
		records = append(records, record)
	}

	status := roundTripGateStatusPassed
	nextTask := "broaden the checker-ready Go generated-codec gate to the next corpus slice"
	if blocked > 0 {
		status = roundTripGateStatusBlocked
		nextTask = "make the first checker-ready selected Go source produce reusable parser-backed metadata, source-instance projection, and generated-codec report evidence without adding a report-specific fixture ladder"
	}
	return roundTripGateReport{
		ArtifactKind:               roundTripGateArtifactKind,
		Version:                    roundTripGateVersion,
		Tool:                       roundTripGateTool,
		Status:                     status,
		GoVersion:                  runtime.Version(),
		SelectionReport:            selectionSummary(options.selectionReportPath, selection),
		CheckerReport:              checkerSummary(options, checker),
		ReportIngestionManifest:    manifestSummary(options.reportIngestionManifestPath(), ingestion),
		SelectedRecordCount:        len(records),
		CheckerReady:               true,
		SupportedRecordCount:       supported,
		BlockedRecordCount:         blocked,
		FirstGeneratedCodecBlocker: firstBlocker,
		Records:                    records,
		NextConcreteTask:           nextTask,
		ValidationCommands:         validationCommands(options),
		ClaimBoundary:              "checker-ready Go source selection to generated-codec round-trip gate; pass records require encode/decode token evidence plus render, gofmt, and go/parser sidecar validation. Blocked records are precise generated-codec intake blockers, not Arena IR, route-handoff, BPE, paper, type-correctness, generated-hot-path, or stable-backend evidence.",
		NoStableClaims:             append([]string(nil), roundTripNoStableClaims...),
	}, nil
}

func (options roundTripGateOptions) reportIngestionManifestPath() string {
	return options.reportIngestionManifest
}

func validateSelectionAndChecker(options roundTripGateOptions, selection corpusSelectionReport, checker corpusCheckReport) error {
	if selection.ArtifactKind != selectionArtifactKind {
		return fmt.Errorf("selection artifact_kind must be %q, got %q", selectionArtifactKind, selection.ArtifactKind)
	}
	if selection.Status != "ready" {
		return fmt.Errorf("selection report must be ready, got %q", selection.Status)
	}
	if selection.SelectedCount <= 0 || len(selection.SelectedCandidates) == 0 {
		return errors.New("selection report must contain selected candidates")
	}
	if checker.ArtifactKind != checkArtifactKind {
		return fmt.Errorf("checker artifact_kind must be %q, got %q", checkArtifactKind, checker.ArtifactKind)
	}
	if checker.Status != "ready" {
		return fmt.Errorf("checker report must be ready, got %q", checker.Status)
	}
	if len(checker.UnsupportedConstructs) > 0 || len(checker.UnsupportedGrammarGaps) > 0 || checker.NextMissingConstruct != nil || checker.NextMissingGrammarGap != nil {
		return errors.New("checker report must have no unsupported constructs or grammar gaps")
	}
	if checker.EntryCount != selection.SelectedCount {
		return fmt.Errorf("checker entry_count %d must match selection selected_count %d", checker.EntryCount, selection.SelectedCount)
	}
	if checker.SelectionReport == nil {
		return errors.New("checker report must record the matching selection_report file")
	}
	selectionSHA, err := fileSHA256(options.selectionReportPath)
	if err != nil {
		return err
	}
	if checker.SelectionReport.SHA256 != selectionSHA {
		return fmt.Errorf("checker selection_report sha256 %s does not match %s", checker.SelectionReport.SHA256, selectionSHA)
	}
	checkerBySHA := map[string]checkerFileSummary{}
	for _, file := range checker.Files {
		if len(file.UnsupportedConstructs) > 0 || len(file.UnsupportedGrammarGaps) > 0 {
			return fmt.Errorf("checker file %s is not ready", file.Path)
		}
		checkerBySHA[file.SHA256] = file
	}
	for _, candidate := range selection.SelectedCandidates {
		if !candidate.Selected {
			continue
		}
		if _, ok := checkerBySHA[candidate.File.SHA256]; !ok {
			return fmt.Errorf("selected candidate %s sha256 %s missing from checker files", candidate.ManifestPath, candidate.File.SHA256)
		}
	}
	return nil
}

func buildGateRecord(candidate roundTripCandidate, checker corpusCheckReport, ingestion reportIngestionManifest, ingestionPath string) roundTripGateRecord {
	record := roundTripGateRecord{
		ManifestPath:         candidate.ManifestPath,
		SourcePath:           normalizedCandidateSourcePath(candidate),
		SourceSHA256:         candidate.File.SHA256,
		Bytes:                candidate.File.Bytes,
		PackageName:          candidate.PackageName,
		SelectionOrder:       candidate.SelectionOrder,
		CheckerStatus:        checkerStatusFor(candidate, checker),
		GeneratedCodecStatus: roundTripGateStatusBlocked,
	}
	ingestionRecord, found := findIngestionRecord(ingestion, candidate)
	if !found {
		record.Blockers = []roundTripBlocker{{
			Kind: "generated_codec_report_ingestion_record_missing",
			Path: ingestionPath,
			Reason: fmt.Sprintf(
				"checker-ready selected source %q has no generated-codec report-ingestion record; current generated-codec round-trip evidence still requires reusable parser-backed metadata, source-instance projection, and generated report production before this source can pass the gate",
				candidate.ManifestPath,
			),
		}}
		return record
	}
	record.GeneratedCodecRoute = ingestionRecord.GeneratedCodecRoute
	record.ReportPath = ingestionRecord.ReportPath
	report, err := readRoundTripJSON[generatedCodecReport](ingestionRecord.ReportPath)
	if err != nil {
		record.Blockers = []roundTripBlocker{{
			Kind:   "generated_codec_report_missing",
			Path:   ingestionRecord.ReportPath,
			Reason: fmt.Sprintf("generated-codec report for checker-ready source %q could not be read: %v", candidate.ManifestPath, err),
		}}
		return record
	}
	evidence, blockers := generatedCodecEvidence(candidate, ingestionRecord, report)
	record.Evidence = evidence
	if len(blockers) > 0 {
		record.Blockers = blockers
		return record
	}
	record.GeneratedCodecStatus = roundTripGateStatusPassed
	return record
}

func generatedCodecEvidence(candidate roundTripCandidate, ingestion reportIngestionRecord, report generatedCodecReport) (roundTripEvidence, []roundTripBlocker) {
	sourcePath, sourceSHA := generatedReportSource(report)
	tokenCount := report.TCTTokenCount
	if tokenCount == 0 {
		tokenCount = report.RawTokenCount
	}
	tokenHash := report.TCTTokensSHA256
	if tokenHash == "" {
		tokenHash = report.RawTokenIDsSHA256
	}
	evidence := roundTripEvidence{
		ReportLoaded:            true,
		ArtifactKind:            report.ArtifactKind,
		EncodeDecodeRoundTrip:   generatedReportReady(report.Status) && tokenCount > 0 && (len(report.TCTTokens) > 0 || tokenHash != ""),
		RenderedCanonicalSource: canonicalRenderReady(report),
		GofmtPassed:             gofmtPassed(report),
		GoParserPassed:          goParserPassed(report),
		TokenCount:              tokenCount,
		TokensSHA256:            tokenHash,
	}
	blockers := make([]roundTripBlocker, 0)
	if report.ArtifactKind == "" {
		blockers = append(blockers, roundTripBlocker{Kind: "generated_codec_report_artifact_kind_missing", Path: ingestion.ReportPath, Reason: "generated-codec report artifact_kind is empty"})
	}
	if ingestion.GeneratedCodecReportArtifactKind != "" && report.ArtifactKind != ingestion.GeneratedCodecReportArtifactKind {
		blockers = append(blockers, roundTripBlocker{Kind: "generated_codec_report_artifact_kind_mismatch", Path: ingestion.ReportPath, Reason: fmt.Sprintf("report artifact_kind %q does not match ingestion artifact kind %q", report.ArtifactKind, ingestion.GeneratedCodecReportArtifactKind)})
	}
	if !generatedReportReady(report.Status) {
		blockers = append(blockers, roundTripBlocker{Kind: "generated_codec_report_not_ready", Path: ingestion.ReportPath, Reason: fmt.Sprintf("generated-codec report status is %q", report.Status)})
	}
	if report.Route != ingestion.GeneratedCodecRoute {
		blockers = append(blockers, roundTripBlocker{Kind: "generated_codec_route_mismatch", Path: ingestion.ReportPath, Reason: fmt.Sprintf("report route %q does not match ingestion route %q", report.Route, ingestion.GeneratedCodecRoute)})
	}
	if sourceSHA != candidate.File.SHA256 {
		blockers = append(blockers, roundTripBlocker{Kind: "generated_codec_source_sha_mismatch", Path: ingestion.ReportPath, Reason: fmt.Sprintf("report source sha256 %q does not match selected source %q", sourceSHA, candidate.File.SHA256)})
	}
	if sourcePath != "" && !sameSourcePath(sourcePath, candidate) {
		blockers = append(blockers, roundTripBlocker{Kind: "generated_codec_source_path_mismatch", Path: ingestion.ReportPath, Reason: fmt.Sprintf("report source path %q does not match selected source %q", sourcePath, normalizedCandidateSourcePath(candidate))})
	}
	if !evidence.EncodeDecodeRoundTrip {
		blockers = append(blockers, roundTripBlocker{Kind: "generated_codec_encode_decode_missing", Path: ingestion.ReportPath, Reason: "generated-codec report lacks non-empty token evidence for encode/decode round trip"})
	}
	if !evidence.RenderedCanonicalSource || !evidence.GofmtPassed || !evidence.GoParserPassed {
		blockers = append(blockers, roundTripBlocker{Kind: "generated_codec_render_reparse_missing", Path: ingestion.ReportPath, Reason: "generated-codec report lacks passed canonical render, gofmt, or go/parser evidence"})
	}
	return evidence, blockers
}

func generatedReportReady(status string) bool {
	return status == "ready" || status == sourceInstanceReportReadyStatus
}

func generatedReportSource(report generatedCodecReport) (string, string) {
	if report.Source != nil {
		return report.Source.Path, report.Source.SHA256
	}
	return report.SourcePath, report.SourceSHA256
}

func canonicalRenderReady(report generatedCodecReport) bool {
	if value, ok := report.CanonicalRender["matches_source"].(bool); ok && !value {
		return false
	}
	return gofmtPassed(report) && goParserPassed(report)
}

func gofmtPassed(report generatedCodecReport) bool {
	if report.ParserValidation["gofmt"] == "passed" {
		return true
	}
	if value, ok := report.CanonicalRender["gofmt"].(string); ok {
		return value == "passed"
	}
	return false
}

func goParserPassed(report generatedCodecReport) bool {
	if report.ParserValidation["go_parser"] == "passed" {
		return true
	}
	if value, ok := report.CanonicalRender["go_parser"].(string); ok {
		return value == "passed"
	}
	return false
}

func findIngestionRecord(ingestion reportIngestionManifest, candidate roundTripCandidate) (reportIngestionRecord, bool) {
	for _, record := range ingestion.Records {
		if record.ManifestPath == candidate.ManifestPath {
			return record, true
		}
		if record.SourceSHA256 != "" && record.SourceSHA256 == candidate.File.SHA256 {
			return record, true
		}
	}
	return reportIngestionRecord{}, false
}

func checkerStatusFor(candidate roundTripCandidate, checker corpusCheckReport) string {
	for _, file := range checker.Files {
		if file.SHA256 == candidate.File.SHA256 {
			if len(file.UnsupportedConstructs) == 0 && len(file.UnsupportedGrammarGaps) == 0 {
				return "ready"
			}
			return "blocked"
		}
	}
	return "missing"
}

func selectedCandidatesInOrder(candidates []roundTripCandidate) []roundTripCandidate {
	selected := make([]roundTripCandidate, 0, len(candidates))
	for _, candidate := range candidates {
		if candidate.Selected {
			selected = append(selected, candidate)
		}
	}
	sort.SliceStable(selected, func(left, right int) bool {
		return selected[left].SelectionOrder < selected[right].SelectionOrder
	})
	return selected
}

func selectionSummary(path string, selection corpusSelectionReport) roundTripSelectionSummary {
	existingPath := ""
	if selection.ExistingManifest != nil {
		existingPath = selection.ExistingManifest.Path
	}
	return roundTripSelectionSummary{
		Path:                 path,
		SHA256:               mustFileSHA256(path),
		ArtifactKind:         selection.ArtifactKind,
		Version:              selection.Version,
		Tool:                 selection.Tool,
		Status:               selection.Status,
		SourceRoot:           filepath.ToSlash(selection.SourceRoot),
		SourceRevision:       selection.SourceRevision,
		SelectedCount:        selection.SelectedCount,
		SelectedBytes:        selection.SelectedBytes,
		SelectedListSHA256:   selection.SelectedListSHA256,
		ExistingManifestPath: existingPath,
	}
}

func checkerSummary(options roundTripGateOptions, checker corpusCheckReport) roundTripCheckerSummary {
	selectionSHA := mustFileSHA256(options.selectionReportPath)
	hashMatched := checker.SelectionReport != nil && checker.SelectionReport.SHA256 == selectionSHA
	return roundTripCheckerSummary{
		Path:                       options.checkerReportPath,
		SHA256:                     mustFileSHA256(options.checkerReportPath),
		ArtifactKind:               checker.ArtifactKind,
		Version:                    checker.Version,
		Tool:                       checker.Tool,
		Status:                     checker.Status,
		EntryCount:                 checker.EntryCount,
		UnsupportedConstructCount:  len(checker.UnsupportedConstructs),
		UnsupportedGrammarGapCount: len(checker.UnsupportedGrammarGaps),
		StablePromotionBlockers:    append([]string(nil), checker.StablePromotionBlockers...),
		SelectionReportHashMatched: hashMatched,
	}
}

func manifestSummary(path string, manifest reportIngestionManifest) roundTripManifestSummary {
	return roundTripManifestSummary{
		Path:        path,
		SHA256:      mustFileSHA256(path),
		CorpusID:    manifest.CorpusID,
		RecordCount: len(manifest.Records),
	}
}

func validationCommands(options roundTripGateOptions) []string {
	return []string{
		"go test scripts/analysis/go_syntax_generated_codec_round_trip_gate.go scripts/analysis/go_syntax_generated_codec_round_trip_gate_test.go",
		fmt.Sprintf("go run scripts/analysis/go_syntax_generated_codec_round_trip_gate.go --selection-report %s --checker-report %s --report-ingestion-manifest %s --output target/tct-analysis/go-generated-codec-round-trip-gate/current-plus-report.json", options.selectionReportPath, options.checkerReportPath, options.reportIngestionManifestPath()),
		fmt.Sprintf("go run scripts/analysis/go_syntax_generated_codec_round_trip_gate.go --selection-report %s --checker-report %s --report-ingestion-manifest %s --validate-report target/tct-analysis/go-generated-codec-round-trip-gate/current-plus-report.json", options.selectionReportPath, options.checkerReportPath, options.reportIngestionManifestPath()),
	}
}

func normalizedCandidateSourcePath(candidate roundTripCandidate) string {
	if candidate.SourcePath != "" {
		return filepath.ToSlash(candidate.SourcePath)
	}
	if candidate.File.Path != "" {
		return filepath.ToSlash(candidate.File.Path)
	}
	return filepath.ToSlash(candidate.ManifestPath)
}

func sameSourcePath(reportPath string, candidate roundTripCandidate) bool {
	reportPath = filepath.ToSlash(reportPath)
	candidatePath := normalizedCandidateSourcePath(candidate)
	if reportPath == candidatePath {
		return true
	}
	return strings.HasSuffix(reportPath, "/"+candidate.ManifestPath) || strings.HasSuffix(candidatePath, "/"+candidate.ManifestPath)
}

func readRoundTripJSON[T any](path string) (T, error) {
	var value T
	data, err := os.ReadFile(path)
	if err != nil {
		return value, err
	}
	if err := json.Unmarshal(data, &value); err != nil {
		return value, err
	}
	return value, nil
}

func writeRoundTripJSON(path string, value any) error {
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

func fileSHA256(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	sum := sha256.Sum256(data)
	return hex.EncodeToString(sum[:]), nil
}

func mustFileSHA256(path string) string {
	sum, err := fileSHA256(path)
	if err != nil {
		return ""
	}
	return sum
}
