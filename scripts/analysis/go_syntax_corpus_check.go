// go_syntax_corpus_check validates a pinned parser-backed Go syntax corpus
// manifest and reports which source constructs are not covered by the current
// sidecar syntax route fixtures.
package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
)

const (
	artifactKind             = "tct_go_syntax_corpus_check_v1"
	corpusManifestKind       = "tct_go_syntax_real_source_corpus_manifest_v1"
	selectionArtifactKind    = "tct_go_syntax_corpus_selection_v1"
	reportVersion            = 1
	defaultMetadataRoot      = "tests/fixtures/go_syntax_metadata"
	sourceManifestMarker     = "# tct-go-syntax-route-corpus-v1"
	noStablePromotionClaim   = "sidecar-only parser-backed corpus evidence; no TctCompiler route and no stable backend promotion"
	metadataProjectorCommand = "go run scripts/analysis/go_syntax_metadata_smoke.go --self-test"
)

var supportedRouteFixtures = []string{
	"minimal-function",
	"local-declaration",
	"local-assignment",
	"local-escaped-assignment",
	"local-short-var-declaration",
	"local-integer-short-var-declaration",
	"local-integer-return-literal",
	"local-parenthesized-integer-return-expression",
	"local-unary-integer-return-expression",
	"local-binary-integer-return-expression",
	"local-call-return-expression",
	"local-string-return-literal",
	"local-float-return-literal",
	"local-boolean-return-identifier",
	"local-nil-return-identifier",
	"local-typed-declaration",
	"local-zero-value-typed-declaration",
	"local-integer-typed-declaration",
	"local-untyped-integer-declaration",
	"local-integer-assignment",
	"local-compound-assignment",
	"local-increment-decrement",
	"local-branch-statement",
	"local-if-else",
	"local-for-condition",
	"local-switch",
	"local-switch-init",
	"local-type-switch",
	"local-range-string",
	"package-type-declaration",
	"type-spec-assign",
	"type-spec-comment",
	"package-const-declaration",
	"method-declaration",
	"struct-type",
	"composite-literal",
	"key-value-composite-literal",
	"array-composite-literal",
	"index-expression",
	"index-list-expression",
	"slice-expression",
	"slice-expression-three-index",
	"star-expression",
	"package-var-declaration",
	"function-literal",
	"local-generic-function",
	"concurrency-map-interface-type-assertion",
	"local-variadic-ellipsis",
	"value-spec-comment",
	"import-spec-comment",
}

var supportedConstructs = map[string]bool{
	"assignment_statement":          true,
	"array_type":                    true,
	"basic_lit_integer":             true,
	"basic_lit_other":               true,
	"basic_lit_string":              true,
	"binary_expression":             true,
	"branch_statement":              true,
	"block_statement":               true,
	"case_clause":                   true,
	"call_expression":               true,
	"channel_type":                  true,
	"compound_assignment_statement": true,
	"composite_literal":             true,
	"const_declaration":             true,
	"declaration_statement":         true,
	"defer_statement":               true,
	"field":                         true,
	"field_list":                    true,
	"for_statement":                 true,
	"function_declaration":          true,
	"function_literal":              true,
	"function_type":                 true,
	"go_statement":                  true,
	"identifier_expression":         true,
	"if_statement":                  true,
	"increment_decrement_statement": true,
	"index_expression":              true,
	"import_declaration":            true,
	"import_spec":                   true,
	"interface_type":                true,
	"key_value_expression":          true,
	"local_var_declaration":         true,
	"map_type":                      true,
	"method_declaration":            true,
	"package_clause":                true,
	"parenthesized_expression":      true,
	"range_statement":               true,
	"return_statement":              true,
	"select_statement":              true,
	"selector_expression":           true,
	"send_statement":                true,
	"short_var_declaration":         true,
	"slice_expression":              true,
	"star_expression":               true,
	"struct_type":                   true,
	"switch_statement":              true,
	"type_assertion_expression":     true,
	"type_declaration":              true,
	"type_spec":                     true,
	"unary_expression":              true,
	"value_spec":                    true,
	"var_declaration":               true,
}

var nextConstructPriority = []string{
	"if_statement",
	"for_statement",
	"range_statement",
	"switch_statement",
	"case_clause",
	"type_declaration",
	"method_declaration",
	"struct_type",
	"interface_type",
	"composite_literal",
	"function_literal",
	"key_value_expression",
	"array_type",
	"index_expression",
	"slice_expression",
	"star_expression",
	"const_declaration",
	"var_declaration",
	"compound_assignment_statement",
	"increment_decrement_statement",
	"branch_statement",
	"basic_lit_other",
}

var grammarCategoryOrder = []string{
	"packages_imports_declarations",
	"functions_methods",
	"statements_control_flow",
	"expressions_and_precedence",
	"types_and_generics",
	"literals_and_composites",
	"comments_directives_build_tags",
	"uncategorized",
}

type corpusManifest struct {
	ArtifactKind             string          `json:"artifact_kind"`
	Version                  int             `json:"version"`
	CorpusID                 string          `json:"corpus_id"`
	SourceRoot               string          `json:"source_root"`
	SourceManifest           string          `json:"source_manifest"`
	RouteSourceManifestShape routeSourceInfo `json:"route_source_manifest_shape"`
	Selection                selectionInfo   `json:"selection"`
	Entries                  []manifestEntry `json:"entries"`
	NoStablePromotionClaim   string          `json:"no_stable_promotion_claim"`
}

type routeSourceInfo struct {
	Tool                string `json:"tool"`
	Version             int    `json:"version"`
	Target              string `json:"target"`
	Marker              string `json:"marker"`
	CorpusFingerprint   string `json:"corpus_fingerprint_sha256"`
	SourceListSHA256    string `json:"source_list_sha256"`
	MinimumEntryCount   int    `json:"minimum_entry_count"`
	FixtureScope        bool   `json:"fixture_scope"`
	RouteDecisionMarker string `json:"route_decision_marker"`
}

type selectionInfo struct {
	Source               string `json:"source"`
	SourceRevision       string `json:"source_revision"`
	License              string `json:"license"`
	LicenseTextPath      string `json:"license_text_path"`
	SelectionNote        string `json:"selection_note"`
	ExpectedNextGap      string `json:"expected_next_missing_construct"`
	ExpectedEntryCount   int    `json:"expected_entry_count"`
	RepresentativeStatus string `json:"representative_status"`
}

type manifestEntry struct {
	ManifestPath  string       `json:"manifest_path"`
	File          fileArtifact `json:"file"`
	License       string       `json:"license"`
	SourceURI     string       `json:"source_uri"`
	SelectionNote string       `json:"selection_note"`
}

type fileArtifact struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int64  `json:"bytes"`
}

type corpusReport struct {
	Tool                    string                 `json:"tool"`
	Version                 int                    `json:"version"`
	ArtifactKind            string                 `json:"artifact_kind"`
	Status                  string                 `json:"status"`
	Manifest                fileArtifact           `json:"manifest"`
	SelectionReport         *fileArtifact          `json:"selection_report,omitempty"`
	SourceManifest          fileArtifact           `json:"source_manifest"`
	CorpusID                string                 `json:"corpus_id"`
	SourceRoot              string                 `json:"source_root"`
	EntryCount              int                    `json:"entry_count"`
	MetadataProjector       metadataProjector      `json:"metadata_projector"`
	GrammarCoverage         grammarCoverageSummary `json:"grammar_coverage"`
	SupportedRouteFixtures  []string               `json:"supported_route_fixtures"`
	SupportedConstructs     []string               `json:"supported_constructs"`
	ObservedConstructs      []constructCount       `json:"observed_constructs"`
	UnsupportedConstructs   []constructCount       `json:"unsupported_constructs"`
	NextMissingConstruct    *constructCount        `json:"next_missing_construct"`
	UnsupportedGrammarGaps  []grammarGapCount      `json:"unsupported_grammar_gaps"`
	NextMissingGrammarGap   *grammarGapCount       `json:"next_missing_grammar_gap"`
	Files                   []fileConstructSummary `json:"files"`
	NoStablePromotionClaim  string                 `json:"no_stable_promotion_claim"`
	StablePromotionBlockers []string               `json:"stable_promotion_blockers"`
}

