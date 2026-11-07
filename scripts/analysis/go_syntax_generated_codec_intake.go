// go_syntax_generated_codec_intake records whether a harness-selected Go
// source file can enter the current generated-codec training corpus path.
package main

import (
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"io/fs"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strings"
)

const (
	intakeTool          = "go_syntax_generated_codec_intake"
	intakeVersion       = 1
	intakeArtifactKind  = "tct_go_syntax_generated_codec_intake_blocker_v1"
	intakeStatusReady   = "ready"
	intakeStatusBlocked = "blocked"

	goAstSchemaInventoryDefaultPath  = "tests/fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json"
	goAstSchemaInventoryArtifactKind = "tct_go_ast_schema_inventory_v1"
	goAstSchemaInventoryVersion      = 1

	goAstRawTreeRendererCodegenCoverageStatusReady = "go_ast_raw_tree_renderer_codegen_coverage_ready"
	goAstRawTreeRendererCodegenCoverageNextTask    = "wire BranchStmt statement rendering through the inventory-wide generated Go raw-tree renderer wrapper before selecting more source breadth"
	goRawTreeRendererDispatchAuditGate             = "cargo test --test go_ast_schema_inventory_test checked_go_ast_schema_inventory_classifies_raw_tree_renderer_codegen_coverage -- --exact"
)

var requiredClaimBoundaries = []string{
	"no_broad_corpus_claim",
	"no_source_validity_by_construction_claim",
	"no_type_correctness_claim",
	"no_model_quality_claim",
	"no_paper_claim",
	"no_generated_hot_path_claim",
	"no_stable_backend_claim",
	"no_arena_ir_promotion",
}

type selectionReport struct {
	ArtifactKind       string              `json:"artifact_kind"`
	Version            int                 `json:"version"`
	Tool               string              `json:"tool"`
	Status             string              `json:"status"`
	SourceRoot         string              `json:"source_root"`
	SourceRevision     string              `json:"source_revision"`
	SourceURIPrefix    string              `json:"source_uri_prefix"`
	License            string              `json:"license"`
	LicenseTextPath    string              `json:"license_text_path"`
	SelectedCount      int                 `json:"selected_count"`
	SelectedBytes      int                 `json:"selected_bytes"`
	SelectedListSHA256 string              `json:"selected_list_sha256"`
	SelectedCandidates []goSourceCandidate `json:"selected_candidates"`
}

type goSourceCandidate struct {
	ManifestPath     string           `json:"manifest_path"`
	SourcePath       string           `json:"source_path"`
	SourceURI        string           `json:"source_uri"`
	File             sourceFileRef    `json:"file"`
	PackageName      string           `json:"package"`
	Imports          []string         `json:"imports,omitempty"`
	TopLevelDecls    int              `json:"top_level_decls"`
	FunctionDecls    int              `json:"function_decls"`
	MethodDecls      int              `json:"method_decls"`
	CommentGroups    int              `json:"comment_groups"`
	UniqueConstructs []string         `json:"unique_constructs"`
	Constructs       []constructCount `json:"constructs,omitempty"`
	ShapeSHA256      string           `json:"shape_sha256"`
	SelectionScore   int              `json:"selection_score"`
	SelectionOrder   int              `json:"selection_order"`
	Selected         bool             `json:"selected"`
	SelectionReason  string           `json:"selection_reason"`
}

type sourceFileRef struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int    `json:"bytes"`
}

type constructCount struct {
	Construct string `json:"construct"`
	Count     int    `json:"count"`
}

type corpusManifest struct {
	CorpusID string                `json:"corpus_id"`
	Entries  []corpusManifestEntry `json:"entries"`
}

type corpusManifestEntry struct {
	ManifestPath string `json:"manifest_path"`
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
	ExpectedMaxRawTokenID            int    `json:"expected_max_raw_token_id,omitempty"`
	ExpectedVocabSize                int    `json:"expected_vocab_size,omitempty"`
	ExpectedPadToken                 int    `json:"expected_pad_token,omitempty"`
	ExpectedPadTokenAbsent           *bool  `json:"expected_pad_token_absent,omitempty"`
	GoParserVersion                  string `json:"go_parser_version,omitempty"`
}

type sourceRouteSupportRecord struct {
	Found          bool   `json:"found"`
	Source         string `json:"source,omitempty"`
	ManifestPath   string `json:"manifest_path,omitempty"`
	GeneratedRoute string `json:"generated_codec_route,omitempty"`
	Evidence       string `json:"evidence,omitempty"`
}

type goAstSchemaInventoryFile struct {
	ArtifactKind string                     `json:"artifact_kind"`
	Version      int                        `json:"version"`
	Go           goAstSchemaInventoryGo     `json:"go"`
	NodeStructs  []goAstSchemaInventoryNode `json:"node_structs"`
}

type goAstSchemaInventoryGo struct {
	Version string `json:"version"`
}

type goAstSchemaInventoryNode struct {
	Name        string `json:"name"`
	Tokenizable bool   `json:"tokenizable"`
}

type rendererCodegenCoverage struct {
	InventoryPath                   string
	ArtifactKind                    string
	Version                         int
	GoVersion                       string
	Status                          string
	NextReadyTask                   string
	TokenizableNodeCount            int
	GeneratedSkeletonReadyNodeKinds map[string]bool
	SourceFileRootExcludedNodeKinds []string
}

type syntaxMetadataFile struct {
	Source struct {
		Path string `json:"path"`
	} `json:"source"`
	Rules []syntaxMetadataRule `json:"rules,omitempty"`
}

type syntaxMetadataRule struct {
	Name     string `json:"name"`
	NodeKind string `json:"node_kind"`
}

type intakeReport struct {
	ArtifactKind              string                `json:"artifact_kind"`
	Version                   int                   `json:"version"`
	Tool                      string                `json:"tool"`
	Status                    string                `json:"status"`
	GoVersion                 string                `json:"go_version"`
	SelectionReport           intakeSelectionReport `json:"selection_report"`
	SelectionPolicy           selectionPolicy       `json:"selection_policy"`
	SelectedCandidate         intakeCandidate       `json:"selected_candidate"`
	CheckedSourceTargetPath   string                `json:"checked_source_target_path"`
	CurrentEvidence           currentEvidence       `json:"current_evidence"`
	Blockers                  []intakeBlocker       `json:"blockers"`
	ClaimBoundaries           []string              `json:"claim_boundaries"`
	NextConcreteTask          string                `json:"next_concrete_task"`
	ValidationCommandTemplate []string              `json:"validation_command_template"`
}

type intakeSelectionReport struct {
	Path               string `json:"path"`
	ArtifactKind       string `json:"artifact_kind"`
	Version            int    `json:"version"`
	Tool               string `json:"tool"`
	Status             string `json:"status"`
	SourceRoot         string `json:"source_root"`
	SourceRevision     string `json:"source_revision"`
	SelectedCount      int    `json:"selected_count"`
	SelectedBytes      int    `json:"selected_bytes"`
	SelectedListSHA256 string `json:"selected_list_sha256"`
}