type metadataProjector struct {
	Command string `json:"command"`
	Role    string `json:"role"`
}

type constructCount struct {
	Construct string   `json:"construct"`
	Count     int      `json:"count"`
	Files     []string `json:"files"`
}

type fileConstructSummary struct {
	Path                   string            `json:"path"`
	SHA256                 string            `json:"sha256"`
	Bytes                  int64             `json:"bytes"`
	Package                string            `json:"package"`
	Constructs             []string          `json:"constructs"`
	UnsupportedConstructs  []string          `json:"unsupported_constructs"`
	UnsupportedGrammarGaps []grammarGapCount `json:"unsupported_grammar_gaps"`
}

type grammarCoverageSummary struct {
	Source               string `json:"source"`
	MetadataRoot         string `json:"metadata_root"`
	MetadataFixtureCount int    `json:"metadata_fixture_count"`
	NodeKindCount        int    `json:"node_kind_count"`
	GrammarFieldCount    int    `json:"grammar_field_count"`
	PayloadKindCount     int    `json:"payload_kind_count"`
}

type grammarCoverage struct {
	summary      grammarCoverageSummary
	nodeKinds    map[string]bool
	fields       map[string]bool
	payloadKinds map[string]bool
}

type syntaxMetadata struct {
	Rules []syntaxMetadataRule `json:"rules"`
}

type syntaxMetadataRule struct {
	Name     string                `json:"name"`
	NodeKind string                `json:"node_kind"`
	Fields   []syntaxMetadataField `json:"fields"`
}

type syntaxMetadataField struct {
	Name         string                      `json:"name"`
	Alternatives []syntaxMetadataAlternative `json:"alternatives"`
}

type syntaxMetadataAlternative struct {
	Terminal string `json:"terminal"`
}

type grammarGapCount struct {
	GrammarCategory     string   `json:"grammar_category"`
	InteractionCategory string   `json:"interaction_category"`
	ASTNodeKind         string   `json:"ast_node_kind"`
	GrammarField        string   `json:"grammar_field"`
	PayloadKind         string   `json:"payload_kind"`
	Construct           string   `json:"construct"`
	Count               int      `json:"count"`
	Files               []string `json:"files"`
}

type selectionReport struct {
	ArtifactKind       string               `json:"artifact_kind"`
	Version            int                  `json:"version"`
	Tool               string               `json:"tool"`
	Status             string               `json:"status"`
	CorpusID           string               `json:"corpus_id"`
	SourceRoot         string               `json:"source_root"`
	SelectedCandidates []selectionCandidate `json:"selected_candidates"`
}

type selectionCandidate struct {
	ManifestPath string       `json:"manifest_path"`
	SourcePath   string       `json:"source_path"`
	File         fileArtifact `json:"file"`
}

func main() {
	manifestPath := flag.String("manifest", "", "checked Go syntax corpus manifest")
	selectionReportPath := flag.String("selection-report", "", "Go syntax corpus selection report to classify selected candidates")
	metadataRoot := flag.String("metadata-root", defaultMetadataRoot, "checked Go syntax metadata root used as current support coverage")
	jsonOutput := flag.String("json-output", "", "write JSON report to this path instead of stdout")
	selfTest := flag.Bool("self-test", false, "run built-in checks")
	flag.Parse()

	if *selfTest {
		if err := runSelfTest(); err != nil {
			exitf("go_syntax_corpus_check self-test failed: %v", err)
		}
		fmt.Println("go_syntax_corpus_check self-test passed")
		return
	}
	if (*manifestPath == "") == (*selectionReportPath == "") {
		exitf("exactly one of --manifest or --selection-report is required unless --self-test is used")
	}
	var report *corpusReport
	var err error
	if *manifestPath != "" {
		report, err = analyzeCorpus(*manifestPath, *metadataRoot)
	} else {
		report, err = analyzeSelectionReport(*selectionReportPath, *metadataRoot)
	}
	if err != nil {
		exitf("%v", err)
	}
	rendered, err := json.MarshalIndent(report, "", "  ")
	if err != nil {
		exitf("render report: %v", err)
	}
	rendered = append(rendered, '\n')
	if *jsonOutput == "" {
		fmt.Print(string(rendered))
		return
	}
	if err := os.MkdirAll(filepath.Dir(*jsonOutput), 0o755); err != nil {
		exitf("create report directory: %v", err)
	}
	if err := os.WriteFile(*jsonOutput, rendered, 0o644); err != nil {
		exitf("write report: %v", err)
	}
}

func analyzeCorpus(manifestPath string, metadataRoot string) (*corpusReport, error) {
	manifestInfo, err := fileInfo(manifestPath, manifestPath)
	if err != nil {
		return nil, err
	}
	manifest, err := loadManifest(manifestPath)
	if err != nil {
		return nil, err
	}
	if err := validateManifestShape(manifest); err != nil {
		return nil, err
	}
	sourceList, err := readSourceManifest(manifest.SourceManifest)
	if err != nil {
		return nil, err
	}
	sourceManifestInfo, err := fileInfo(manifest.SourceManifest, manifest.SourceManifest)
	if err != nil {
		return nil, err
	}
	if err := validateSourceManifest(sourceList, manifest, sourceManifestInfo); err != nil {
		return nil, err
	}
	if _, err := fileInfo(manifest.Selection.LicenseTextPath, manifest.Selection.LicenseTextPath); err != nil {
		return nil, fmt.Errorf("validate corpus license text: %w", err)
	}
	coverage, err := loadGrammarCoverage(metadataRoot)
	if err != nil {
		return nil, err
	}

	observed := map[string]*constructCount{}
	unsupported := map[string]*constructCount{}
	unsupportedGrammarGaps := map[string]*grammarGapCount{}
	fileSummaries := make([]fileConstructSummary, 0, len(manifest.Entries))
	for _, entry := range manifest.Entries {
		path := filepath.Join(manifest.SourceRoot, filepath.FromSlash(entry.ManifestPath))
		info, err := fileInfo(path, entry.File.Path)
		if err != nil {
			return nil, err
		}
		if info.SHA256 != entry.File.SHA256 {
			return nil, fmt.Errorf("%s: sha256 mismatch: manifest=%s actual=%s", entry.ManifestPath, entry.File.SHA256, info.SHA256)
		}
		if info.Bytes != entry.File.Bytes {
			return nil, fmt.Errorf("%s: byte length mismatch: manifest=%d actual=%d", entry.ManifestPath, entry.File.Bytes, info.Bytes)
		}
		fileSummary, err := analyzeFile(path, info, coverage)
		if err != nil {
			return nil, err
		}
		fileSummaries = append(fileSummaries, fileSummary)
		for _, construct := range fileSummary.Constructs {
			addConstruct(observed, construct, fileSummary.Path)
			if !supportedConstructs[construct] {
				addConstruct(unsupported, construct, fileSummary.Path)
			}
		}
		for _, gap := range fileSummary.UnsupportedGrammarGaps {
			addGrammarGap(unsupportedGrammarGaps, gap, fileSummary.Path)
		}
	}

	report := &corpusReport{
		Tool:           "go_syntax_corpus_check",
		Version:        reportVersion,
		ArtifactKind:   artifactKind,
		Status:         "ready",
		Manifest:       manifestInfo,
		SourceManifest: sourceManifestInfo,
		CorpusID:       manifest.CorpusID,
		SourceRoot:     manifest.SourceRoot,
		EntryCount:     len(manifest.Entries),
		MetadataProjector: metadataProjector{
			Command: metadataProjectorCommand,
			Role:    "parser-backed checked fixture projector that defines current Go syntax route coverage",
		},
		GrammarCoverage:         coverage.summary,
		SupportedRouteFixtures:  append([]string(nil), supportedRouteFixtures...),
		SupportedConstructs:     sortedSupportedConstructs(),
		ObservedConstructs:      sortedConstructCounts(observed),
		UnsupportedConstructs:   sortedConstructCounts(unsupported),
		UnsupportedGrammarGaps:  sortedGrammarGaps(unsupportedGrammarGaps),
		Files:                   fileSummaries,
		NoStablePromotionClaim:  noStablePromotionClaim,
		StablePromotionBlockers: []string{"real_source_end_to_end_generated_codec_round_trip"},
	}
	if len(report.UnsupportedConstructs) > 0 {
		report.Status = "ready_with_unsupported_constructs"
		report.NextMissingConstruct = selectNextMissingConstruct(unsupported)
	}
	if len(report.UnsupportedGrammarGaps) > 0 {
		report.Status = "ready_with_unsupported_grammar_gaps"
		report.NextMissingGrammarGap = selectNextGrammarGap(unsupportedGrammarGaps)
	}
	if err := validateExpectedNextGap(manifest.Selection.ExpectedNextGap, report.NextMissingConstruct, report.NextMissingGrammarGap); err != nil {
		return nil, err
	}
	return report, nil
}

func analyzeSelectionReport(selectionReportPath string, metadataRoot string) (*corpusReport, error) {
	reportInfo, err := fileInfo(selectionReportPath, selectionReportPath)
	if err != nil {
		return nil, err
	}
	selection, err := loadSelectionReport(selectionReportPath)
	if err != nil {
		return nil, err
	}
	if selection.ArtifactKind != selectionArtifactKind {
		return nil, fmt.Errorf("selection report artifact_kind must be %q", selectionArtifactKind)
	}
	if selection.Version != 1 {
		return nil, fmt.Errorf("selection report version must be 1")
	}
	if selection.Tool != "go_syntax_corpus_select" {
		return nil, fmt.Errorf("selection report tool must be go_syntax_corpus_select")
	}
	if len(selection.SelectedCandidates) == 0 {
		return nil, errors.New("selection report has no selected candidates")
	}
	coverage, err := loadGrammarCoverage(metadataRoot)
	if err != nil {
		return nil, err
	}

	observed := map[string]*constructCount{}
	unsupported := map[string]*constructCount{}
	unsupportedGrammarGaps := map[string]*grammarGapCount{}
	fileSummaries := make([]fileConstructSummary, 0, len(selection.SelectedCandidates))
	for _, candidate := range selection.SelectedCandidates {
		path := normalizeSelectionSourcePath(candidate.SourcePath, selection.SourceRoot, candidate.ManifestPath)
		displayPath := candidate.File.Path
		if displayPath == "" {
			displayPath = path
		}
		info, err := fileInfo(path, displayPath)
		if err != nil {
			return nil, err
		}
		if candidate.File.SHA256 != "" && info.SHA256 != candidate.File.SHA256 {
			return nil, fmt.Errorf("%s: sha256 mismatch: selection=%s actual=%s", candidate.ManifestPath, candidate.File.SHA256, info.SHA256)
		}
		if candidate.File.Bytes > 0 && info.Bytes != candidate.File.Bytes {
			return nil, fmt.Errorf("%s: byte length mismatch: selection=%d actual=%d", candidate.ManifestPath, candidate.File.Bytes, info.Bytes)
		}
		fileSummary, err := analyzeFile(path, info, coverage)
		if err != nil {
			return nil, err
		}
		fileSummaries = append(fileSummaries, fileSummary)
		for _, construct := range fileSummary.Constructs {
			addConstruct(observed, construct, fileSummary.Path)
			if !supportedConstructs[construct] {
				addConstruct(unsupported, construct, fileSummary.Path)
			}
		}
		for _, gap := range fileSummary.UnsupportedGrammarGaps {
			addGrammarGap(unsupportedGrammarGaps, gap, fileSummary.Path)
		}
	}

	report := &corpusReport{
		Tool:            "go_syntax_corpus_check",
		Version:         reportVersion,
		ArtifactKind:    artifactKind,
		Status:          "ready",
		SelectionReport: &reportInfo,
		CorpusID:        selection.CorpusID,
		SourceRoot:      normalizeSelectionSourceRoot(selection.SourceRoot),
		EntryCount:      len(selection.SelectedCandidates),
		MetadataProjector: metadataProjector{
			Command: metadataProjectorCommand,
			Role:    "parser-backed checked fixture projector that defines current Go syntax route coverage",
		},
		GrammarCoverage:         coverage.summary,
		SupportedRouteFixtures:  append([]string(nil), supportedRouteFixtures...),
		SupportedConstructs:     sortedSupportedConstructs(),
		ObservedConstructs:      sortedConstructCounts(observed),
		UnsupportedConstructs:   sortedConstructCounts(unsupported),
		UnsupportedGrammarGaps:  sortedGrammarGaps(unsupportedGrammarGaps),
		Files:                   fileSummaries,
		NoStablePromotionClaim:  noStablePromotionClaim,
		StablePromotionBlockers: []string{"real_source_end_to_end_generated_codec_round_trip"},
	}
	if len(report.UnsupportedConstructs) > 0 {
		report.Status = "ready_with_unsupported_constructs"
		report.NextMissingConstruct = selectNextMissingConstruct(unsupported)
	}
	if len(report.UnsupportedGrammarGaps) > 0 {
		report.Status = "ready_with_unsupported_grammar_gaps"
		report.NextMissingGrammarGap = selectNextGrammarGap(unsupportedGrammarGaps)
	}
	return report, nil
}

func loadManifest(path string) (*corpusManifest, error) {
	contents, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read manifest: %w", err)
	}
	var manifest corpusManifest
	if err := json.Unmarshal(contents, &manifest); err != nil {
		return nil, fmt.Errorf("parse manifest: %w", err)
	}
	return &manifest, nil
}

func loadSelectionReport(path string) (*selectionReport, error) {
	contents, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read selection report: %w", err)
	}
	var report selectionReport
	if err := json.Unmarshal(contents, &report); err != nil {
		return nil, fmt.Errorf("parse selection report: %w", err)
	}
	return &report, nil
}

func loadGrammarCoverage(metadataRoot string) (grammarCoverage, error) {
	coverage := grammarCoverage{
		summary: grammarCoverageSummary{
			Source:       "checked_go_syntax_metadata",
			MetadataRoot: filepath.ToSlash(metadataRoot),
		},
		nodeKinds:    map[string]bool{},
		fields:       map[string]bool{},
		payloadKinds: map[string]bool{},
	}
	if metadataRoot == "" {
		return coverage, errors.New("--metadata-root is required")
	}
	err := filepath.WalkDir(metadataRoot, func(path string, entry os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if entry.IsDir() || filepath.Base(path) != "syntax-metadata-v0.json" {
			return nil
		}
		contents, err := os.ReadFile(path)
		if err != nil {
			return fmt.Errorf("read metadata fixture %s: %w", path, err)
		}
		var metadata syntaxMetadata
		if err := json.Unmarshal(contents, &metadata); err != nil {
			return fmt.Errorf("parse metadata fixture %s: %w", path, err)
		}
		coverage.summary.MetadataFixtureCount++
		for _, rule := range metadata.Rules {
			nodeKind := strings.TrimSpace(rule.NodeKind)
			if nodeKind == "" {
				nodeKind = strings.TrimSpace(rule.Name)
			}
			if nodeKind == "" {
				continue
			}
			coverage.nodeKinds[nodeKind] = true
			for _, field := range rule.Fields {
				fieldName := strings.TrimSpace(field.Name)
				if fieldName == "" {
					continue
				}
				coverage.fields[nodeKind+"."+fieldName] = true
				for _, alternative := range field.Alternatives {
					terminal := strings.TrimSpace(alternative.Terminal)
					if strings.HasSuffix(terminal, "_payload") {
						coverage.payloadKinds[terminal] = true
					}
				}
			}
		}
		return nil
	})
	if err != nil {
		return coverage, fmt.Errorf("walk metadata root %s: %w", metadataRoot, err)
	}
	if coverage.summary.MetadataFixtureCount == 0 {
		return coverage, fmt.Errorf("metadata root %s contains no syntax-metadata-v0.json fixtures", metadataRoot)
	}
	coverage.summary.NodeKindCount = len(coverage.nodeKinds)
	coverage.summary.GrammarFieldCount = len(coverage.fields)
	coverage.summary.PayloadKindCount = len(coverage.payloadKinds)
	return coverage, nil
}