type selectionPolicy struct {
	PreferredManifestPath string `json:"preferred_manifest_path"`
	PreferredSelected     bool   `json:"preferred_selected"`
	SelectedBy            string `json:"selected_by"`
}

type intakeCandidate struct {
	ManifestPath     string   `json:"manifest_path"`
	SourcePath       string   `json:"source_path"`
	SourceURI        string   `json:"source_uri"`
	PackageName      string   `json:"package"`
	SHA256           string   `json:"sha256"`
	Bytes            int      `json:"bytes"`
	SelectionOrder   int      `json:"selection_order"`
	SelectionScore   int      `json:"selection_score"`
	SelectionReason  string   `json:"selection_reason"`
	UniqueConstructs []string `json:"unique_constructs"`
	ShapeSHA256      string   `json:"shape_sha256"`
}

type currentEvidence struct {
	ExistingTrainingManifestPath          string                   `json:"existing_training_manifest_path"`
	ExistingTrainingCorpusID              string                   `json:"existing_training_corpus_id"`
	AlreadyInExistingManifest             bool                     `json:"already_in_existing_manifest"`
	MetadataRoot                          string                   `json:"metadata_root"`
	CheckedMetadataFixtureFound           bool                     `json:"checked_metadata_fixture_found"`
	CheckedMetadataFixturePath            string                   `json:"checked_metadata_fixture_path,omitempty"`
	ReportIngestionManifestPath           string                   `json:"report_ingestion_manifest_path"`
	ReportIngestionCorpusID               string                   `json:"report_ingestion_corpus_id"`
	GeneratedCodecReportFound             bool                     `json:"generated_codec_report_found"`
	GeneratedCodecRoute                   string                   `json:"generated_codec_route,omitempty"`
	GeneratedCodecReportPath              string                   `json:"generated_codec_report_path,omitempty"`
	GeneratedCodecReportArtifactKind      string                   `json:"generated_codec_report_artifact_kind,omitempty"`
	GeneratedCodecSourceSHA256            string                   `json:"generated_codec_source_sha256,omitempty"`
	GeneratedCodecExpectedTCTTokenCount   int                      `json:"generated_codec_expected_tct_token_count,omitempty"`
	GeneratedCodecExpectedTCTTokensSHA256 string                   `json:"generated_codec_expected_tct_tokens_sha256,omitempty"`
	GeneratedCodecExpectedMaxRawTokenID   int                      `json:"generated_codec_expected_max_raw_token_id,omitempty"`
	GeneratedCodecExpectedVocabSize       int                      `json:"generated_codec_expected_vocab_size,omitempty"`
	GeneratedCodecExpectedPadToken        int                      `json:"generated_codec_expected_pad_token,omitempty"`
	GeneratedCodecExpectedPadTokenAbsent  *bool                    `json:"generated_codec_expected_pad_token_absent,omitempty"`
	GeneratedCodecGoParserVersion         string                   `json:"generated_codec_go_parser_version,omitempty"`
	GeneratedCodecSourceRoute             sourceRouteSupportRecord `json:"generated_codec_source_route_support"`
	GeneratedCodecRendererBoundary        string                   `json:"generated_codec_renderer_boundary"`
	GeneratedCodecReportBoundary          string                   `json:"generated_codec_report_boundary"`
	ReusableIntakeRequirements            []string                 `json:"reusable_intake_requirements"`
	WrongDirectionAvoided                 string                   `json:"wrong_direction_avoided"`
	SyntaxConstructCoverage               syntaxConstructCoverage  `json:"syntax_construct_coverage"`
	GeneratedCodecRendererGap             rendererGapAnalysis      `json:"generated_codec_renderer_gap_analysis"`
}

type syntaxConstructCoverage struct {
	CheckedMetadataFixtureCount int                          `json:"checked_metadata_fixture_count"`
	SupportedConstructCount     int                          `json:"supported_construct_count"`
	UnsupportedConstructCount   int                          `json:"unsupported_construct_count"`
	SourceSpanStatus            string                       `json:"source_span_status"`
	UnsupportedConstructs       []unsupportedSyntaxConstruct `json:"unsupported_constructs,omitempty"`
}

type unsupportedSyntaxConstruct struct {
	Construct   string `json:"construct"`
	Count       int    `json:"count"`
	FirstLine   int    `json:"first_line,omitempty"`
	FirstColumn int    `json:"first_column,omitempty"`
	FirstNode   string `json:"first_node,omitempty"`
}

type rendererGapAnalysis struct {
	Analyzed                               bool     `json:"analyzed"`
	CoverageBasis                          string   `json:"coverage_basis"`
	ExistingReportIngestionRecordCount     int      `json:"existing_report_ingestion_record_count"`
	ExistingGeneratedCodecMetadataCount    int      `json:"existing_generated_codec_metadata_fixture_count"`
	RendererReadinessSource                string   `json:"renderer_readiness_source"`
	RendererCodegenCoverageInventoryPath   string   `json:"renderer_codegen_coverage_inventory_path,omitempty"`
	RendererCodegenCoverageArtifactKind    string   `json:"renderer_codegen_coverage_artifact_kind,omitempty"`
	RendererCodegenCoverageVersion         int      `json:"renderer_codegen_coverage_version,omitempty"`
	RendererCodegenCoverageGoVersion       string   `json:"renderer_codegen_coverage_go_version,omitempty"`
	RendererCodegenCoverageStatus          string   `json:"renderer_codegen_coverage_status,omitempty"`
	RendererCodegenCoverageNextReadyTask   string   `json:"renderer_codegen_coverage_next_ready_task,omitempty"`
	RendererCodegenCoverageReadyNodeCount  int      `json:"renderer_codegen_coverage_generated_skeleton_ready_node_count,omitempty"`
	RendererCodegenSourceFileRootExcluded  []string `json:"renderer_codegen_source_file_root_excluded_node_kinds,omitempty"`
	RawTreeRendererDispatchAuditGate       string   `json:"raw_tree_renderer_dispatch_audit_gate,omitempty"`
	SelectedMetadataRuleCount              int      `json:"selected_metadata_rule_count"`
	CoveredSelectedMetadataRuleCount       int      `json:"covered_selected_metadata_rule_count"`
	MissingGeneratedCodecRendererRuleCount int      `json:"missing_generated_codec_renderer_rule_count"`
	MissingGeneratedCodecRendererRules     []string `json:"missing_generated_codec_renderer_rules,omitempty"`
}

type intakeBlocker struct {
	Kind   string `json:"kind"`
	Path   string `json:"path"`
	Reason string `json:"reason"`
}