func validateManifestShape(manifest *corpusManifest) error {
	if manifest.ArtifactKind != corpusManifestKind {
		return fmt.Errorf("manifest artifact_kind must be %q", corpusManifestKind)
	}
	if manifest.Version != 1 {
		return fmt.Errorf("manifest version must be 1")
	}
	if manifest.CorpusID == "" {
		return errors.New("manifest corpus_id is required")
	}
	if manifest.SourceRoot == "" {
		return errors.New("manifest source_root is required")
	}
	if manifest.SourceManifest == "" {
		return errors.New("manifest source_manifest is required")
	}
	if manifest.RouteSourceManifestShape.Tool != "route_source_manifest" {
		return errors.New("manifest must record route_source_manifest tool provenance")
	}
	if manifest.RouteSourceManifestShape.Target != "go-syntax" {
		return errors.New("manifest must use the go-syntax route source target")
	}
	if manifest.RouteSourceManifestShape.Marker != "tct-go-syntax-route-corpus-v1" {
		return errors.New("manifest has wrong route source marker")
	}
	if manifest.RouteSourceManifestShape.RouteDecisionMarker != "tct-go-syntax-route-corpus-v1" {
		return errors.New("manifest has wrong route decision marker")
	}
	if manifest.NoStablePromotionClaim != noStablePromotionClaim {
		return errors.New("manifest must preserve sidecar-only no-stable-promotion claim")
	}
	if len(manifest.Entries) < manifest.RouteSourceManifestShape.MinimumEntryCount {
		return errors.New("manifest entry count is below the recorded minimum")
	}
	if manifest.Selection.ExpectedEntryCount != len(manifest.Entries) {
		return errors.New("selection expected_entry_count does not match entries")
	}
	if manifest.Selection.LicenseTextPath == "" {
		return errors.New("selection license_text_path is required")
	}
	if manifest.RouteSourceManifestShape.CorpusFingerprint != corpusFingerprint(manifest.Entries) {
		return fmt.Errorf(
			"manifest corpus fingerprint mismatch: manifest=%s actual=%s",
			manifest.RouteSourceManifestShape.CorpusFingerprint,
			corpusFingerprint(manifest.Entries),
		)
	}
	seen := map[string]bool{}
	for _, entry := range manifest.Entries {
		if entry.ManifestPath == "" {
			return errors.New("manifest entry missing manifest_path")
		}
		if seen[entry.ManifestPath] {
			return fmt.Errorf("duplicate manifest entry: %s", entry.ManifestPath)
		}
		seen[entry.ManifestPath] = true
		if entry.File.Path != filepath.ToSlash(filepath.Join(manifest.SourceRoot, filepath.FromSlash(entry.ManifestPath))) {
			return fmt.Errorf("%s: file path must match source_root and manifest_path", entry.ManifestPath)
		}
		if entry.File.SHA256 == "" {
			return fmt.Errorf("%s: sha256 is required", entry.ManifestPath)
		}
		if entry.File.Bytes <= 0 {
			return fmt.Errorf("%s: byte count is required", entry.ManifestPath)
		}
		if entry.License == "" {
			return fmt.Errorf("%s: license is required", entry.ManifestPath)
		}
		if entry.SourceURI == "" {
			return fmt.Errorf("%s: source_uri is required", entry.ManifestPath)
		}
	}
	return nil
}

func readSourceManifest(path string) ([]string, error) {
	contents, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read source manifest: %w", err)
	}
	lines := strings.Split(strings.TrimSpace(string(contents)), "\n")
	if len(lines) == 0 || lines[0] != sourceManifestMarker {
		return nil, fmt.Errorf("source manifest must start with %q", sourceManifestMarker)
	}
	var entries []string
	for _, line := range lines[1:] {
		trimmed := strings.TrimSpace(line)
		if trimmed != "" {
			entries = append(entries, trimmed)
		}
	}
	return entries, nil
}

func validateSourceManifest(sourceList []string, manifest *corpusManifest, sourceManifestInfo fileArtifact) error {
	if sourceManifestInfo.SHA256 != manifest.RouteSourceManifestShape.SourceListSHA256 {
		return fmt.Errorf(
			"source manifest sha256 mismatch: manifest=%s actual=%s",
			manifest.RouteSourceManifestShape.SourceListSHA256,
			sourceManifestInfo.SHA256,
		)
	}
	if len(sourceList) != len(manifest.Entries) {
		return fmt.Errorf("source manifest has %d entries but corpus manifest has %d", len(sourceList), len(manifest.Entries))
	}
	for index, entry := range manifest.Entries {
		if sourceList[index] != entry.ManifestPath {
			return fmt.Errorf("source manifest entry %d mismatch: %s != %s", index, sourceList[index], entry.ManifestPath)
		}
	}
	return nil
}

func validateExpectedNextGap(expected string, nextConstruct *constructCount, nextGrammarGap *grammarGapCount) error {
	if expected == "" {
		if nextConstruct != nil {
			return fmt.Errorf("expected no missing construct, got %s", nextConstruct.Construct)
		}
		return nil
	}
	if nextConstruct != nil && nextConstruct.Construct == expected {
		return nil
	}
	if nextGrammarGap != nil && grammarGapMatchesExpected(*nextGrammarGap, expected) {
		return nil
	}
	if nextConstruct == nil && nextGrammarGap == nil {
		return fmt.Errorf("expected next missing construct %q, got none", expected)
	}
	return fmt.Errorf("expected next missing construct %q, got %q", expected, nextGapName(nextConstruct, nextGrammarGap))
}

func grammarGapMatchesExpected(gap grammarGapCount, expected string) bool {
	return gap.GrammarCategory == expected ||
		gap.ASTNodeKind == expected ||
		gap.GrammarField == expected ||
		gap.PayloadKind == expected ||
		gap.Construct == expected ||
		grammarGapExpectationName(gap) == expected
}

func grammarGapExpectationName(gap grammarGapCount) string {
	if gap.GrammarField != "" {
		return gap.GrammarField
	}
	if gap.ASTNodeKind != "" {
		return gap.ASTNodeKind
	}
	if gap.PayloadKind != "" {
		return gap.PayloadKind
	}
	if gap.Construct != "" {
		return gap.Construct
	}
	return gap.GrammarCategory
}

func nextGapName(nextConstruct *constructCount, nextGrammarGap *grammarGapCount) string {
	if nextConstruct != nil {
		return nextConstruct.Construct
	}
	if nextGrammarGap != nil {
		return grammarGapExpectationName(*nextGrammarGap)
	}
	return ""
}

func analyzeFile(path string, info fileArtifact, coverage grammarCoverage) (fileConstructSummary, error) {
	fset := token.NewFileSet()
	parsed, err := parser.ParseFile(fset, path, nil, parser.ParseComments)
	if err != nil {
		return fileConstructSummary{}, fmt.Errorf("%s: parse Go source: %w", path, err)
	}
	constructs := collectConstructs(parsed)
	unsupported := make([]string, 0)
	for _, construct := range constructs {
		if !supportedConstructs[construct] {
			unsupported = append(unsupported, construct)
		}
	}
	grammarGaps := collectUnsupportedGrammarGaps(parsed, coverage, info.Path)
	return fileConstructSummary{
		Path:                   info.Path,
		SHA256:                 info.SHA256,
		Bytes:                  info.Bytes,
		Package:                parsed.Name.Name,
		Constructs:             constructs,
		UnsupportedConstructs:  unsupported,
		UnsupportedGrammarGaps: grammarGaps,
	}, nil
}

func collectUnsupportedGrammarGaps(file *ast.File, coverage grammarCoverage, path string) []grammarGapCount {
	gaps := map[string]*grammarGapCount{}
	recordNode := func(nodeKind string) {
		if nodeKind == "" || coverage.nodeKinds[nodeKind] {
			return
		}
		addGrammarGap(gaps, grammarGapCount{
			GrammarCategory:     grammarCategoryForNodeKind(nodeKind),
			InteractionCategory: "grammar_node_kind",
			ASTNodeKind:         nodeKind,
		}, path)
	}
	recordField := func(nodeKind string, fieldName string, present bool) {
		if !present {
			return
		}
		field := nodeKind + "." + fieldName
		if coverage.fields[field] {
			return
		}
		addGrammarGap(gaps, grammarGapCount{
			GrammarCategory:     grammarCategoryForField(nodeKind, fieldName),
			InteractionCategory: "grammar_field",
			ASTNodeKind:         nodeKind,
			GrammarField:        field,
		}, path)
	}
	recordPayload := func(nodeKind string, fieldName string, payloadKind string, present bool) {
		if !present || payloadKind == "" || coverage.payloadKinds[payloadKind] {
			return
		}
		addGrammarGap(gaps, grammarGapCount{
			GrammarCategory:     grammarCategoryForField(nodeKind, fieldName),
			InteractionCategory: "terminal_payload",
			ASTNodeKind:         nodeKind,
			GrammarField:        nodeKind + "." + fieldName,
			PayloadKind:         payloadKind,
		}, path)
	}

	recordNode("File")
	recordField("File", "Name", file.Name != nil)
	recordField("File", "Decls", len(file.Decls) > 0)
	recordField("File", "Imports", len(file.Imports) > 0)
	recordField("File", "Comments", len(file.Comments) > 0)
	for _, group := range file.Comments {
		recordNode("CommentGroup")
		recordField("CommentGroup", "List", len(group.List) > 0)
		for _, comment := range group.List {
			recordNode("Comment")
			recordField("Comment", "Text", comment.Text != "")
			recordPayload("Comment", "Text", "string_payload", comment.Text != "")
		}
	}

	ast.Inspect(file, func(node ast.Node) bool {
		if node == nil {
			return true
		}
		nodeKind := astNodeKind(node)
		recordNode(nodeKind)
		switch typed := node.(type) {
		case *ast.GenDecl:
			recordField("GenDecl", "Doc", typed.Doc != nil)
			recordField("GenDecl", "Tok", typed.Tok.IsLiteral() || typed.Tok.IsKeyword())
			recordPayload("GenDecl", "Tok", "string_payload", typed.Tok.IsLiteral() || typed.Tok.IsKeyword())
			recordField("GenDecl", "Specs", len(typed.Specs) > 0)
		case *ast.ImportSpec:
			recordField("ImportSpec", "Name", typed.Name != nil)
			recordField("ImportSpec", "Path", typed.Path != nil)
			recordPayload("ImportSpec", "Path", "string_payload", typed.Path != nil)
			recordField("ImportSpec", "Doc", typed.Doc != nil)
			recordField("ImportSpec", "Comment", typed.Comment != nil)
		case *ast.ValueSpec:
			recordField("ValueSpec", "Doc", typed.Doc != nil)
			recordField("ValueSpec", "Names", len(typed.Names) > 0)
			recordField("ValueSpec", "Type", typed.Type != nil)
			recordField("ValueSpec", "Values", len(typed.Values) > 0)
			recordField("ValueSpec", "Comment", typed.Comment != nil)
		case *ast.TypeSpec:
			recordField("TypeSpec", "Doc", typed.Doc != nil)
			recordField("TypeSpec", "Name", typed.Name != nil)
			recordField("TypeSpec", "TypeParams", typed.TypeParams != nil && len(typed.TypeParams.List) > 0)
			recordField("TypeSpec", "Assign", typed.Assign.IsValid())
			recordPayload("TypeSpec", "Assign", "string_payload", typed.Assign.IsValid())
			recordField("TypeSpec", "Type", typed.Type != nil)
			recordField("TypeSpec", "Comment", typed.Comment != nil)
		case *ast.FuncDecl:
			recordField("FuncDecl", "Doc", typed.Doc != nil)
			recordField("FuncDecl", "Recv", typed.Recv != nil)
			recordField("FuncDecl", "Name", typed.Name != nil)
			recordPayload("FuncDecl", "Name", "identifier_payload", typed.Name != nil)
			recordField("FuncDecl", "Type", typed.Type != nil)
			recordField("FuncDecl", "Body", typed.Body != nil)
		case *ast.FuncType:
			recordField("FuncType", "TypeParams", typed.TypeParams != nil && len(typed.TypeParams.List) > 0)
			recordField("FuncType", "Params", typed.Params != nil)
			recordField("FuncType", "Results", typed.Results != nil)
		case *ast.FieldList:
			recordField("FieldList", "List", len(typed.List) > 0)
		case *ast.Field:
			recordField("Field", "Doc", typed.Doc != nil)
			recordField("Field", "Names", len(typed.Names) > 0)
			recordField("Field", "Type", typed.Type != nil)
			recordField("Field", "Comment", typed.Comment != nil)
		case *ast.BlockStmt:
			recordField("BlockStmt", "List", len(typed.List) > 0)
		case *ast.DeclStmt:
			recordField("DeclStmt", "Decl", typed.Decl != nil)
		case *ast.ReturnStmt:
			recordField("ReturnStmt", "Results", len(typed.Results) > 0)
		case *ast.AssignStmt:
			recordField("AssignStmt", "Lhs", len(typed.Lhs) > 0)
			recordField("AssignStmt", "Tok", true)
			recordPayload("AssignStmt", "Tok", "string_payload", true)
			recordField("AssignStmt", "Rhs", len(typed.Rhs) > 0)
		case *ast.IfStmt:
			recordField("IfStmt", "Init", typed.Init != nil)
			recordField("IfStmt", "Cond", typed.Cond != nil)
			recordField("IfStmt", "Body", typed.Body != nil)
			recordField("IfStmt", "Else", typed.Else != nil)
		case *ast.ForStmt:
			recordField("ForStmt", "Init", typed.Init != nil)
			recordField("ForStmt", "Cond", typed.Cond != nil)
			recordField("ForStmt", "Post", typed.Post != nil)
			recordField("ForStmt", "Body", typed.Body != nil)
		case *ast.RangeStmt:
			recordField("RangeStmt", "Key", typed.Key != nil)
			recordField("RangeStmt", "Value", typed.Value != nil)
			recordField("RangeStmt", "Tok", true)
			recordPayload("RangeStmt", "Tok", "string_payload", true)
			recordField("RangeStmt", "X", typed.X != nil)
			recordField("RangeStmt", "Body", typed.Body != nil)
		case *ast.SwitchStmt:
			recordField("SwitchStmt", "Init", typed.Init != nil)
			recordField("SwitchStmt", "Tag", typed.Tag != nil)
			recordField("SwitchStmt", "Body", typed.Body != nil)
		case *ast.TypeSwitchStmt:
			recordField("TypeSwitchStmt", "Init", typed.Init != nil)
			recordField("TypeSwitchStmt", "Assign", typed.Assign != nil)
			recordField("TypeSwitchStmt", "Body", typed.Body != nil)
		case *ast.CaseClause:
			recordField("CaseClause", "List", len(typed.List) > 0)
			recordField("CaseClause", "Body", len(typed.Body) > 0)
		case *ast.CommClause:
			recordField("CommClause", "Comm", typed.Comm != nil)
			recordField("CommClause", "Body", len(typed.Body) > 0)
		case *ast.SelectStmt:
			recordField("SelectStmt", "Body", typed.Body != nil)
		case *ast.BranchStmt:
			recordField("BranchStmt", "Tok", true)
			recordPayload("BranchStmt", "Tok", "string_payload", true)
			recordField("BranchStmt", "Label", typed.Label != nil)
		case *ast.GoStmt:
			recordField("GoStmt", "Call", typed.Call != nil)
		case *ast.DeferStmt:
			recordField("DeferStmt", "Call", typed.Call != nil)
		case *ast.SendStmt:
			recordField("SendStmt", "Chan", typed.Chan != nil)
			recordField("SendStmt", "Value", typed.Value != nil)
		case *ast.LabeledStmt:
			recordField("LabeledStmt", "Label", typed.Label != nil)
			recordField("LabeledStmt", "Stmt", typed.Stmt != nil)
		case *ast.ExprStmt:
			recordField("ExprStmt", "X", typed.X != nil)
		case *ast.IncDecStmt:
			recordField("IncDecStmt", "X", typed.X != nil)
			recordField("IncDecStmt", "Tok", true)
			recordPayload("IncDecStmt", "Tok", "string_payload", true)
		case *ast.CallExpr:
			recordField("CallExpr", "Fun", typed.Fun != nil)
			recordField("CallExpr", "Args", len(typed.Args) > 0)
			recordField("CallExpr", "Ellipsis", typed.Ellipsis.IsValid())
		case *ast.SelectorExpr:
			recordField("SelectorExpr", "X", typed.X != nil)
			recordField("SelectorExpr", "Sel", typed.Sel != nil)
		case *ast.Ident:
			recordField("Ident", "Name", typed.Name != "")
			recordPayload("Ident", "Name", "identifier_payload", typed.Name != "")
		case *ast.BasicLit:
			recordField("BasicLit", "Kind", true)
			recordPayload("BasicLit", "Kind", "string_payload", true)
			recordField("BasicLit", "Value", typed.Value != "")
			recordPayload("BasicLit", "Value", payloadKindForBasicLit(typed.Kind), typed.Value != "")
		case *ast.ParenExpr:
			recordField("ParenExpr", "X", typed.X != nil)
		case *ast.UnaryExpr:
			recordField("UnaryExpr", "Op", true)
			recordPayload("UnaryExpr", "Op", "string_payload", true)
			recordField("UnaryExpr", "X", typed.X != nil)
		case *ast.BinaryExpr:
			recordField("BinaryExpr", "X", typed.X != nil)
			recordField("BinaryExpr", "Op", true)
			recordPayload("BinaryExpr", "Op", "string_payload", true)
			recordField("BinaryExpr", "Y", typed.Y != nil)
		case *ast.FuncLit:
			recordField("FuncLit", "Type", typed.Type != nil)
			recordField("FuncLit", "Body", typed.Body != nil)
		case *ast.CompositeLit:
			recordField("CompositeLit", "Type", typed.Type != nil)
			recordField("CompositeLit", "Elts", len(typed.Elts) > 0)
		case *ast.KeyValueExpr:
			recordField("KeyValueExpr", "Key", typed.Key != nil)
			recordField("KeyValueExpr", "Value", typed.Value != nil)
		case *ast.ArrayType:
			recordField("ArrayType", "Len", typed.Len != nil)
			recordField("ArrayType", "Elt", typed.Elt != nil)
		case *ast.StructType:
			recordField("StructType", "Fields", typed.Fields != nil)
		case *ast.InterfaceType:
			recordField("InterfaceType", "Methods", typed.Methods != nil)
		case *ast.MapType:
			recordField("MapType", "Key", typed.Key != nil)
			recordField("MapType", "Value", typed.Value != nil)
		case *ast.ChanType:
			recordField("ChanType", "Dir", true)
			recordPayload("ChanType", "Dir", "string_payload", true)
			recordField("ChanType", "Value", typed.Value != nil)
		case *ast.Ellipsis:
			recordField("Ellipsis", "Elt", typed.Elt != nil)
		case *ast.TypeAssertExpr:
			recordField("TypeAssertExpr", "X", typed.X != nil)
			recordField("TypeAssertExpr", "Type", typed.Type != nil)
		case *ast.StarExpr:
			recordField("StarExpr", "X", typed.X != nil)
		case *ast.IndexExpr:
			recordField("IndexExpr", "X", typed.X != nil)
			recordField("IndexExpr", "Index", typed.Index != nil)
		case *ast.IndexListExpr:
			recordField("IndexListExpr", "X", typed.X != nil)
			recordField("IndexListExpr", "Indices", len(typed.Indices) > 0)
		case *ast.SliceExpr:
			recordField("SliceExpr", "X", typed.X != nil)
			recordField("SliceExpr", "Low", typed.Low != nil)
			recordField("SliceExpr", "High", typed.High != nil)
			recordField("SliceExpr", "Max", typed.Max != nil)
			recordField("SliceExpr", "Slice3", typed.Slice3)
		}
		return true
	})
	return sortedGrammarGaps(gaps)
}