type intakeOptions struct {
	selectionReportPath           string
	preferredManifestPath         string
	selectionExistingManifestPath string
	existingManifestPath          string
	metadataRoot                  string
	reportIngestionManifest       string
	rendererCodegenInventoryPath  string
	outputPath                    string
	validateReportPath            string
}

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string) error {
	var options intakeOptions
	flags := flag.NewFlagSet("go_syntax_generated_codec_intake", flag.ContinueOnError)
	flags.StringVar(&options.selectionReportPath, "selection-report", "", "Go source-selection report JSON")
	flags.StringVar(&options.preferredManifestPath, "preferred-manifest-path", "", "preferred selected Go source manifest path")
	flags.StringVar(&options.selectionExistingManifestPath, "selection-existing-manifest", "", "existing corpus manifest used to produce the selection report; defaults to --existing-manifest")
	flags.StringVar(&options.existingManifestPath, "existing-manifest", "tests/fixtures/go_syntax_corpus/training-manifest-eight-record-v1.json", "checked Go training corpus manifest")
	flags.StringVar(&options.metadataRoot, "metadata-root", "tests/fixtures/go_syntax_metadata", "checked Go syntax metadata fixture root")
	flags.StringVar(&options.reportIngestionManifest, "report-ingestion-manifest", "tests/fixtures/go_syntax_corpus/training-report-ingestion-eight-record-v1.json", "checked Go training report-ingestion manifest")
	flags.StringVar(&options.rendererCodegenInventoryPath, "renderer-codegen-inventory", goAstSchemaInventoryDefaultPath, "checked go/ast inventory fixture whose renderer codegen coverage and dispatch audit gate define raw-tree renderer rule readiness")
	flags.StringVar(&options.outputPath, "output", "", "write generated intake report JSON")
	flags.StringVar(&options.validateReportPath, "validate-report", "", "validate a checked intake report JSON")
	if err := flags.Parse(args); err != nil {
		return err
	}
	if options.selectionReportPath == "" {
		return errors.New("--selection-report is required")
	}
	if options.preferredManifestPath == "" {
		return errors.New("--preferred-manifest-path is required")
	}
	if options.outputPath == "" && options.validateReportPath == "" {
		return errors.New("one of --output or --validate-report is required")
	}

	expected, err := buildIntakeReport(options)
	if err != nil {
		return err
	}
	if options.outputPath != "" {
		if err := writeJSON(options.outputPath, expected); err != nil {
			return err
		}
	}
	if options.validateReportPath != "" {
		actual, err := readJSON[intakeReport](options.validateReportPath)
		if err != nil {
			return err
		}
		if err := validateIntakeReport(actual, expected); err != nil {
			return err
		}
	}
	return nil
}