func astNodeKind(node ast.Node) string {
	switch node.(type) {
	case *ast.File:
		return "File"
	case *ast.GenDecl:
		return "GenDecl"
	case *ast.ImportSpec:
		return "ImportSpec"
	case *ast.ValueSpec:
		return "ValueSpec"
	case *ast.TypeSpec:
		return "TypeSpec"
	case *ast.FuncDecl:
		return "FuncDecl"
	case *ast.FuncType:
		return "FuncType"
	case *ast.FieldList:
		return "FieldList"
	case *ast.Field:
		return "Field"
	case *ast.BlockStmt:
		return "BlockStmt"
	case *ast.DeclStmt:
		return "DeclStmt"
	case *ast.ReturnStmt:
		return "ReturnStmt"
	case *ast.AssignStmt:
		return "AssignStmt"
	case *ast.IfStmt:
		return "IfStmt"
	case *ast.ForStmt:
		return "ForStmt"
	case *ast.RangeStmt:
		return "RangeStmt"
	case *ast.SwitchStmt:
		return "SwitchStmt"
	case *ast.TypeSwitchStmt:
		return "TypeSwitchStmt"
	case *ast.CaseClause:
		return "CaseClause"
	case *ast.CommClause:
		return "CommClause"
	case *ast.SelectStmt:
		return "SelectStmt"
	case *ast.BranchStmt:
		return "BranchStmt"
	case *ast.GoStmt:
		return "GoStmt"
	case *ast.DeferStmt:
		return "DeferStmt"
	case *ast.SendStmt:
		return "SendStmt"
	case *ast.LabeledStmt:
		return "LabeledStmt"
	case *ast.ExprStmt:
		return "ExprStmt"
	case *ast.IncDecStmt:
		return "IncDecStmt"
	case *ast.CallExpr:
		return "CallExpr"
	case *ast.SelectorExpr:
		return "SelectorExpr"
	case *ast.Ident:
		return "Ident"
	case *ast.BasicLit:
		return "BasicLit"
	case *ast.ParenExpr:
		return "ParenExpr"
	case *ast.UnaryExpr:
		return "UnaryExpr"
	case *ast.BinaryExpr:
		return "BinaryExpr"
	case *ast.FuncLit:
		return "FuncLit"
	case *ast.CompositeLit:
		return "CompositeLit"
	case *ast.KeyValueExpr:
		return "KeyValueExpr"
	case *ast.ArrayType:
		return "ArrayType"
	case *ast.StructType:
		return "StructType"
	case *ast.InterfaceType:
		return "InterfaceType"
	case *ast.MapType:
		return "MapType"
	case *ast.ChanType:
		return "ChanType"
	case *ast.Ellipsis:
		return "Ellipsis"
	case *ast.TypeAssertExpr:
		return "TypeAssertExpr"
	case *ast.StarExpr:
		return "StarExpr"
	case *ast.IndexExpr:
		return "IndexExpr"
	case *ast.IndexListExpr:
		return "IndexListExpr"
	case *ast.SliceExpr:
		return "SliceExpr"
	}
	return ""
}

func payloadKindForBasicLit(kind token.Token) string {
	switch kind {
	case token.INT:
		return "integer_payload"
	default:
		return "string_payload"
	}
}

func collectConstructs(file *ast.File) []string {
	seen := map[string]bool{"package_clause": true}
	topLevelDecls := map[*ast.GenDecl]bool{}
	for _, decl := range file.Decls {
		if genDecl, ok := decl.(*ast.GenDecl); ok {
			topLevelDecls[genDecl] = true
		}
	}
	ast.Inspect(file, func(node ast.Node) bool {
		switch typed := node.(type) {
		case nil:
			return true
		case *ast.GenDecl:
			switch typed.Tok {
			case token.IMPORT:
				seen["import_declaration"] = true
			case token.CONST:
				seen["const_declaration"] = true
			case token.TYPE:
				seen["type_declaration"] = true
			case token.VAR:
				if topLevelDecls[typed] || typed.Lparen.IsValid() {
					seen["var_declaration"] = true
				} else {
					seen["local_var_declaration"] = true
				}
			}
		case *ast.ImportSpec:
			seen["import_spec"] = true
		case *ast.TypeSpec:
			seen["type_spec"] = true
		case *ast.ValueSpec:
			seen["value_spec"] = true
		case *ast.FuncDecl:
			if typed.Recv == nil {
				seen["function_declaration"] = true
			} else {
				seen["method_declaration"] = true
			}
		case *ast.FuncType:
			seen["function_type"] = true
		case *ast.FieldList:
			seen["field_list"] = true
		case *ast.Field:
			seen["field"] = true
		case *ast.BlockStmt:
			seen["block_statement"] = true
		case *ast.DeclStmt:
			seen["declaration_statement"] = true
		case *ast.ReturnStmt:
			seen["return_statement"] = true
		case *ast.AssignStmt:
			switch typed.Tok {
			case token.DEFINE:
				seen["short_var_declaration"] = true
			case token.ASSIGN:
				seen["assignment_statement"] = true
			default:
				seen["compound_assignment_statement"] = true
			}
		case *ast.IfStmt:
			seen["if_statement"] = true
		case *ast.ForStmt:
			seen["for_statement"] = true
		case *ast.RangeStmt:
			seen["range_statement"] = true
		case *ast.SwitchStmt, *ast.TypeSwitchStmt:
			seen["switch_statement"] = true
		case *ast.CaseClause:
			seen["case_clause"] = true
		case *ast.BranchStmt:
			seen["branch_statement"] = true
		case *ast.GoStmt:
			seen["go_statement"] = true
		case *ast.DeferStmt:
			seen["defer_statement"] = true
		case *ast.SendStmt:
			seen["send_statement"] = true
		case *ast.SelectStmt:
			seen["select_statement"] = true
		case *ast.IncDecStmt:
			seen["increment_decrement_statement"] = true
		case *ast.CallExpr:
			seen["call_expression"] = true
		case *ast.SelectorExpr:
			seen["selector_expression"] = true
		case *ast.Ident:
			seen["identifier_expression"] = true
		case *ast.BasicLit:
			switch typed.Kind {
			case token.INT:
				seen["basic_lit_integer"] = true
			case token.STRING, token.CHAR:
				seen["basic_lit_string"] = true
			default:
				seen["basic_lit_other"] = true
			}
		case *ast.ParenExpr:
			seen["parenthesized_expression"] = true
		case *ast.UnaryExpr:
			seen["unary_expression"] = true
		case *ast.BinaryExpr:
			seen["binary_expression"] = true
		case *ast.FuncLit:
			seen["function_literal"] = true
		case *ast.CompositeLit:
			seen["composite_literal"] = true
		case *ast.KeyValueExpr:
			seen["key_value_expression"] = true
		case *ast.ArrayType:
			seen["array_type"] = true
		case *ast.StructType:
			seen["struct_type"] = true
		case *ast.InterfaceType:
			seen["interface_type"] = true
		case *ast.MapType:
			seen["map_type"] = true
		case *ast.ChanType:
			seen["channel_type"] = true
		case *ast.TypeAssertExpr:
			seen["type_assertion_expression"] = true
		case *ast.StarExpr:
			seen["star_expression"] = true
		case *ast.IndexExpr, *ast.IndexListExpr:
			seen["index_expression"] = true
		case *ast.SliceExpr:
			seen["slice_expression"] = true
		}
		return true
	})
	constructs := make([]string, 0, len(seen))
	for construct := range seen {
		constructs = append(constructs, construct)
	}
	sort.Strings(constructs)
	return constructs
}

func fileInfo(path string, displayPath string) (fileArtifact, error) {
	contents, err := os.ReadFile(path)
	if err != nil {
		return fileArtifact{}, fmt.Errorf("read %s: %w", path, err)
	}
	hash := sha256.Sum256(contents)
	return fileArtifact{
		Path:   filepath.ToSlash(displayPath),
		SHA256: hex.EncodeToString(hash[:]),
		Bytes:  int64(len(contents)),
	}, nil
}

func corpusFingerprint(entries []manifestEntry) string {
	hasher := sha256.New()
	for _, entry := range entries {
		hasher.Write([]byte(entry.ManifestPath))
		hasher.Write([]byte{0})
		hasher.Write([]byte(entry.File.SHA256))
		hasher.Write([]byte{'\n'})
	}
	return hex.EncodeToString(hasher.Sum(nil))
}

func addConstruct(counts map[string]*constructCount, construct string, path string) {
	record := counts[construct]
	if record == nil {
		record = &constructCount{Construct: construct}
		counts[construct] = record
	}
	record.Count++
	if !contains(record.Files, path) {
		record.Files = append(record.Files, path)
		sort.Strings(record.Files)
	}
}

func addGrammarGap(counts map[string]*grammarGapCount, gap grammarGapCount, path string) {
	if gap.GrammarCategory == "" {
		gap.GrammarCategory = "uncategorized"
	}
	key := strings.Join([]string{
		gap.GrammarCategory,
		gap.InteractionCategory,
		gap.ASTNodeKind,
		gap.GrammarField,
		gap.PayloadKind,
		gap.Construct,
	}, "\x00")
	record := counts[key]
	if record == nil {
		record = &grammarGapCount{
			GrammarCategory:     gap.GrammarCategory,
			InteractionCategory: gap.InteractionCategory,
			ASTNodeKind:         gap.ASTNodeKind,
			GrammarField:        gap.GrammarField,
			PayloadKind:         gap.PayloadKind,
			Construct:           gap.Construct,
		}
		counts[key] = record
	}
	record.Count++
	if !contains(record.Files, path) {
		record.Files = append(record.Files, path)
		sort.Strings(record.Files)
	}
}

func sortedConstructCounts(counts map[string]*constructCount) []constructCount {
	values := make([]constructCount, 0, len(counts))
	for _, count := range counts {
		values = append(values, *count)
	}
	sort.Slice(values, func(i, j int) bool {
		if values[i].Construct == values[j].Construct {
			return values[i].Count > values[j].Count
		}
		return values[i].Construct < values[j].Construct
	})
	return values
}

func sortedSupportedConstructs() []string {
	constructs := make([]string, 0, len(supportedConstructs))
	for construct := range supportedConstructs {
		constructs = append(constructs, construct)
	}
	sort.Strings(constructs)
	return constructs
}

func sortedGrammarGaps(counts map[string]*grammarGapCount) []grammarGapCount {
	values := make([]grammarGapCount, 0, len(counts))
	for _, count := range counts {
		values = append(values, *count)
	}
	sort.Slice(values, func(i, j int) bool {
		leftCategory := grammarCategoryRank(values[i].GrammarCategory)
		rightCategory := grammarCategoryRank(values[j].GrammarCategory)
		if leftCategory != rightCategory {
			return leftCategory < rightCategory
		}
		leftInteraction := grammarInteractionRank(values[i].InteractionCategory)
		rightInteraction := grammarInteractionRank(values[j].InteractionCategory)
		if leftInteraction != rightInteraction {
			return leftInteraction < rightInteraction
		}
		if values[i].ASTNodeKind != values[j].ASTNodeKind {
			return values[i].ASTNodeKind < values[j].ASTNodeKind
		}
		if values[i].GrammarField != values[j].GrammarField {
			return values[i].GrammarField < values[j].GrammarField
		}
		if values[i].PayloadKind != values[j].PayloadKind {
			return values[i].PayloadKind < values[j].PayloadKind
		}
		if values[i].Construct != values[j].Construct {
			return values[i].Construct < values[j].Construct
		}
		return values[i].Count > values[j].Count
	})
	return values
}