func buildIntakeReport(options intakeOptions) (intakeReport, error) {
	options = normalizeIntakeOptions(options)
	selection, err := readJSON[selectionReport](options.selectionReportPath)
	if err != nil {
		return intakeReport{}, fmt.Errorf("read selection report: %w", err)
	}
	candidate, selectedBy, preferredSelected, err := chooseCandidate(selection.SelectedCandidates, options.preferredManifestPath)
	if err != nil {
		return intakeReport{}, err
	}
	existingManifest, err := readJSON[corpusManifest](options.existingManifestPath)
	if err != nil {
		return intakeReport{}, fmt.Errorf("read existing manifest: %w", err)
	}
	reportIngestion, err := readJSON[reportIngestionManifest](options.reportIngestionManifest)
	if err != nil {
		return intakeReport{}, fmt.Errorf("read report-ingestion manifest: %w", err)
	}

	checkedSourcePath := filepath.ToSlash(filepath.Join(
		"tests/fixtures/go_syntax_corpus/src",
		filepath.FromSlash(candidate.ManifestPath),
	))
	metadataPath, metadataFound, err := findMetadataFixture(options.metadataRoot, checkedSourcePath)
	if err != nil {
		return intakeReport{}, err
	}
	reportRecord, reportFound := findReportIngestionRecord(reportIngestion, candidate.ManifestPath, checkedSourcePath)
	alreadyExisting := manifestContains(existingManifest, candidate.ManifestPath)
	constructCoverage, err := buildSyntaxConstructCoverage(options.metadataRoot, candidate)
	if err != nil {
		return intakeReport{}, err
	}
	rendererGaps, err := buildRendererGapAnalysis(
		options.metadataRoot,
		metadataPath,
		metadataFound,
		reportIngestion,
		options.rendererCodegenInventoryPath,
	)
	if err != nil {
		return intakeReport{}, err
	}
	sourceRouteSupport := sourceRouteSupportRecord{Found: false}
	if reportFound {
		sourceRouteSupport = sourceRouteSupportRecord{
			Found:          true,
			Source:         "report_ingestion_manifest",
			ManifestPath:   options.reportIngestionManifest,
			GeneratedRoute: reportRecord.GeneratedCodecRoute,
			Evidence:       reportRecord.ReportPath,
		}
	}

	blockers := make([]intakeBlocker, 0, 4)
	if !metadataFound {
		blockers = append(blockers, intakeBlocker{
			Kind:   "checked_metadata_fixture_missing",
			Path:   options.metadataRoot,
			Reason: fmt.Sprintf("no checked Go syntax metadata fixture has source.path %q", checkedSourcePath),
		})
	}
	if !reportFound {
		blockers = append(blockers, intakeBlocker{
			Kind:   "report_ingestion_manifest_record_missing",
			Path:   options.reportIngestionManifest,
			Reason: fmt.Sprintf("no training report-ingestion record exists for selected source %q", candidate.ManifestPath),
		})
	}
	if !alreadyExisting {
		blockers = append(blockers, intakeBlocker{
			Kind: "training_corpus_manifest_record_missing",
			Path: options.existingManifestPath,
			Reason: fmt.Sprintf(
				"selected source %q is absent from the training corpus manifest; report-ingestion records must move with the paired corpus manifest because the packager validates matching record counts and paths",
				candidate.ManifestPath,
			),
		})
	}
	if !metadataFound || rendererGaps.MissingGeneratedCodecRendererRuleCount > 0 {
		reason := "current Go real-source generated-codec reports are produced by route-specific renderer branches and hand-registered report-ingestion records; ingesting the selected source would require another narrow manual renderer instead of reusable parser-backed source-file intake"
		if metadataFound && rendererGaps.MissingGeneratedCodecRendererRuleCount > 0 {
			ruleNoun := "rule node kind"
			if rendererGaps.MissingGeneratedCodecRendererRuleCount != 1 {
				ruleNoun = "rule node kinds"
			}
			reason = fmt.Sprintf(
				"selected checked metadata contains %d syntax %s absent from inventory-backed raw-tree renderer codegen coverage: %s; reusable renderer coverage should cover these before adding a report-ingestion record",
				rendererGaps.MissingGeneratedCodecRendererRuleCount,
				ruleNoun,
				strings.Join(rendererGaps.MissingGeneratedCodecRendererRules, ", "),
			)
		}
		blockers = append(blockers, intakeBlocker{
			Kind:   "generated_codec_renderer_abstraction_missing",
			Path:   "src/syntax_generated_codec.rs::render_go_syntax_codec_fixture",
			Reason: reason,
		})
	}
	if metadataFound && rendererGaps.MissingGeneratedCodecRendererRuleCount == 0 && !sourceRouteSupport.Found {
		blockers = append(blockers, intakeBlocker{
			Kind: "generated_codec_source_route_missing",
			Path: "src/syntax_generated_codec.rs::render_go_syntax_codec_fixture",
			Reason: fmt.Sprintf(
				"selected source %q has checked metadata rule coverage but no report-ingestion record proves a generated-codec route for the source; inventory-backed renderer dispatch coverage proves rule readiness only, so manifest promotion must wait for reusable source-route report production",
				candidate.ManifestPath,
			),
		})
	}
	if !preferredSelected {
		blockers = append(blockers, intakeBlocker{
			Kind:   "preferred_candidate_not_selected",
			Path:   options.preferredManifestPath,
			Reason: "preferred candidate was absent from the current selected_candidates set; the report selected the smallest current selected candidate instead",
		})
	}
	if constructCoverage.UnsupportedConstructCount > 0 {
		blockers = append(blockers, intakeBlocker{
			Kind:   "unsupported_syntax_constructs_for_reusable_intake",
			Path:   candidate.ManifestPath,
			Reason: fmt.Sprintf("selected source uses %d construct classes absent from checked Go syntax metadata coverage: %s", constructCoverage.UnsupportedConstructCount, strings.Join(unsupportedConstructNames(constructCoverage.UnsupportedConstructs), ", ")),
		})
	}
	sort.Slice(blockers, func(left, right int) bool {
		return blockers[left].Kind < blockers[right].Kind
	})

	status := intakeStatusReady
	nextTask := "add the selected source to the next Go syntax training corpus manifest and report-ingestion manifest"
	if len(blockers) > 0 {
		status = intakeStatusBlocked
		nextTask = "produce reusable Go generated-codec report evidence and paired Go syntax training-corpus/report-ingestion manifest records for the selected stdlib source"
		if constructCoverage.UnsupportedConstructCount > 0 {
			nextTask = fmt.Sprintf("add reusable Go syntax metadata support for the selected source's unsupported constructs before generated-codec intake: %s", strings.Join(unsupportedConstructNames(constructCoverage.UnsupportedConstructs), ", "))
		} else if metadataFound && !reportFound && rendererGaps.MissingGeneratedCodecRendererRuleCount > 0 {
			nextTask = fmt.Sprintf("implement reusable Go generated-codec renderer coverage for selected-source syntax rule gaps before adding the report record: %s", strings.Join(rendererGaps.MissingGeneratedCodecRendererRules, ", "))
		} else if metadataFound && !sourceRouteSupport.Found && rendererGaps.MissingGeneratedCodecRendererRuleCount == 0 {
			nextTask = fmt.Sprintf("implement reusable Go generated-codec source-route report support for selected source %q before paired manifest promotion", candidate.ManifestPath)
		} else if metadataFound && !reportFound && !alreadyExisting {
			nextTask = "produce reusable Go generated-codec report evidence for the selected source, then add it through paired next-version training-corpus and report-ingestion manifests"
		}
	} else if alreadyExisting && reportFound {
		nextTask = "package and validate the expanded Go syntax training corpus, then hand it to downstream loader and batch smokes without promoting model-quality or paper claims"
	}

	return intakeReport{
		ArtifactKind: intakeArtifactKind,
		Version:      intakeVersion,
		Tool:         intakeTool,
		Status:       status,
		GoVersion:    runtime.Version(),
		SelectionReport: intakeSelectionReport{
			Path:               options.selectionReportPath,
			ArtifactKind:       selection.ArtifactKind,
			Version:            selection.Version,
			Tool:               selection.Tool,
			Status:             selection.Status,
			SourceRoot:         normalizeSourceRoot(selection.SourceRoot),
			SourceRevision:     selection.SourceRevision,
			SelectedCount:      selection.SelectedCount,
			SelectedBytes:      selection.SelectedBytes,
			SelectedListSHA256: selection.SelectedListSHA256,
		},
		SelectionPolicy: selectionPolicy{
			PreferredManifestPath: options.preferredManifestPath,
			PreferredSelected:     preferredSelected,
			SelectedBy:            selectedBy,
		},
		SelectedCandidate: intakeCandidate{
			ManifestPath:     candidate.ManifestPath,
			SourcePath:       normalizeSourcePath(selection.SourceRoot, candidate.SourcePath, candidate.ManifestPath),
			SourceURI:        candidate.SourceURI,
			PackageName:      candidate.PackageName,
			SHA256:           candidate.File.SHA256,
			Bytes:            candidate.File.Bytes,
			SelectionOrder:   candidate.SelectionOrder,
			SelectionScore:   candidate.SelectionScore,
			SelectionReason:  candidate.SelectionReason,
			UniqueConstructs: append([]string(nil), candidate.UniqueConstructs...),
			ShapeSHA256:      candidate.ShapeSHA256,
		},
		CheckedSourceTargetPath: checkedSourcePath,
		CurrentEvidence: currentEvidence{
			ExistingTrainingManifestPath:          options.existingManifestPath,
			ExistingTrainingCorpusID:              existingManifest.CorpusID,
			AlreadyInExistingManifest:             alreadyExisting,
			MetadataRoot:                          options.metadataRoot,
			CheckedMetadataFixtureFound:           metadataFound,
			CheckedMetadataFixturePath:            metadataPath,
			ReportIngestionManifestPath:           options.reportIngestionManifest,
			ReportIngestionCorpusID:               reportIngestion.CorpusID,
			GeneratedCodecReportFound:             reportFound,
			GeneratedCodecRoute:                   reportRecord.GeneratedCodecRoute,
			GeneratedCodecReportPath:              reportRecord.ReportPath,
			GeneratedCodecReportArtifactKind:      reportRecord.GeneratedCodecReportArtifactKind,
			GeneratedCodecSourceSHA256:            reportRecord.SourceSHA256,
			GeneratedCodecExpectedTCTTokenCount:   reportRecord.ExpectedTCTTokenCount,
			GeneratedCodecExpectedTCTTokensSHA256: reportRecord.ExpectedTCTTokensSHA256,
			GeneratedCodecExpectedMaxRawTokenID:   reportRecord.ExpectedMaxRawTokenID,
			GeneratedCodecExpectedVocabSize:       reportRecord.ExpectedVocabSize,
			GeneratedCodecExpectedPadToken:        reportRecord.ExpectedPadToken,
			GeneratedCodecExpectedPadTokenAbsent:  reportRecord.ExpectedPadTokenAbsent,
			GeneratedCodecGoParserVersion:         reportRecord.GoParserVersion,
			GeneratedCodecSourceRoute:             sourceRouteSupport,
			GeneratedCodecRendererBoundary:        "checked go/ast raw-tree renderer codegen coverage plus the raw-tree dispatch audit prove renderer rule readiness; report-ingestion records still prove selected-source generated-codec route evidence",
			GeneratedCodecReportBoundary:          fmt.Sprintf("%s maps checked generated-codec routes to token reports consumed by the training packager", options.reportIngestionManifest),
			ReusableIntakeRequirements:            reusableIntakeRequirements(metadataFound),
			WrongDirectionAvoided:                 "did not add another route-specific generated-codec renderer for the selected source",
			SyntaxConstructCoverage:               constructCoverage,
			GeneratedCodecRendererGap:             rendererGaps,
		},
		Blockers:                  blockers,
		ClaimBoundaries:           append([]string(nil), requiredClaimBoundaries...),
		NextConcreteTask:          nextTask,
		ValidationCommandTemplate: validationCommands(options),
	}, nil
}

func normalizeIntakeOptions(options intakeOptions) intakeOptions {
	if options.selectionExistingManifestPath == "" {
		options.selectionExistingManifestPath = options.existingManifestPath
	}
	return options
}

func buildSyntaxConstructCoverage(metadataRoot string, candidate goSourceCandidate) (syntaxConstructCoverage, error) {
	supported, fixtureCount, err := checkedMetadataSupportedConstructs(metadataRoot)
	if err != nil {
		return syntaxConstructCoverage{}, err
	}
	selected, spanStatus := selectedConstructOccurrences(candidate)
	unsupported := make([]unsupportedSyntaxConstruct, 0)
	for _, construct := range sortedConstructNames(selected) {
		if supported[construct] {
			continue
		}
		occurrence := selected[construct]
		unsupported = append(unsupported, unsupportedSyntaxConstruct{
			Construct:   construct,
			Count:       occurrence.Count,
			FirstLine:   occurrence.FirstLine,
			FirstColumn: occurrence.FirstColumn,
			FirstNode:   occurrence.FirstNode,
		})
	}
	return syntaxConstructCoverage{
		CheckedMetadataFixtureCount: fixtureCount,
		SupportedConstructCount:     len(supported),
		UnsupportedConstructCount:   len(unsupported),
		SourceSpanStatus:            spanStatus,
		UnsupportedConstructs:       unsupported,
	}, nil
}

func buildRendererGapAnalysis(
	metadataRoot string,
	selectedMetadataPath string,
	metadataFound bool,
	reportIngestion reportIngestionManifest,
	rendererCodegenInventoryPath string,
) (rendererGapAnalysis, error) {
	analysis := rendererGapAnalysis{
		Analyzed:                           metadataFound,
		CoverageBasis:                      "checked go/ast raw-tree renderer codegen coverage from the pinned inventory fixture plus the raw-tree dispatch audit gate; report-ingestion records remain selected-source route evidence, not renderer rule-readiness authority",
		ExistingReportIngestionRecordCount: len(reportIngestion.Records),
		RendererReadinessSource:            "go_ast_raw_tree_renderer_codegen_coverage_and_dispatch_audit",
	}
	if !metadataFound {
		return analysis, nil
	}

	selectedRules, err := metadataRuleNodeKinds(selectedMetadataPath)
	if err != nil {
		return rendererGapAnalysis{}, err
	}
	rendererCoverage, err := buildRendererCodegenCoverage(rendererCodegenInventoryPath)
	if err != nil {
		return rendererGapAnalysis{}, err
	}
	analysis.RendererCodegenCoverageInventoryPath = rendererCoverage.InventoryPath
	analysis.RendererCodegenCoverageArtifactKind = rendererCoverage.ArtifactKind
	analysis.RendererCodegenCoverageVersion = rendererCoverage.Version
	analysis.RendererCodegenCoverageGoVersion = rendererCoverage.GoVersion
	analysis.RendererCodegenCoverageStatus = rendererCoverage.Status
	analysis.RendererCodegenCoverageNextReadyTask = rendererCoverage.NextReadyTask
	analysis.RendererCodegenCoverageReadyNodeCount = len(rendererCoverage.GeneratedSkeletonReadyNodeKinds)
	analysis.RendererCodegenSourceFileRootExcluded = rendererCoverage.SourceFileRootExcludedNodeKinds
	analysis.RawTreeRendererDispatchAuditGate = goRawTreeRendererDispatchAuditGate
	supportedRules := rendererCoverage.GeneratedSkeletonReadyNodeKinds
	for _, record := range reportIngestion.Records {
		_, found, err := findMetadataFixture(metadataRoot, filepath.ToSlash(record.SourcePath))
		if err != nil {
			return rendererGapAnalysis{}, err
		}
		if !found {
			continue
		}
		analysis.ExistingGeneratedCodecMetadataCount++
	}

	selectedRuleNames := sortedStringSet(selectedRules)
	missingRules := make([]string, 0)
	for _, rule := range selectedRuleNames {
		if supportedRules[rule] {
			analysis.CoveredSelectedMetadataRuleCount++
			continue
		}
		missingRules = append(missingRules, rule)
	}
	analysis.SelectedMetadataRuleCount = len(selectedRuleNames)
	analysis.MissingGeneratedCodecRendererRuleCount = len(missingRules)
	analysis.MissingGeneratedCodecRendererRules = missingRules
	return analysis, nil
}

func buildRendererCodegenCoverage(inventoryPath string) (rendererCodegenCoverage, error) {
	if inventoryPath == "" {
		inventoryPath = goAstSchemaInventoryDefaultPath
	}
	inventory, err := readJSON[goAstSchemaInventoryFile](inventoryPath)
	if err != nil {
		return rendererCodegenCoverage{}, fmt.Errorf("read renderer codegen inventory: %w", err)
	}
	if inventory.ArtifactKind != goAstSchemaInventoryArtifactKind {
		return rendererCodegenCoverage{}, fmt.Errorf("renderer codegen inventory artifact_kind: expected %s, got %s", goAstSchemaInventoryArtifactKind, inventory.ArtifactKind)
	}
	if inventory.Version != goAstSchemaInventoryVersion {
		return rendererCodegenCoverage{}, fmt.Errorf("renderer codegen inventory version: expected %d, got %d", goAstSchemaInventoryVersion, inventory.Version)
	}
	ready := map[string]bool{}
	sourceFileRootExcluded := make([]string, 0, 1)
	tokenizableCount := 0
	for _, node := range inventory.NodeStructs {
		nodeName := strings.TrimSpace(node.Name)
		if !node.Tokenizable || nodeName == "" {
			continue
		}
		tokenizableCount++
		if nodeName == "Package" {
			sourceFileRootExcluded = append(sourceFileRootExcluded, nodeName)
			continue
		}
		ready[nodeName] = true
	}
	sort.Strings(sourceFileRootExcluded)
	return rendererCodegenCoverage{
		InventoryPath:                   filepath.ToSlash(inventoryPath),
		ArtifactKind:                    inventory.ArtifactKind,
		Version:                         inventory.Version,
		GoVersion:                       inventory.Go.Version,
		Status:                          goAstRawTreeRendererCodegenCoverageStatusReady,
		NextReadyTask:                   goAstRawTreeRendererCodegenCoverageNextTask,
		TokenizableNodeCount:            tokenizableCount,
		GeneratedSkeletonReadyNodeKinds: ready,
		SourceFileRootExcludedNodeKinds: sourceFileRootExcluded,
	}, nil
}

func metadataRuleNodeKinds(metadataPath string) (map[string]bool, error) {
	metadata, err := readJSON[syntaxMetadataFile](metadataPath)
	if err != nil {
		return nil, fmt.Errorf("read metadata fixture %s: %w", metadataPath, err)
	}
	rules := map[string]bool{}
	for _, rule := range metadata.Rules {
		nodeKind := strings.TrimSpace(rule.NodeKind)
		if nodeKind == "" {
			nodeKind = strings.TrimSpace(rule.Name)
		}
		if nodeKind == "" {
			continue
		}
		rules[nodeKind] = true
	}
	return rules, nil
}