func selectNextMissingConstruct(counts map[string]*constructCount) *constructCount {
	for _, construct := range nextConstructPriority {
		if count := counts[construct]; count != nil {
			selected := *count
			return &selected
		}
	}
	sorted := sortedConstructCounts(counts)
	if len(sorted) == 0 {
		return nil
	}
	return &sorted[0]
}

func selectNextGrammarGap(counts map[string]*grammarGapCount) *grammarGapCount {
	sorted := sortedGrammarGaps(counts)
	if len(sorted) == 0 {
		return nil
	}
	selected := sorted[0]
	return &selected
}

func grammarCategoryRank(category string) int {
	for index, candidate := range grammarCategoryOrder {
		if candidate == category {
			return index
		}
	}
	return len(grammarCategoryOrder)
}

func grammarInteractionRank(interaction string) int {
	switch interaction {
	case "grammar_node_kind":
		return 0
	case "grammar_field":
		return 1
	case "terminal_payload":
		return 2
	default:
		return 3
	}
}

func grammarCategoryForField(nodeKind string, fieldName string) string {
	if fieldName == "TypeParams" || nodeKind == "IndexListExpr" {
		return "types_and_generics"
	}
	if nodeKind == "Comment" || nodeKind == "CommentGroup" || fieldName == "Comments" || fieldName == "Doc" || fieldName == "Comment" {
		return "comments_directives_build_tags"
	}
	return grammarCategoryForNodeKind(nodeKind)
}

func grammarCategoryForNodeKind(nodeKind string) string {
	switch nodeKind {
	case "File", "GenDecl", "ImportSpec", "TypeSpec", "ValueSpec":
		return "packages_imports_declarations"
	case "FuncDecl", "FuncType", "FieldList", "Field":
		return "functions_methods"
	case "BlockStmt", "DeclStmt", "ReturnStmt", "AssignStmt", "IfStmt", "ForStmt", "RangeStmt", "SwitchStmt", "TypeSwitchStmt", "CaseClause", "CommClause", "SelectStmt", "BranchStmt", "GoStmt", "DeferStmt", "SendStmt", "LabeledStmt", "ExprStmt", "IncDecStmt":
		return "statements_control_flow"
	case "CallExpr", "SelectorExpr", "Ident", "ParenExpr", "UnaryExpr", "BinaryExpr", "IndexExpr", "IndexListExpr", "SliceExpr", "StarExpr", "TypeAssertExpr":
		if nodeKind == "IndexListExpr" {
			return "types_and_generics"
		}
		return "expressions_and_precedence"
	case "ArrayType", "StructType", "InterfaceType", "MapType", "ChanType", "Ellipsis":
		return "types_and_generics"
	case "BasicLit", "FuncLit", "CompositeLit", "KeyValueExpr":
		return "literals_and_composites"
	case "Comment", "CommentGroup":
		return "comments_directives_build_tags"
	default:
		return "uncategorized"
	}
}

func normalizeSelectionSourceRoot(sourceRoot string) string {
	if strings.Contains(sourceRoot, "$(go env GOROOT)") {
		return strings.ReplaceAll(sourceRoot, "$(go env GOROOT)", runtime.GOROOT())
	}
	return filepath.ToSlash(sourceRoot)
}

func normalizeSelectionSourcePath(sourcePath string, sourceRoot string, manifestPath string) string {
	normalizedPath := sourcePath
	if normalizedPath == "" {
		normalizedPath = filepath.ToSlash(filepath.Join(normalizeSelectionSourceRoot(sourceRoot), filepath.FromSlash(manifestPath)))
	}
	if strings.Contains(normalizedPath, "$(go env GOROOT)") {
		normalizedPath = strings.ReplaceAll(normalizedPath, "$(go env GOROOT)", runtime.GOROOT())
	}
	return filepath.FromSlash(normalizedPath)
}

func contains(values []string, expected string) bool {
	for _, value := range values {
		if value == expected {
			return true
		}
	}
	return false
}

func runSelfTest() error {
	tempDir, err := os.MkdirTemp("", "tct-go-syntax-corpus-check-")
	if err != nil {
		return err
	}
	defer os.RemoveAll(tempDir)
	sourceRoot := filepath.Join(tempDir, "src")
	if err := os.MkdirAll(sourceRoot, 0o755); err != nil {
		return err
	}
	sourcePath := filepath.Join(sourceRoot, "sample.go")
	source := []byte(`package sample

func SumPositive(values []int) int {
	total := values[0]
	for _, value := range values {
		if value > 0 {
			total = total + value
		}
	}
	return total
}

type Subscriber interface {
	Notify(chan int)
}

type Registry struct {
	subscribers map[string]Subscriber
}

func NotifyAsync(value any, done chan int) {
	var ch chan int
	subscriber := value.(Subscriber)
	defer subscriber.Notify(ch)
	go func() {
		ch <- 1
	}()
	select {
	case done <- 1:
	case <-ch:
	}
}
`)
	if err := os.WriteFile(sourcePath, source, 0o644); err != nil {
		return err
	}
	sourceHash := sha256.Sum256(source)
	sourceManifestPath := filepath.Join(tempDir, "sources.txt")
	if err := os.WriteFile(sourceManifestPath, []byte(sourceManifestMarker+"\nsample.go\n"), 0o644); err != nil {
		return err
	}
	sourceManifestInfo, err := fileInfo(sourceManifestPath, sourceManifestPath)
	if err != nil {
		return err
	}
	licensePath := filepath.Join(tempDir, "LICENSE")
	if err := os.WriteFile(licensePath, []byte("self-test license\n"), 0o644); err != nil {
		return err
	}
	manifestPath := filepath.Join(tempDir, "manifest-v1.json")
	entry := manifestEntry{
		ManifestPath: "sample.go",
		File: fileArtifact{
			Path:   filepath.ToSlash(sourcePath),
			SHA256: hex.EncodeToString(sourceHash[:]),
			Bytes:  int64(len(source)),
		},
		License:       "test-only",
		SourceURI:     "self-test",
		SelectionNote: "self-test",
	}
	manifest := corpusManifest{
		ArtifactKind:   corpusManifestKind,
		Version:        1,
		CorpusID:       "self-test",
		SourceRoot:     sourceRoot,
		SourceManifest: sourceManifestPath,
		RouteSourceManifestShape: routeSourceInfo{
			Tool:                "route_source_manifest",
			Version:             1,
			Target:              "go-syntax",
			Marker:              "tct-go-syntax-route-corpus-v1",
			CorpusFingerprint:   corpusFingerprint([]manifestEntry{entry}),
			SourceListSHA256:    sourceManifestInfo.SHA256,
			MinimumEntryCount:   1,
			FixtureScope:        true,
			RouteDecisionMarker: "tct-go-syntax-route-corpus-v1",
		},
		Selection: selectionInfo{
			Source:               "self-test",
			SourceRevision:       "self-test",
			License:              "test-only",
			LicenseTextPath:      filepath.ToSlash(licensePath),
			SelectionNote:        "self-test",
			ExpectedNextGap:      "",
			ExpectedEntryCount:   1,
			RepresentativeStatus: "synthetic_self_test",
		},
		Entries:                []manifestEntry{entry},
		NoStablePromotionClaim: noStablePromotionClaim,
	}
	rendered, err := json.MarshalIndent(manifest, "", "  ")
	if err != nil {
		return err
	}
	rendered = append(rendered, '\n')
	if err := os.WriteFile(manifestPath, rendered, 0o644); err != nil {
		return err
	}
	report, err := analyzeCorpus(manifestPath, defaultMetadataRoot)
	if err != nil {
		return err
	}
	if report.Status != "ready" {
		return fmt.Errorf("unexpected self-test status: %s", report.Status)
	}
	if report.NextMissingConstruct != nil {
		return fmt.Errorf("expected no self-test next gap, got %#v", report.NextMissingConstruct)
	}
	return nil
}

func exitf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}