func sortedStringSet(values map[string]bool) []string {
	names := make([]string, 0, len(values))
	for value := range values {
		names = append(names, value)
	}
	sort.Strings(names)
	return names
}

func checkedMetadataSupportedConstructs(metadataRoot string) (map[string]bool, int, error) {
	supported := map[string]bool{}
	fixtureCount := 0
	err := filepath.WalkDir(metadataRoot, func(path string, entry fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if entry.IsDir() || filepath.Base(path) != "syntax-metadata-v0.json" {
			return nil
		}
		fixtureCount++
		metadata, err := readJSON[syntaxMetadataFile](path)
		if err != nil {
			return fmt.Errorf("read metadata fixture %s: %w", path, err)
		}
		sourcePath := filepath.FromSlash(metadata.Source.Path)
		occurrences, err := collectConstructOccurrencesFromFile(sourcePath)
		if err != nil {
			return nil
		}
		for construct := range occurrences {
			supported[construct] = true
		}
		return nil
	})
	if err != nil {
		return nil, 0, err
	}
	return supported, fixtureCount, nil
}

func selectedConstructOccurrences(candidate goSourceCandidate) (map[string]constructOccurrence, string) {
	for _, path := range []string{candidate.File.Path, candidate.SourcePath} {
		if strings.TrimSpace(path) == "" || strings.Contains(path, "$(") {
			continue
		}
		occurrences, err := collectConstructOccurrencesFromFile(filepath.FromSlash(path))
		if err == nil {
			return occurrences, "source_parsed_with_go_parser_positions"
		}
	}
	return constructOccurrencesFromCandidate(candidate), "source_unavailable_used_selection_report_counts"
}

func constructOccurrencesFromCandidate(candidate goSourceCandidate) map[string]constructOccurrence {
	occurrences := map[string]constructOccurrence{}
	for _, construct := range candidate.Constructs {
		occurrences[construct.Construct] = constructOccurrence{
			Count:     construct.Count,
			FirstNode: construct.Construct,
		}
	}
	for _, construct := range candidate.UniqueConstructs {
		if _, ok := occurrences[construct]; !ok {
			occurrences[construct] = constructOccurrence{FirstNode: construct}
		}
	}
	return occurrences
}

type constructOccurrence struct {
	Count       int
	FirstLine   int
	FirstColumn int
	FirstNode   string
}

func collectConstructOccurrencesFromFile(path string) (map[string]constructOccurrence, error) {
	source, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	fileSet := token.NewFileSet()
	file, err := parser.ParseFile(fileSet, path, source, parser.ParseComments)
	if err != nil {
		return nil, err
	}
	return collectConstructOccurrences(fileSet, file), nil
}

func collectConstructOccurrences(fileSet *token.FileSet, file *ast.File) map[string]constructOccurrence {
	occurrences := map[string]constructOccurrence{}
	recordConstruct(occurrences, fileSet, "file", "ast.File", file.Package)
	recordConstruct(occurrences, fileSet, "package_clause", "ast.Ident", file.Name.Pos())
	topLevelDecls := map[*ast.GenDecl]bool{}
	for _, decl := range file.Decls {
		if genDecl, ok := decl.(*ast.GenDecl); ok {
			topLevelDecls[genDecl] = true
		}
	}
	ast.Inspect(file, func(node ast.Node) bool {
		switch n := node.(type) {
		case *ast.GenDecl:
			switch n.Tok {
			case token.IMPORT:
				recordConstruct(occurrences, fileSet, "import_declaration", "*ast.GenDecl", n.Pos())
			case token.CONST:
				recordConstruct(occurrences, fileSet, "const_declaration", "*ast.GenDecl", n.Pos())
			case token.TYPE:
				recordConstruct(occurrences, fileSet, "type_declaration", "*ast.GenDecl", n.Pos())
			case token.VAR:
				construct := "local_var_declaration"
				if topLevelDecls[n] || n.Lparen.IsValid() {
					construct = "var_declaration"
				}
				recordConstruct(occurrences, fileSet, construct, "*ast.GenDecl", n.Pos())
			}
		case *ast.FuncDecl:
			if n.Recv != nil {
				recordConstruct(occurrences, fileSet, "method_declaration", "*ast.FuncDecl", n.Pos())
			} else {
				recordConstruct(occurrences, fileSet, "function_declaration", "*ast.FuncDecl", n.Pos())
			}
		case *ast.ImportSpec:
			recordConstruct(occurrences, fileSet, "import_spec", "*ast.ImportSpec", n.Pos())
		case *ast.ValueSpec:
			recordConstruct(occurrences, fileSet, "value_spec", "*ast.ValueSpec", n.Pos())
		case *ast.TypeSpec:
			recordConstruct(occurrences, fileSet, "type_spec", "*ast.TypeSpec", n.Pos())
		case *ast.Field:
			recordConstruct(occurrences, fileSet, "field", "*ast.Field", n.Pos())
		case *ast.FieldList:
			recordConstruct(occurrences, fileSet, "field_list", "*ast.FieldList", n.Pos())
		case *ast.BlockStmt:
			recordConstruct(occurrences, fileSet, "block_statement", "*ast.BlockStmt", n.Pos())
		case *ast.ReturnStmt:
			recordConstruct(occurrences, fileSet, "return_statement", "*ast.ReturnStmt", n.Pos())
		case *ast.AssignStmt:
			switch n.Tok {
			case token.DEFINE:
				recordConstruct(occurrences, fileSet, "short_var_declaration", "*ast.AssignStmt", n.Pos())
			case token.ASSIGN:
				recordConstruct(occurrences, fileSet, "assignment_statement", "*ast.AssignStmt", n.Pos())
			default:
				recordConstruct(occurrences, fileSet, "compound_assignment_statement", "*ast.AssignStmt", n.Pos())
			}
		case *ast.IncDecStmt:
			recordConstruct(occurrences, fileSet, "increment_decrement_statement", "*ast.IncDecStmt", n.Pos())
		case *ast.BranchStmt:
			recordConstruct(occurrences, fileSet, "branch_statement", "*ast.BranchStmt", n.Pos())
		case *ast.IfStmt:
			recordConstruct(occurrences, fileSet, "if_statement", "*ast.IfStmt", n.Pos())
		case *ast.ForStmt:
			recordConstruct(occurrences, fileSet, "for_statement", "*ast.ForStmt", n.Pos())
		case *ast.RangeStmt:
			recordConstruct(occurrences, fileSet, "range_statement", "*ast.RangeStmt", n.Pos())
		case *ast.SwitchStmt, *ast.TypeSwitchStmt:
			recordConstruct(occurrences, fileSet, "switch_statement", fmt.Sprintf("%T", n), n.Pos())
		case *ast.CaseClause:
			recordConstruct(occurrences, fileSet, "case_clause", "*ast.CaseClause", n.Pos())
		case *ast.DeclStmt:
			recordConstruct(occurrences, fileSet, "declaration_statement", "*ast.DeclStmt", n.Pos())
		case *ast.GoStmt:
			recordConstruct(occurrences, fileSet, "go_statement", "*ast.GoStmt", n.Pos())
		case *ast.DeferStmt:
			recordConstruct(occurrences, fileSet, "defer_statement", "*ast.DeferStmt", n.Pos())
		case *ast.SendStmt:
			recordConstruct(occurrences, fileSet, "send_statement", "*ast.SendStmt", n.Pos())
		case *ast.SelectStmt:
			recordConstruct(occurrences, fileSet, "select_statement", "*ast.SelectStmt", n.Pos())
		case *ast.CallExpr:
			recordConstruct(occurrences, fileSet, "call_expression", "*ast.CallExpr", n.Pos())
		case *ast.SelectorExpr:
			recordConstruct(occurrences, fileSet, "selector_expression", "*ast.SelectorExpr", n.Pos())
		case *ast.Ident:
			recordConstruct(occurrences, fileSet, "identifier_expression", "*ast.Ident", n.Pos())
		case *ast.BasicLit:
			switch n.Kind {
			case token.INT:
				recordConstruct(occurrences, fileSet, "basic_lit_integer", "*ast.BasicLit", n.Pos())
			case token.STRING, token.CHAR:
				recordConstruct(occurrences, fileSet, "basic_lit_string", "*ast.BasicLit", n.Pos())
			default:
				recordConstruct(occurrences, fileSet, "basic_lit_other", "*ast.BasicLit", n.Pos())
			}
		case *ast.BinaryExpr:
			recordConstruct(occurrences, fileSet, "binary_expression", "*ast.BinaryExpr", n.Pos())
		case *ast.UnaryExpr:
			recordConstruct(occurrences, fileSet, "unary_expression", "*ast.UnaryExpr", n.Pos())
		case *ast.ParenExpr:
			recordConstruct(occurrences, fileSet, "parenthesized_expression", "*ast.ParenExpr", n.Pos())
		case *ast.CompositeLit:
			recordConstruct(occurrences, fileSet, "composite_literal", "*ast.CompositeLit", n.Pos())
		case *ast.KeyValueExpr:
			recordConstruct(occurrences, fileSet, "key_value_expression", "*ast.KeyValueExpr", n.Pos())
		case *ast.ArrayType:
			recordConstruct(occurrences, fileSet, "array_type", "*ast.ArrayType", n.Pos())
		case *ast.IndexExpr, *ast.IndexListExpr:
			recordConstruct(occurrences, fileSet, "index_expression", fmt.Sprintf("%T", n), n.Pos())
		case *ast.SliceExpr:
			recordConstruct(occurrences, fileSet, "slice_expression", "*ast.SliceExpr", n.Pos())
		case *ast.StarExpr:
			recordConstruct(occurrences, fileSet, "star_expression", "*ast.StarExpr", n.Pos())
		case *ast.StructType:
			recordConstruct(occurrences, fileSet, "struct_type", "*ast.StructType", n.Pos())
		case *ast.InterfaceType:
			recordConstruct(occurrences, fileSet, "interface_type", "*ast.InterfaceType", n.Pos())
		case *ast.FuncLit:
			recordConstruct(occurrences, fileSet, "function_literal", "*ast.FuncLit", n.Pos())
		case *ast.FuncType:
			recordConstruct(occurrences, fileSet, "function_type", "*ast.FuncType", n.Pos())
		case *ast.MapType:
			recordConstruct(occurrences, fileSet, "map_type", "*ast.MapType", n.Pos())
		case *ast.ChanType:
			recordConstruct(occurrences, fileSet, "channel_type", "*ast.ChanType", n.Pos())
		case *ast.TypeAssertExpr:
			recordConstruct(occurrences, fileSet, "type_assertion_expression", "*ast.TypeAssertExpr", n.Pos())
		}
		return true
	})
	return occurrences
}

func recordConstruct(occurrences map[string]constructOccurrence, fileSet *token.FileSet, construct string, nodeKind string, pos token.Pos) {
	occurrence := occurrences[construct]
	occurrence.Count++
	if occurrence.FirstLine == 0 && pos.IsValid() {
		position := fileSet.Position(pos)
		occurrence.FirstLine = position.Line
		occurrence.FirstColumn = position.Column
		occurrence.FirstNode = nodeKind
	}
	if occurrence.FirstNode == "" {
		occurrence.FirstNode = nodeKind
	}
	occurrences[construct] = occurrence
}

func sortedConstructNames(occurrences map[string]constructOccurrence) []string {
	names := make([]string, 0, len(occurrences))
	for name := range occurrences {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}

func unsupportedConstructNames(constructs []unsupportedSyntaxConstruct) []string {
	names := make([]string, 0, len(constructs))
	for _, construct := range constructs {
		names = append(names, construct.Construct)
	}
	return names
}

func reusableIntakeRequirements(metadataFound bool) []string {
	requirements := make([]string, 0, 4)
	if !metadataFound {
		requirements = append(requirements, "derive checked parser-backed metadata for the selected source without adding a one-off fixture-specific rule function")
	}
	return append(requirements,
		"lower checked parser-backed metadata through GrammarIR and StructuralModel using the normal Go syntax-level path",
		"produce generated-codec round-trip, canonical render, gofmt, go/parser, token-count, and report-ingestion evidence through reusable source-file machinery",
		"preserve source hashes, split seed, artifact hashes, commands, tool versions, and claim boundaries before packaging the next training corpus",
	)
}

func chooseCandidate(candidates []goSourceCandidate, preferredManifestPath string) (goSourceCandidate, string, bool, error) {
	if len(candidates) == 0 {
		return goSourceCandidate{}, "", false, errors.New("selection report has no selected_candidates")
	}
	var smallest *goSourceCandidate
	for index := range candidates {
		candidate := &candidates[index]
		if candidate.ManifestPath == preferredManifestPath {
			return *candidate, "preferred_selected_candidate", true, nil
		}
		if smallest == nil ||
			candidate.File.Bytes < smallest.File.Bytes ||
			(candidate.File.Bytes == smallest.File.Bytes && candidate.ManifestPath < smallest.ManifestPath) {
			smallest = candidate
		}
	}
	return *smallest, "smallest_selected_candidate_fallback", false, nil
}

type metadataFixtureMatch struct {
	path      string
	ruleCount int
}

func findMetadataFixture(metadataRoot string, expectedSourcePath string) (string, bool, error) {
	var matches []metadataFixtureMatch
	err := filepath.WalkDir(metadataRoot, func(path string, entry fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if entry.IsDir() || filepath.Base(path) != "syntax-metadata-v0.json" {
			return nil
		}
		metadata, err := readJSON[syntaxMetadataFile](path)
		if err != nil {
			return fmt.Errorf("read metadata fixture %s: %w", path, err)
		}
		if filepath.ToSlash(metadata.Source.Path) == expectedSourcePath {
			matches = append(matches, metadataFixtureMatch{
				path:      filepath.ToSlash(path),
				ruleCount: len(metadata.Rules),
			})
		}
		return nil
	})
	if err != nil {
		return "", false, err
	}
	if len(matches) == 0 {
		return "", false, nil
	}
	sort.Slice(matches, func(left, right int) bool {
		if matches[left].ruleCount != matches[right].ruleCount {
			return matches[left].ruleCount > matches[right].ruleCount
		}
		return matches[left].path < matches[right].path
	})
	return matches[0].path, true, nil
}

func findReportIngestionRecord(manifest reportIngestionManifest, manifestPath string, checkedSourcePath string) (reportIngestionRecord, bool) {
	for _, record := range manifest.Records {
		if record.ManifestPath == manifestPath &&
			filepath.ToSlash(record.SourcePath) == checkedSourcePath &&
			!isSourceSpanReportIngestionRecord(record) {
			return record, true
		}
	}
	return reportIngestionRecord{}, false
}

func isSourceSpanReportIngestionRecord(record reportIngestionRecord) bool {
	return record.EvidenceScope == "source_span_generated_codec" ||
		strings.Contains(record.GeneratedCodecRoute, "source_span")
}

func manifestContains(manifest corpusManifest, manifestPath string) bool {
	for _, entry := range manifest.Entries {
		if entry.ManifestPath == manifestPath {
			return true
		}
	}
	return false
}

func normalizeSourceRoot(sourceRoot string) string {
	gorootSrc := filepath.Join(runtime.GOROOT(), "src")
	if sameCleanPath(sourceRoot, gorootSrc) {
		return "$(go env GOROOT)/src"
	}
	return filepath.ToSlash(sourceRoot)
}

func normalizeSourcePath(sourceRoot string, sourcePath string, manifestPath string) string {
	if sourceRoot != "" && sameCleanPath(filepath.Dir(filepath.Join(sourceRoot, manifestPath)), filepath.Dir(sourcePath)) {
		return "$(go env GOROOT)/src/" + filepath.ToSlash(manifestPath)
	}
	if sourceRoot != "" {
		if rel, err := filepath.Rel(sourceRoot, sourcePath); err == nil && filepath.ToSlash(rel) == filepath.ToSlash(manifestPath) {
			return "$(go env GOROOT)/src/" + filepath.ToSlash(manifestPath)
		}
	}
	return filepath.ToSlash(sourcePath)
}

func sameCleanPath(left string, right string) bool {
	return filepath.Clean(left) == filepath.Clean(right)
}

func validationCommands(options intakeOptions) []string {
	reportPath := options.validateReportPath
	if reportPath == "" {
		reportPath = options.outputPath
	}
	return []string{
		"go test scripts/analysis/go_syntax_corpus_select.go scripts/analysis/go_syntax_corpus_select_test.go",
		fmt.Sprintf("go run scripts/analysis/go_syntax_corpus_select.go --source-root \"$(go env GOROOT)/src\" --existing-manifest %s --exclude-existing --include-test-files=false --dedupe-shapes --max-files 24 --max-file-bytes 12000 --max-total-bytes 80000 --output target/tct-analysis/go-syntax-corpus-selection/goroot-src-report.json",
			options.selectionExistingManifestPath,
		),
		fmt.Sprintf("go run scripts/analysis/go_syntax_generated_codec_intake.go --selection-report %s --preferred-manifest-path %s --existing-manifest %s --metadata-root %s --report-ingestion-manifest %s --renderer-codegen-inventory %s --validate-report %s",
			options.selectionReportPath,
			options.preferredManifestPath,
			options.existingManifestPath,
			options.metadataRoot,
			options.reportIngestionManifest,
			options.rendererCodegenInventoryPath,
			reportPath,
		),
	}
}

func validateIntakeReport(actual intakeReport, expected intakeReport) error {
	if actual.ArtifactKind != intakeArtifactKind {
		return fmt.Errorf("artifact_kind: expected %s, got %s", intakeArtifactKind, actual.ArtifactKind)
	}
	if actual.Version != intakeVersion {
		return fmt.Errorf("version: expected %d, got %d", intakeVersion, actual.Version)
	}
	if actual.Tool != intakeTool {
		return fmt.Errorf("tool: expected %s, got %s", intakeTool, actual.Tool)
	}
	if actual.Status != intakeStatusBlocked && actual.Status != intakeStatusReady {
		return fmt.Errorf("status: expected ready or blocked, got %s", actual.Status)
	}
	if actual.Status != expected.Status {
		return fmt.Errorf("status drift: expected %s from current inputs, got %s", expected.Status, actual.Status)
	}
	if actual.SelectedCandidate.ManifestPath != expected.SelectedCandidate.ManifestPath {
		return fmt.Errorf("selected_candidate.manifest_path drift: expected %s, got %s", expected.SelectedCandidate.ManifestPath, actual.SelectedCandidate.ManifestPath)
	}
	if actual.SelectedCandidate.SHA256 != expected.SelectedCandidate.SHA256 {
		return fmt.Errorf("selected_candidate.sha256 drift: expected %s, got %s", expected.SelectedCandidate.SHA256, actual.SelectedCandidate.SHA256)
	}
	if actual.CheckedSourceTargetPath != expected.CheckedSourceTargetPath {
		return fmt.Errorf("checked_source_target_path drift: expected %s, got %s", expected.CheckedSourceTargetPath, actual.CheckedSourceTargetPath)
	}
	if actual.CurrentEvidence.CheckedMetadataFixtureFound != expected.CurrentEvidence.CheckedMetadataFixtureFound {
		return errors.New("checked_metadata_fixture_found drift")
	}
	if actual.CurrentEvidence.GeneratedCodecReportFound != expected.CurrentEvidence.GeneratedCodecReportFound {
		return errors.New("generated_codec_report_found drift")
	}
	if actual.CurrentEvidence.GeneratedCodecSourceRoute.Found != expected.CurrentEvidence.GeneratedCodecSourceRoute.Found {
		return errors.New("generated_codec_source_route_support.found drift")
	}
	if actual.CurrentEvidence.GeneratedCodecSourceRoute.GeneratedRoute != expected.CurrentEvidence.GeneratedCodecSourceRoute.GeneratedRoute {
		return errors.New("generated_codec_source_route_support.generated_codec_route drift")
	}
	if !reflect.DeepEqual(blockerKinds(actual.Blockers), blockerKinds(expected.Blockers)) {
		return fmt.Errorf("blocker kind drift: expected %v, got %v", blockerKinds(expected.Blockers), blockerKinds(actual.Blockers))
	}
	if !reflect.DeepEqual(actual.ClaimBoundaries, requiredClaimBoundaries) {
		return fmt.Errorf("claim_boundaries must equal %v", requiredClaimBoundaries)
	}
	if actual.NextConcreteTask == "" {
		return errors.New("next_concrete_task must not be empty")
	}
	return nil
}

func blockerKinds(blockers []intakeBlocker) []string {
	kinds := make([]string, 0, len(blockers))
	for _, blocker := range blockers {
		kinds = append(kinds, blocker.Kind)
	}
	sort.Strings(kinds)
	return kinds
}

func readJSON[T any](path string) (T, error) {
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

func writeJSON(path string, value any) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	data, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		return err
	}
	data = append(data, '\n')
	return os.WriteFile(path, data, 0o644)
}
