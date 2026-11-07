// go_syntax_source_instance_project emits parser-backed source-instance
// projection evidence for a Go syntax metadata fixture. It is a bridge report
// for whole-source generated-codec work: metadata already proves the schema
// shape, while this report proves that the selected source can be projected
// into reusable AST-field instance data before a generated-codec renderer
// consumes it.
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
	"regexp"
	"runtime"
	"sort"
	"strings"
)

const (
	sourceInstanceProjectionArtifactKind = "tct_go_syntax_source_instance_projection_v1"
	sourceInstanceReportArtifactKind     = "tct_go_syntax_source_instance_projection_report_v1"
	sourceInstanceTool                   = "go_syntax_source_instance_project"
	sourceInstanceVersion                = 1
	defaultNetLookupPlan9GrammarID       = "go_net_lookup_plan9_source_file_syntax_v0"
	defaultNetLookupPlan9MetadataFixture = "net-lookup-plan9-source-file"
	defaultReportIngestionManifestPath   = "tests/fixtures/go_syntax_corpus/training-report-ingestion-ten-record-v1.json"

	// Inventory-as-metadata mode: when --metadata names the checked go/ast
	// schema inventory fixture, its grammar_ir_rules drive the projection
	// directly so every source file projects against the one unified
	// inventory grammar instead of a per-file observed grammar.
	goAstSchemaInventoryArtifactKind = "tct_go_ast_schema_inventory_v1"
)

type sourceInstanceOptions struct {
	sourcePath                  string
	metadataPath                string
	metadataSmokeFixture        string
	manifestPath                string
	candidateGrammarID          string
	reportIngestionManifestPath string
	projectionOutput            string
	reportOutput                string
	validateReport              string
	routeRegistered             bool
	selfTest                    bool
	inventoryMetadata           bool
}

type syntaxMetadata struct {
	ArtifactKind string         `json:"artifact_kind"`
	Version      int            `json:"version"`
	Source       metadataSource `json:"source"`
	Parser       metadataParser `json:"parser"`
	Rules        []syntaxRule   `json:"rules"`
}

// goAstSchemaInventoryMetadata is the subset of the checked go/ast schema
// inventory fixture consumed in inventory-as-metadata mode. The inventory's
// grammar_ir_rules are shape-compatible with the per-file metadata rules
// (name/node_kind/fields with name/required/multiple/alternatives); extra
// keys are ignored by the decoder.
type goAstSchemaInventoryMetadata struct {
	ArtifactKind string                        `json:"artifact_kind"`
	Version      int                           `json:"version"`
	GrammarID    string                        `json:"grammar_id"`
	Go           goAstSchemaInventoryToolchain `json:"go"`
	Rules        []syntaxRule                  `json:"grammar_ir_rules"`
}

type goAstSchemaInventoryToolchain struct {
	Version       string `json:"version"`
	ParserPackage string `json:"parser_package"`
}

// decodeSyntaxMetadata decodes either a per-file syntax metadata fixture or
// the checked go/ast schema inventory fixture. In inventory mode the source
// block stays empty (the inventory is source-independent) and the parser
// block carries the pinned inventory toolchain, so checked-report parser
// provenance stays uniform with the per-file era.
func decodeSyntaxMetadata(metadataBytes []byte) (syntaxMetadata, bool, error) {
	var probe struct {
		ArtifactKind string `json:"artifact_kind"`
	}
	if err := json.Unmarshal(metadataBytes, &probe); err != nil {
		return syntaxMetadata{}, false, fmt.Errorf("decode metadata: %w", err)
	}
	if probe.ArtifactKind != goAstSchemaInventoryArtifactKind {
		var metadata syntaxMetadata
		if err := json.Unmarshal(metadataBytes, &metadata); err != nil {
			return syntaxMetadata{}, false, fmt.Errorf("decode metadata: %w", err)
		}
		return metadata, false, nil
	}
	var inventory goAstSchemaInventoryMetadata
	if err := json.Unmarshal(metadataBytes, &inventory); err != nil {
		return syntaxMetadata{}, false, fmt.Errorf("decode inventory metadata: %w", err)
	}
	if len(inventory.Rules) == 0 {
		return syntaxMetadata{}, false, errors.New("inventory metadata contains no grammar_ir_rules")
	}
	if inventory.Go.Version == "" || inventory.Go.ParserPackage == "" {
		return syntaxMetadata{}, false, errors.New("inventory metadata must pin the Go toolchain and parser package")
	}
	return syntaxMetadata{
		ArtifactKind: inventory.ArtifactKind,
		Version:      inventory.Version,
		Parser: metadataParser{
			Tool:           "go",
			Version:        inventory.Go.Version,
			Package:        inventory.Go.ParserPackage,
			PackageVersion: inventory.Go.Version,
		},
		Rules: inventory.Rules,
	}, true, nil
}

type metadataSource struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
}

type metadataParser struct {
	Tool           string `json:"tool"`
	Version        string `json:"version"`
	Package        string `json:"package"`
	PackageVersion string `json:"package_version"`
}

type syntaxRule struct {
	Name     string        `json:"name"`
	NodeKind string        `json:"node_kind"`
	Fields   []syntaxField `json:"fields"`
}

type syntaxField struct {
	Name         string              `json:"name"`
	Required     bool                `json:"required"`
	Multiple     bool                `json:"multiple"`
	Alternatives []syntaxAlternative `json:"alternatives"`
}

type syntaxAlternative struct {
	NodeKind string `json:"node_kind"`
	Terminal string `json:"terminal"`
}

type sourceInstanceProjection struct {
	ArtifactKind       string                 `json:"artifact_kind"`
	Version            int                    `json:"version"`
	Tool               string                 `json:"tool"`
	Status             string                 `json:"status"`
	Target             string                 `json:"target"`
	CandidateGrammarID string                 `json:"candidate_grammar_id"`
	ManifestPath       string                 `json:"manifest_path"`
	Source             projectionSource       `json:"source"`
	Metadata           projectionMetadata     `json:"metadata"`
	Parser             projectionParser       `json:"parser"`
	Root               projectionNode         `json:"root"`
	Stats              projectionStats        `json:"stats"`
	ClaimBoundary      string                 `json:"claim_boundary"`
	NoStableClaims     []string               `json:"no_stable_claims"`
	Notes              map[string]interface{} `json:"notes,omitempty"`
}

type projectionSource struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int    `json:"bytes"`
}

type projectionMetadata struct {
	Path         string `json:"path"`
	SHA256       string `json:"sha256"`
	ArtifactKind string `json:"artifact_kind"`
	Version      int    `json:"version"`
	RuleCount    int    `json:"rule_count"`
	FieldCount   int    `json:"field_count"`
}

type projectionParser struct {
	MetadataTool           string `json:"metadata_tool"`
	MetadataVersion        string `json:"metadata_version"`
	MetadataPackage        string `json:"metadata_package"`
	MetadataPackageVersion string `json:"metadata_package_version"`
	RuntimeGoVersion       string `json:"runtime_go_version"`
}

type projectionNode struct {
	NodeKind string            `json:"node_kind"`
	RuleName string            `json:"rule_name"`
	Fields   []projectionField `json:"fields"`
}

type projectionField struct {
	Name     string            `json:"name"`
	Required bool              `json:"required"`
	Multiple bool              `json:"multiple"`
	Terminal string            `json:"terminal,omitempty"`
	Values   []projectionValue `json:"values,omitempty"`
	Missing  bool              `json:"missing,omitempty"`
}

type projectionValue struct {
	Kind     string `json:"kind"`
	NodeKind string `json:"node_kind"`
	Terminal string `json:"terminal,omitempty"`
	// Payload is a pointer so terminal values always serialize a payload
	// (the empty string is a legal payload, e.g. File.GoVersion) while node
	// values omit the field entirely.
	Payload   *string         `json:"payload,omitempty"`
	TokenName string          `json:"token_name,omitempty"`
	Node      *projectionNode `json:"node,omitempty"`
}

func stringPtr(value string) *string {
	return &value
}

type projectionStats struct {
	NodeCount              int            `json:"node_count"`
	FieldCount             int            `json:"field_count"`
	FieldValueCount        int            `json:"field_value_count"`
	TerminalPayloadCount   int            `json:"terminal_payload_count"`
	OptionalMissingCount   int            `json:"optional_missing_count"`
	RepeatedEmptyCount     int            `json:"repeated_empty_count"`
	MaxDepth               int            `json:"max_depth"`
	DistinctNodeKinds      []string       `json:"distinct_node_kinds"`
	TerminalPayloadsByKind map[string]int `json:"terminal_payloads_by_kind"`
}

type sourceInstanceReport struct {
	ArtifactKind                string                     `json:"artifact_kind"`
	Version                     int                        `json:"version"`
	Tool                        string                     `json:"tool"`
	Status                      string                     `json:"status"`
	Target                      string                     `json:"target"`
	CandidateGrammarID          string                     `json:"candidate_grammar_id"`
	ManifestPath                string                     `json:"manifest_path"`
	ReportIngestionManifestPath string                     `json:"report_ingestion_manifest_path,omitempty"`
	Source                      projectionSource           `json:"source"`
	Metadata                    projectionMetadata         `json:"metadata"`
	Parser                      projectionParser           `json:"parser"`
	ProjectionOutput            string                     `json:"projection_output"`
	ProjectionSHA256            string                     `json:"projection_sha256"`
	ProjectionStats             projectionStats            `json:"projection_stats"`
	CanonicalRender             canonicalRenderEvidence    `json:"canonical_render"`
	CurrentEvidence             sourceRouteCurrentEvidence `json:"current_evidence"`
	CurrentBlockers             []sourceRouteBlocker       `json:"current_blockers"`
	NextConcreteTask            string                     `json:"next_concrete_task"`
	ValidationCommands          []string                   `json:"validation_commands"`
	ClaimBoundary               string                     `json:"claim_boundary"`
	NoStableClaims              []string                   `json:"no_stable_claims"`
}

type canonicalRenderEvidence struct {
	Formatter          string `json:"formatter"`
	Gofmt              string `json:"gofmt"`
	GoParser           string `json:"go_parser"`
	MatchesSource      bool   `json:"matches_source"`
	CanonicalSHA256    string `json:"canonical_sha256"`
	CanonicalByteCount int    `json:"canonical_byte_count"`
}

type sourceRouteCurrentEvidence struct {
	MetadataRulesProjected         int      `json:"metadata_rules_projected"`
	MetadataFieldsProjected        int      `json:"metadata_fields_projected"`
	ProjectionNodeKindsCovered     []string `json:"projection_node_kinds_covered"`
	SourceInstanceProjectionReady  bool     `json:"source_instance_projection_ready"`
	GeneratedCodecReportReady      bool     `json:"generated_codec_report_ready"`
	ReusableRendererConsumerReady  bool     `json:"reusable_renderer_consumer_ready"`
	ReportIngestionReady           bool     `json:"report_ingestion_ready"`
	TrainingManifestPromotionReady bool     `json:"training_manifest_promotion_ready"`
}

type sourceRouteBlocker struct {
	Kind   string `json:"kind"`
	Path   string `json:"path"`
	Reason string `json:"reason"`
}

type projectionBuilder struct {
	rulesByNodeKind                    map[string]syntaxRule
	stats                              projectionStats
	nodeKinds                          map[string]struct{}
	fileSet                            *token.FileSet
	fileComments                       []*ast.CommentGroup
	attachedComments                   map[token.Pos]struct{}
	emptyClauseBodyCommentAnchors      map[token.Pos][]int
	ifStmtHeaderTrailingCommentAnchors map[token.Pos][]int
	ignoredCommentPositions            map[token.Pos]struct{}
}

func main() {
	options := parseSourceInstanceFlags(os.Args[1:])
	if options.selfTest {
		if err := runSourceInstanceSelfTest(); err != nil {
			fatal(err)
		}
		fmt.Println("go syntax source instance projection self-test passed")
		return
	}
	if options.sourcePath == "" || options.metadataPath == "" {
		fatal(errors.New("--source and --metadata are required unless --self-test is set"))
	}

	projection, report, err := buildSourceInstanceProjectionAndReport(options)
	if err != nil {
		fatal(err)
	}
	if options.validateReport != "" {
		if err := validateSourceInstanceReport(options.validateReport, report); err != nil {
			fatal(err)
		}
		return
	}
	if options.projectionOutput != "" {
		if err := writeJSON(options.projectionOutput, projection); err != nil {
			fatal(err)
		}
	}
	if options.reportOutput != "" {
		if err := writeJSON(options.reportOutput, report); err != nil {
			fatal(err)
		}
	}
	if options.projectionOutput == "" && options.reportOutput == "" {
		encoded, err := json.MarshalIndent(report, "", "  ")
		if err != nil {
			fatal(err)
		}
		fmt.Println(string(encoded))
	}
}

func parseSourceInstanceFlags(args []string) sourceInstanceOptions {
	flags := flag.NewFlagSet("go_syntax_source_instance_project", flag.ExitOnError)
	var options sourceInstanceOptions
	flags.StringVar(&options.sourcePath, "source", "", "Go source file to project")
	flags.StringVar(&options.metadataPath, "metadata", "", "checked Go syntax metadata fixture")
	flags.StringVar(&options.metadataSmokeFixture, "metadata-smoke-fixture", "", "go_syntax_metadata_smoke fixture id for validation command provenance")
	flags.StringVar(&options.manifestPath, "manifest-path", "", "corpus manifest path for the source")
	flags.StringVar(&options.candidateGrammarID, "candidate-grammar-id", defaultNetLookupPlan9GrammarID, "candidate generated-codec grammar id")
	flags.StringVar(&options.reportIngestionManifestPath, "report-ingestion-manifest", "", "report-ingestion manifest path to record in current blockers")
	flags.StringVar(&options.projectionOutput, "projection-output", "", "path for the full source-instance projection artifact")
	flags.StringVar(&options.reportOutput, "report-output", "", "path for the checked projection/report-support artifact")
	flags.StringVar(&options.validateReport, "validate-report", "", "validate an existing checked report against current inputs")
	flags.BoolVar(&options.routeRegistered, "route-registered", false, "record that the candidate grammar id is already registered in the Rust syntax codec registry")
	flags.BoolVar(&options.selfTest, "self-test", false, "run a synthetic self-test")
	_ = flags.Parse(args)
	return options
}

func buildSourceInstanceProjectionAndReport(options sourceInstanceOptions) (sourceInstanceProjection, sourceInstanceReport, error) {
	reportOutput := options.reportOutput
	if reportOutput == "" && options.validateReport != "" {
		reportOutput = options.validateReport
	}
	commandOptions := options
	commandOptions.reportOutput = reportOutput
	sourceBytes, err := os.ReadFile(options.sourcePath)
	if err != nil {
		return sourceInstanceProjection{}, sourceInstanceReport{}, fmt.Errorf("read source: %w", err)
	}
	metadataBytes, err := os.ReadFile(options.metadataPath)
	if err != nil {
		return sourceInstanceProjection{}, sourceInstanceReport{}, fmt.Errorf("read metadata: %w", err)
	}
	metadata, inventoryMode, err := decodeSyntaxMetadata(metadataBytes)
	if err != nil {
		return sourceInstanceProjection{}, sourceInstanceReport{}, err
	}
	commandOptions.inventoryMetadata = inventoryMode
	if len(metadata.Rules) == 0 {
		return sourceInstanceProjection{}, sourceInstanceReport{}, errors.New("metadata contains no rules")
	}
	// Pin integrity against the on-disk bytes before gofmt-canonical normalization.
	// TCT's Go token space is gofmt-canonical; non-gofmt stdlib residue (import
	// order, spacing, blank lines) is projected from format.Source form so
	// matches_source and token-only render compare against the same golden form.
	// When normalization rewrites bytes, write a working copy and point Source.Path
	// at it so Rust-side re-read hash checks stay consistent with reported sha256.
	originalSourceHash := sha256Hex(sourceBytes)
	if metadata.Source.SHA256 != "" && metadata.Source.SHA256 != originalSourceHash {
		return sourceInstanceProjection{}, sourceInstanceReport{}, fmt.Errorf("source SHA-256 drift: metadata has %s, actual is %s", metadata.Source.SHA256, originalSourceHash)
	}
	formattedSource, err := format.Source(sourceBytes)
	if err != nil {
		return sourceInstanceProjection{}, sourceInstanceReport{}, fmt.Errorf("gofmt-canonical normalize source: %w", err)
	}
	sourcePath := options.sourcePath
	if !bytes.Equal(sourceBytes, formattedSource) {
		canonicalPath, writeErr := writeGofmtCanonicalWorkingCopy(options, formattedSource)
		if writeErr != nil {
			return sourceInstanceProjection{}, sourceInstanceReport{}, writeErr
		}
		sourcePath = canonicalPath
	}
	sourceBytes = formattedSource
	sourceHash := sha256Hex(sourceBytes)
	fileSet := token.NewFileSet()
	file, err := parser.ParseFile(fileSet, sourcePath, sourceBytes, parser.ParseComments)
	if err != nil {
		return sourceInstanceProjection{}, sourceInstanceReport{}, fmt.Errorf("parse source: %w", err)
	}
	builder, err := newProjectionBuilder(metadata.Rules, fileSet, file, attachedCommentGroupPositions(file))
	if err != nil {
		return sourceInstanceProjection{}, sourceInstanceReport{}, err
	}
	root, err := builder.projectNode(reflect.ValueOf(file), 1)
	if err != nil {
		return sourceInstanceProjection{}, sourceInstanceReport{}, err
	}
	builder.finishStats()
	metadataFieldCount := 0
	for _, rule := range metadata.Rules {
		metadataFieldCount += len(rule.Fields)
	}
	manifestPath := options.manifestPath
	if manifestPath == "" {
		manifestPath = metadata.Source.Path
	}
	reportIngestionManifestPath := reportIngestionManifestPathForBlocker(options)
	projection := sourceInstanceProjection{
		ArtifactKind:       sourceInstanceProjectionArtifactKind,
		Version:            sourceInstanceVersion,
		Tool:               sourceInstanceTool,
		Status:             "source_instance_projection_ready_generated_codec_report_blocked",
		Target:             "go",
		CandidateGrammarID: options.candidateGrammarID,
		ManifestPath:       manifestPath,
		Source: projectionSource{
			Path:   sourcePath,
			SHA256: sourceHash,
			Bytes:  len(sourceBytes),
		},
		Metadata: projectionMetadata{
			Path:         options.metadataPath,
			SHA256:       sha256Hex(metadataBytes),
			ArtifactKind: metadata.ArtifactKind,
			Version:      metadata.Version,
			RuleCount:    len(metadata.Rules),
			FieldCount:   metadataFieldCount,
		},
		Parser: projectionParser{
			MetadataTool:           metadata.Parser.Tool,
			MetadataVersion:        metadata.Parser.Version,
			MetadataPackage:        metadata.Parser.Package,
			MetadataPackageVersion: metadata.Parser.PackageVersion,
			RuntimeGoVersion:       runtime.Version(),
		},
		Root:           root,
		Stats:          builder.stats,
		ClaimBoundary:  projectionClaimBoundary(),
		NoStableClaims: noStableSourceInstanceClaims(),
	}
	projectionBytes, err := json.MarshalIndent(projection, "", "  ")
	if err != nil {
		return sourceInstanceProjection{}, sourceInstanceReport{}, fmt.Errorf("encode projection: %w", err)
	}
	canonical, err := canonicalRenderEvidenceForSource(sourcePath, sourceBytes)
	if err != nil {
		return sourceInstanceProjection{}, sourceInstanceReport{}, err
	}
	currentBlockers := []sourceRouteBlocker{
		{
			Kind:   "generated_codec_report_blocked_by_missing_projection_consumer",
			Path:   "src/syntax_generated_codec.rs::render_go_syntax_codec_fixture",
			Reason: "the selected source now has parser-backed instance projection evidence, but generated Rust codec production still needs a reusable renderer that consumes projected AST field values instead of adding another grammar-id-specific fixture branch",
		},
	}
	if !options.routeRegistered {
		currentBlockers = append(currentBlockers, sourceRouteBlocker{
			Kind:   "generated_codec_route_unregistered",
			Path:   "src/syntax_production_route.rs::SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS",
			Reason: fmt.Sprintf("candidate grammar %q is not registered as a checked Go production route; registration should follow reusable renderer consumption, not precede it", options.candidateGrammarID),
		})
	}
	currentBlockers = append(currentBlockers, sourceRouteBlocker{
		Kind:   "report_ingestion_manifest_record_missing",
		Path:   reportIngestionManifestPath,
		Reason: fmt.Sprintf("no report-ingestion record can be added for %q until a generated-codec report exists with checked tokens, canonical render, gofmt, go/parser, source hash, and claim-boundary fields", manifestPath),
	})

	report := sourceInstanceReport{
		ArtifactKind:       sourceInstanceReportArtifactKind,
		Version:            sourceInstanceVersion,
		Tool:               sourceInstanceTool,
		Status:             "blocked_before_generated_codec_report",
		Target:             "go",
		CandidateGrammarID: options.candidateGrammarID,
		ManifestPath:       manifestPath,
		ReportIngestionManifestPath: func() string {
			if options.reportIngestionManifestPath != "" {
				return reportIngestionManifestPath
			}
			return ""
		}(),
		Source:           projection.Source,
		Metadata:         projection.Metadata,
		Parser:           projection.Parser,
		ProjectionOutput: options.projectionOutput,
		ProjectionSHA256: sha256Hex(projectionBytes),
		ProjectionStats:  projection.Stats,
		CanonicalRender:  canonical,
		CurrentEvidence: sourceRouteCurrentEvidence{
			MetadataRulesProjected:         len(metadata.Rules),
			MetadataFieldsProjected:        metadataFieldCount,
			ProjectionNodeKindsCovered:     projection.Stats.DistinctNodeKinds,
			SourceInstanceProjectionReady:  true,
			GeneratedCodecReportReady:      false,
			ReusableRendererConsumerReady:  false,
			ReportIngestionReady:           false,
			TrainingManifestPromotionReady: false,
		},
		CurrentBlockers: currentBlockers,
		NextConcreteTask: fmt.Sprintf(
			"consume the checked Go source-instance projection for %q in reusable generated-codec renderer/report machinery, then add paired next-version corpus and report-ingestion manifest records",
			manifestPath,
		),
		// Replay commands must use the same working path recorded in Source.Path
		// (gofmt-canonical copy when the on-disk input was rewritten).
		ValidationCommands: validationCommands(func() sourceInstanceOptions {
			commandOptions.sourcePath = sourcePath
			return commandOptions
		}()),
		ClaimBoundary:  projectionClaimBoundary(),
		NoStableClaims: noStableSourceInstanceClaims(),
	}
	return projection, report, nil
}

func newProjectionBuilder(rules []syntaxRule, fileSet *token.FileSet, file *ast.File, attachedComments map[token.Pos]struct{}) (*projectionBuilder, error) {
	rulesByNodeKind := make(map[string]syntaxRule, len(rules))
	for _, rule := range rules {
		if rule.NodeKind == "" {
			return nil, errors.New("metadata rule missing node_kind")
		}
		if _, exists := rulesByNodeKind[rule.NodeKind]; exists {
			return nil, fmt.Errorf("duplicate metadata node kind %q", rule.NodeKind)
		}
		rulesByNodeKind[rule.NodeKind] = rule
	}
	var fileComments []*ast.CommentGroup
	if file != nil {
		fileComments = file.Comments
	}
	emptyClauseBodyCommentAnchors, emptyClauseBodyCommentPositions :=
		emptyClauseBodyCommentAnchors(file, fileSet, fileComments, attachedComments)
	ifStmtHeaderTrailingAnchors := map[token.Pos][]int{}
	ifStmtHeaderTrailingCommentPositions := map[token.Pos]struct{}{}
	if ruleHasField(rulesByNodeKind["IfStmt"], "TrailingComment") {
		ifStmtHeaderTrailingAnchors, ifStmtHeaderTrailingCommentPositions =
			ifStmtHeaderTrailingCommentAnchors(file, fileSet, fileComments, attachedComments)
	}
	ignoredCommentPositions := mergeCommentPositionSets(
		emptyClauseBodyCommentPositions,
		ifStmtHeaderTrailingCommentPositions,
	)
	return &projectionBuilder{
		rulesByNodeKind: rulesByNodeKind,
		stats: projectionStats{
			TerminalPayloadsByKind: make(map[string]int),
		},
		nodeKinds:                          make(map[string]struct{}),
		fileSet:                            fileSet,
		fileComments:                       fileComments,
		attachedComments:                   attachedComments,
		emptyClauseBodyCommentAnchors:      emptyClauseBodyCommentAnchors,
		ifStmtHeaderTrailingCommentAnchors: ifStmtHeaderTrailingAnchors,
		ignoredCommentPositions:            ignoredCommentPositions,
	}, nil
}

func (builder *projectionBuilder) projectNode(value reflect.Value, depth int) (projectionNode, error) {
	value = unwrapInterfaceValue(value)
	if isNilReflectValue(value) {
		return projectionNode{}, errors.New("cannot project nil AST node")
	}
	node, ok := value.Interface().(ast.Node)
	if !ok {
		return projectionNode{}, fmt.Errorf("value %s does not implement ast.Node", value.Type())
	}
	nodeKind := astNodeKind(node)
	rule, ok := builder.rulesByNodeKind[nodeKind]
	if !ok {
		return projectionNode{}, fmt.Errorf("metadata missing rule for AST node kind %q", nodeKind)
	}
	builder.stats.NodeCount++
	if depth > builder.stats.MaxDepth {
		builder.stats.MaxDepth = depth
	}
	builder.nodeKinds[nodeKind] = struct{}{}
	nodeStruct := unwrapReflectValue(reflect.ValueOf(node))
	projected := projectionNode{
		NodeKind: nodeKind,
		RuleName: rule.Name,
		Fields:   make([]projectionField, 0, len(rule.Fields)),
	}
	for _, field := range rule.Fields {
		fieldValue, synthetic := builder.syntheticMetadataField(node, field.Name)
		if !synthetic {
			fieldValue = nodeStruct.FieldByName(field.Name)
			if !fieldValue.IsValid() {
				return projectionNode{}, fmt.Errorf("AST node %s missing metadata field %q", nodeKind, field.Name)
			}
		}
		projectedField, err := builder.projectField(fieldValue, field, depth+1)
		if err != nil {
			return projectionNode{}, fmt.Errorf("%s.%s: %w", nodeKind, field.Name, err)
		}
		projected.Fields = append(projected.Fields, projectedField)
	}
	return projected, nil
}

func (builder *projectionBuilder) syntheticMetadataField(node ast.Node, fieldName string) (reflect.Value, bool) {
	switch typed := node.(type) {
	case *ast.File:
		if fieldName == "Comments" {
			// Each comment's text is emitted exactly once. Groups attached to an
			// AST owner (a Doc or Comment field) render from that owner, so the
			// root channel keeps only their source-order SLOT as an empty marker
			// group. Unattached groups keep their text here, because the root
			// channel is their only home. The renderer distinguishes the two by
			// emptiness (GO_RAW_TREE_COMMENT_REFERENCE_POLICY).
			groups := make([]*ast.CommentGroup, 0, len(typed.Comments))
			for _, group := range typed.Comments {
				if group == nil {
					continue
				}
				if _, attached := builder.attachedComments[group.Pos()]; attached {
					// Marker group: same line count, blank text. The list stays
					// non-empty so grammars that mark CommentGroup.List required
					// still accept it, and the payload bytes are gone.
					marker := &ast.CommentGroup{List: make([]*ast.Comment, 0, len(group.List))}
					for range group.List {
						marker.List = append(marker.List, &ast.Comment{})
					}
					groups = append(groups, marker)
					continue
				}
				groups = append(groups, group)
			}
			return reflect.ValueOf(groups), true
		}
		if fieldName == "CommentBefore" {
			return reflect.ValueOf(fileDeclCommentBeforeIndexes(typed, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
		if fieldName == "NoBlankLineBefore" {
			return reflect.ValueOf(fileDeclNoBlankLineBeforeIndexes(typed, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
		if fieldName == "HeaderComments" {
			groups := make([]*ast.CommentGroup, 0)
			for _, group := range typed.Comments {
				if group != nil && group.End() < typed.Package {
					groups = append(groups, group)
				}
			}
			return reflect.ValueOf(groups), true
		}
		if fieldName == "Imports" {
			declarations := make([]*ast.GenDecl, 0)
			for _, declaration := range typed.Decls {
				if group, ok := declaration.(*ast.GenDecl); ok && group.Tok == token.IMPORT {
					declarations = append(declarations, group)
				}
			}
			return reflect.ValueOf(declarations), true
		}
		if fieldName == "CompareDecl" {
			for _, declaration := range typed.Decls {
				if function, ok := declaration.(*ast.FuncDecl); ok && function.Name != nil && function.Name.Name == "Compare" {
					return reflect.ValueOf(function), true
				}
			}
			return reflect.ValueOf((*ast.FuncDecl)(nil)), true
		}
	case *ast.FieldList:
		if fieldName == "BlankLineBefore" {
			return reflect.ValueOf(fieldListBlankLineBeforeIndexes(typed, builder.fileSet)), true
		}
	case *ast.CompositeLit:
		if fieldName == "LineBreakBefore" {
			return reflect.ValueOf(compositeLitLineBreakBeforeIndexes(typed, builder.fileSet)), true
		}
	case *ast.CallExpr:
		if fieldName == "LineBreakBefore" {
			return reflect.ValueOf(callExprLineBreakBeforeIndexes(typed, builder.fileSet)), true
		}
	case *ast.BlockStmt:
		if fieldName == "CommentBefore" {
			return reflect.ValueOf(blockStmtCommentBeforeIndexes(typed, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
		if fieldName == "TrailingComment" {
			return reflect.ValueOf(blockStmtTrailingCommentIndexes(typed, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
		if fieldName == "BlankLineBefore" {
			return reflect.ValueOf(blockStmtBlankLineBeforeIndexes(typed, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
	case *ast.IfStmt:
		if fieldName == "TrailingComment" && typed.Body != nil {
			return reflect.ValueOf(builder.ifStmtHeaderTrailingCommentAnchors[typed.Body.Lbrace]), true
		}
	case *ast.CaseClause:
		if fieldName == "CommentBefore" {
			if len(typed.Body) == 0 {
				return reflect.ValueOf(builder.emptyClauseBodyCommentAnchors[typed.Colon]), true
			}
			return reflect.ValueOf(statementListCommentBeforeIndexes(typed.Body, typed.Colon, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
		if fieldName == "TrailingComment" {
			return reflect.ValueOf(statementListTrailingCommentIndexes(typed.Body, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
		if fieldName == "BlankLineBefore" {
			return reflect.ValueOf(statementListBlankLineBeforeIndexes(typed.Body, typed.Colon, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
	case *ast.CommClause:
		if fieldName == "CommentBefore" {
			if len(typed.Body) == 0 {
				return reflect.ValueOf(builder.emptyClauseBodyCommentAnchors[typed.Colon]), true
			}
			return reflect.ValueOf(statementListCommentBeforeIndexes(typed.Body, typed.Colon, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
		if fieldName == "TrailingComment" {
			return reflect.ValueOf(statementListTrailingCommentIndexes(typed.Body, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
		if fieldName == "BlankLineBefore" {
			return reflect.ValueOf(statementListBlankLineBeforeIndexes(typed.Body, typed.Colon, builder.fileSet, builder.fileComments, builder.attachedComments, builder.ignoredCommentPositions)), true
		}
	}
	return reflect.Value{}, false
}

func ruleHasField(rule syntaxRule, fieldName string) bool {
	for _, field := range rule.Fields {
		if field.Name == fieldName {
			return true
		}
	}
	return false
}

// fileDeclCommentBeforeIndexes anchors every root-only comment group that sits
// between (or after) the top-level declarations to the declaration index it
// precedes. Index len(Decls) anchors a group after the last declaration.
// Header groups (before the package clause), doc/attached groups, and groups
// inside a declaration are excluded: the header loop and the per-node anchors
// own those.
func fileDeclCommentBeforeIndexes(file *ast.File, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if file == nil || fileSet == nil || len(comments) == 0 {
		return nil
	}
	indexes := make([]int, 0)
	for _, group := range comments {
		if group == nil {
			continue
		}
		if _, attached := attachedComments[group.Pos()]; attached {
			continue
		}
		if _, ignored := ignoredCommentPositions[group.Pos()]; ignored {
			continue
		}
		if file.Package.IsValid() && group.End() < file.Package {
			continue
		}
		index := len(file.Decls)
		inside := false
		for declIndex, decl := range file.Decls {
			if decl == nil {
				continue
			}
			if group.End() < decl.Pos() {
				index = declIndex
				break
			}
			if group.Pos() < decl.End() {
				inside = true
				break
			}
		}
		if inside {
			continue
		}
		indexes = append(indexes, index)
	}
	sort.Ints(indexes)
	return indexes
}

// fileDeclNoBlankLineBeforeIndexes records the top-level declaration
// separators the source suppressed. gofmt preserves at most one blank line
// between declarations and forces one whenever the declaration token changes,
// the declaration carries a doc comment, or the declaration is a multi-line
// function, so "no blank line" is the only source-dependent choice and the
// sparse polarity keeps the elided stream small.
func fileDeclNoBlankLineBeforeIndexes(file *ast.File, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if file == nil || fileSet == nil || len(file.Decls) < 2 {
		return nil
	}
	commentStartLines := map[int]int{}
	for _, group := range comments {
		if group == nil {
			continue
		}
		if _, attached := attachedComments[group.Pos()]; attached {
			continue
		}
		if _, ignored := ignoredCommentPositions[group.Pos()]; ignored {
			continue
		}
		if file.Package.IsValid() && group.End() < file.Package {
			continue
		}
		for declIndex, decl := range file.Decls {
			if decl == nil {
				continue
			}
			if group.End() < decl.Pos() {
				if _, exists := commentStartLines[declIndex]; !exists {
					commentStartLines[declIndex] = fileSet.Position(group.Pos()).Line
				}
				break
			}
			if group.Pos() < decl.End() {
				break
			}
		}
	}
	indexes := make([]int, 0)
	for index := 1; index < len(file.Decls); index++ {
		previous := file.Decls[index-1]
		current := file.Decls[index]
		if previous == nil || current == nil {
			continue
		}
		previousEndLine := fileSet.Position(previous.End()).Line
		separatorStartLine := fileSet.Position(declStartPos(current)).Line
		if commentStartLine, ok := commentStartLines[index]; ok {
			separatorStartLine = commentStartLine
		}
		if previousEndLine > 0 && separatorStartLine <= previousEndLine+1 {
			indexes = append(indexes, index)
		}
	}
	return indexes
}

// declStartPos returns the declaration position including its doc comment, so
// the blank-line separator is measured against the first rendered line.
func declStartPos(decl ast.Decl) token.Pos {
	switch typed := decl.(type) {
	case *ast.GenDecl:
		if typed.Doc != nil && typed.Doc.Pos().IsValid() {
			return typed.Doc.Pos()
		}
	case *ast.FuncDecl:
		if typed.Doc != nil && typed.Doc.Pos().IsValid() {
			return typed.Doc.Pos()
		}
	}
	return decl.Pos()
}

func fieldListBlankLineBeforeIndexes(fieldList *ast.FieldList, fileSet *token.FileSet) []int {
	if fieldList == nil || fileSet == nil || len(fieldList.List) < 2 {
		return nil
	}
	indexes := make([]int, 0)
	for index := 1; index < len(fieldList.List); index++ {
		previous := fieldList.List[index-1]
		current := fieldList.List[index]
		if previous == nil || current == nil {
			continue
		}
		previousEndLine := fileSet.Position(previous.End()).Line
		currentStartLine := fileSet.Position(current.Pos()).Line
		if previousEndLine > 0 && currentStartLine > previousEndLine+1 {
			indexes = append(indexes, index)
		}
	}
	return indexes
}

func compositeLitLineBreakBeforeIndexes(literal *ast.CompositeLit, fileSet *token.FileSet) []int {
	if literal == nil || fileSet == nil || len(literal.Elts) == 0 || !literal.Lbrace.IsValid() {
		return nil
	}
	indexes := make([]int, 0)
	previousLine := fileSet.Position(literal.Lbrace).Line
	for index, element := range literal.Elts {
		if element == nil {
			continue
		}
		elementStartLine := fileSet.Position(element.Pos()).Line
		if previousLine > 0 && elementStartLine > previousLine {
			indexes = append(indexes, index)
		}
		previousLine = fileSet.Position(element.End()).Line
	}
	return indexes
}

func callExprLineBreakBeforeIndexes(call *ast.CallExpr, fileSet *token.FileSet) []int {
	if call == nil || fileSet == nil || len(call.Args) == 0 || !call.Lparen.IsValid() {
		return nil
	}
	indexes := make([]int, 0)
	previousLine := fileSet.Position(call.Lparen).Line
	for index, argument := range call.Args {
		if argument == nil {
			continue
		}
		argumentStartLine := fileSet.Position(argument.Pos()).Line
		if previousLine > 0 && argumentStartLine > previousLine {
			indexes = append(indexes, index)
		}
		previousLine = fileSet.Position(argument.End()).Line
	}
	return indexes
}

func blockStmtCommentBeforeIndexes(block *ast.BlockStmt, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if block == nil {
		return nil
	}
	return statementListCommentBeforeIndexesWithEnd(block.List, block.Lbrace, block.Rbrace, fileSet, comments, attachedComments, ignoredCommentPositions)
}

func blockStmtTrailingCommentIndexes(block *ast.BlockStmt, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if block == nil {
		return nil
	}
	return statementListTrailingCommentIndexes(block.List, fileSet, comments, attachedComments, ignoredCommentPositions)
}

func blockStmtBlankLineBeforeIndexes(block *ast.BlockStmt, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if block == nil {
		return nil
	}
	return statementListBlankLineBeforeIndexesWithEnd(block.List, block.Lbrace, block.Rbrace, fileSet, comments, attachedComments, ignoredCommentPositions)
}

func statementListCommentBeforeIndexes(statements []ast.Stmt, anchor token.Pos, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	indexes, _ := statementListCommentBeforeIndexLines(statements, anchor, token.NoPos, fileSet, comments, attachedComments, ignoredCommentPositions)
	return indexes
}

// statementListCommentBeforeIndexesWithEnd additionally anchors a comment
// group that follows the last statement to index len(statements), using `end`
// (the closing brace) as the upper bound. Without it such a group has no
// anchor at all and the file-header loop consumes it out of order.
func statementListCommentBeforeIndexesWithEnd(statements []ast.Stmt, anchor token.Pos, end token.Pos, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	indexes, _ := statementListCommentBeforeIndexLines(statements, anchor, end, fileSet, comments, attachedComments, ignoredCommentPositions)
	return indexes
}

func statementListTrailingCommentIndexes(statements []ast.Stmt, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if fileSet == nil || len(statements) == 0 || len(comments) == 0 {
		return nil
	}
	indexes := make([]int, 0)
	seen := map[int]struct{}{}
	for _, group := range comments {
		if group == nil {
			continue
		}
		if _, attached := attachedComments[group.Pos()]; attached {
			continue
		}
		if _, ignored := ignoredCommentPositions[group.Pos()]; ignored {
			continue
		}
		for index, stmt := range statements {
			if stmt == nil || !commentGroupTrailsStatement(group, stmt, fileSet) {
				continue
			}
			if _, exists := seen[index]; !exists {
				indexes = append(indexes, index)
				seen[index] = struct{}{}
			}
			break
		}
	}
	return indexes
}

func statementListBlankLineBeforeIndexes(statements []ast.Stmt, anchor token.Pos, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	return statementListBlankLineBeforeIndexesWithEnd(statements, anchor, token.NoPos, fileSet, comments, attachedComments, ignoredCommentPositions)
}

// statementListBlankLineBeforeIndexesWithEnd additionally records a blank line
// before the trailing comment group anchored at index len(statements).
func statementListBlankLineBeforeIndexesWithEnd(statements []ast.Stmt, anchor token.Pos, end token.Pos, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) []int {
	if fileSet == nil || !anchor.IsValid() || len(statements) < 1 {
		return nil
	}
	_, commentStartLines := statementListCommentBeforeIndexLines(statements, anchor, end, fileSet, comments, attachedComments, ignoredCommentPositions)
	indexes := make([]int, 0)
	for index := 1; index < len(statements); index++ {
		previous := statements[index-1]
		current := statements[index]
		if previous == nil || current == nil {
			continue
		}
		previousEndLine := fileSet.Position(previous.End()).Line
		separatorStartLine := fileSet.Position(current.Pos()).Line
		if commentStartLine, ok := commentStartLines[index]; ok {
			separatorStartLine = commentStartLine
		} else if ignoredCommentStartLine, ok := firstIgnoredCommentStartLineBetween(
			previous.End(),
			current.Pos(),
			fileSet,
			comments,
			ignoredCommentPositions,
		); ok {
			separatorStartLine = ignoredCommentStartLine
		}
		if previousEndLine > 0 && separatorStartLine > previousEndLine+1 {
			indexes = append(indexes, index)
		}
	}
	if commentStartLine, ok := commentStartLines[len(statements)]; ok {
		last := statements[len(statements)-1]
		if last != nil {
			previousEndLine := fileSet.Position(last.End()).Line
			if previousEndLine > 0 && commentStartLine > previousEndLine+1 {
				indexes = append(indexes, len(statements))
			}
		}
	}
	return indexes
}

func blockStmtCommentBeforeIndexLines(block *ast.BlockStmt, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) ([]int, map[int]int) {
	if block == nil {
		return nil, map[int]int{}
	}
	return statementListCommentBeforeIndexLines(block.List, block.Lbrace, block.Rbrace, fileSet, comments, attachedComments, ignoredCommentPositions)
}

func statementListCommentBeforeIndexLines(statements []ast.Stmt, anchor token.Pos, end token.Pos, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}, ignoredCommentPositions map[token.Pos]struct{}) ([]int, map[int]int) {
	commentStartLines := map[int]int{}
	if fileSet == nil || !anchor.IsValid() || len(statements) == 0 || len(comments) == 0 {
		return nil, commentStartLines
	}
	indexes := make([]int, 0)
	for _, group := range comments {
		if group == nil || group.Pos() <= anchor {
			continue
		}
		if _, attached := attachedComments[group.Pos()]; attached {
			continue
		}
		if _, ignored := ignoredCommentPositions[group.Pos()]; ignored {
			continue
		}
		matched := false
		for index, stmt := range statements {
			if stmt == nil {
				continue
			}
			if commentGroupTrailsStatement(group, stmt, fileSet) {
				matched = true
				break
			}
			if group.End() >= stmt.Pos() {
				continue
			}
			previousEnd := anchor
			if index > 0 && statements[index-1] != nil {
				if commentGroupTrailsStatement(group, statements[index-1], fileSet) {
					matched = true
					break
				}
				previousEnd = statements[index-1].End()
			}
			if group.Pos() > previousEnd {
				if _, exists := commentStartLines[index]; !exists {
					indexes = append(indexes, index)
					commentStartLines[index] = fileSet.Position(group.Pos()).Line
				}
			}
			matched = true
			break
		}
		if matched || !end.IsValid() {
			continue
		}
		// The group follows every statement. Anchor it to the slot before the
		// closing brace so it is not left for the greedy file-header loop.
		last := statements[len(statements)-1]
		if last == nil || group.Pos() <= last.End() || group.End() >= end {
			continue
		}
		if commentGroupTrailsStatement(group, last, fileSet) {
			continue
		}
		if _, exists := commentStartLines[len(statements)]; !exists {
			indexes = append(indexes, len(statements))
			commentStartLines[len(statements)] = fileSet.Position(group.Pos()).Line
		}
	}
	sort.Ints(indexes)
	return indexes, commentStartLines
}

func commentGroupTrailsStatement(group *ast.CommentGroup, stmt ast.Stmt, fileSet *token.FileSet) bool {
	if group == nil || stmt == nil || fileSet == nil || group.Pos() <= stmt.End() {
		return false
	}
	statementEndLine := fileSet.Position(stmt.End()).Line
	groupStartLine := fileSet.Position(group.Pos()).Line
	groupEndLine := fileSet.Position(group.End()).Line
	return statementEndLine > 0 && groupStartLine == statementEndLine && groupEndLine == statementEndLine
}

func ifStmtHeaderTrailingCommentAnchors(root ast.Node, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}) (map[token.Pos][]int, map[token.Pos]struct{}) {
	anchors := map[token.Pos][]int{}
	positions := map[token.Pos]struct{}{}
	if root == nil || fileSet == nil || len(comments) == 0 {
		return anchors, positions
	}
	ast.Inspect(root, func(node ast.Node) bool {
		ifStmt, ok := node.(*ast.IfStmt)
		if !ok || ifStmt.Body == nil || !ifStmt.Body.Lbrace.IsValid() {
			return true
		}
		for _, group := range comments {
			if group == nil || !commentGroupTrailsIfHeader(group, ifStmt, fileSet) {
				continue
			}
			if _, attached := attachedComments[group.Pos()]; attached {
				continue
			}
			if _, alreadyClaimed := positions[group.Pos()]; alreadyClaimed {
				continue
			}
			anchors[ifStmt.Body.Lbrace] = []int{0}
			positions[group.Pos()] = struct{}{}
			break
		}
		return true
	})
	return anchors, positions
}

func commentGroupTrailsIfHeader(group *ast.CommentGroup, ifStmt *ast.IfStmt, fileSet *token.FileSet) bool {
	if group == nil || ifStmt == nil || ifStmt.Body == nil || fileSet == nil {
		return false
	}
	if !ifStmt.Body.Lbrace.IsValid() || group.Pos() <= ifStmt.Body.Lbrace {
		return false
	}
	headerLine := fileSet.Position(ifStmt.Body.Lbrace).Line
	groupStartLine := fileSet.Position(group.Pos()).Line
	groupEndLine := fileSet.Position(group.End()).Line
	return headerLine > 0 && groupStartLine == headerLine && groupEndLine == headerLine
}

func mergeCommentPositionSets(sets ...map[token.Pos]struct{}) map[token.Pos]struct{} {
	merged := map[token.Pos]struct{}{}
	for _, set := range sets {
		for position := range set {
			merged[position] = struct{}{}
		}
	}
	return merged
}

func firstIgnoredCommentStartLineBetween(previousEnd token.Pos, currentStart token.Pos, fileSet *token.FileSet, comments []*ast.CommentGroup, ignoredCommentPositions map[token.Pos]struct{}) (int, bool) {
	if fileSet == nil || len(ignoredCommentPositions) == 0 {
		return 0, false
	}
	for _, group := range comments {
		if group == nil {
			continue
		}
		if _, ignored := ignoredCommentPositions[group.Pos()]; !ignored {
			continue
		}
		if group.Pos() > previousEnd && group.End() < currentStart {
			line := fileSet.Position(group.Pos()).Line
			if line > 0 {
				return line, true
			}
		}
	}
	return 0, false
}

func emptyClauseBodyCommentAnchors(root ast.Node, fileSet *token.FileSet, comments []*ast.CommentGroup, attachedComments map[token.Pos]struct{}) (map[token.Pos][]int, map[token.Pos]struct{}) {
	anchors := map[token.Pos][]int{}
	positions := map[token.Pos]struct{}{}
	if root == nil || fileSet == nil || len(comments) == 0 {
		return anchors, positions
	}
	ast.Inspect(root, func(node ast.Node) bool {
		block, ok := node.(*ast.BlockStmt)
		if !ok || len(block.List) == 0 {
			return true
		}
		for index, stmt := range block.List {
			colon, body, ok := clauseBody(stmt)
			if !ok || len(body) != 0 || !colon.IsValid() {
				continue
			}
			upper := block.Rbrace
			if index+1 < len(block.List) && block.List[index+1] != nil {
				upper = block.List[index+1].Pos()
			}
			if !upper.IsValid() {
				continue
			}
			for _, group := range comments {
				if group == nil || group.Pos() <= colon || group.End() >= upper {
					continue
				}
				if _, attached := attachedComments[group.Pos()]; attached {
					continue
				}
				if _, alreadyClaimed := positions[group.Pos()]; alreadyClaimed {
					continue
				}
				if _, exists := anchors[colon]; exists {
					break
				}
				anchors[colon] = []int{0}
				positions[group.Pos()] = struct{}{}
				break
			}
		}
		return true
	})
	return anchors, positions
}

func clauseBody(stmt ast.Stmt) (token.Pos, []ast.Stmt, bool) {
	switch typed := stmt.(type) {
	case *ast.CaseClause:
		return typed.Colon, typed.Body, true
	case *ast.CommClause:
		return typed.Colon, typed.Body, true
	default:
		return token.NoPos, nil, false
	}
}

func attachedCommentGroupPositions(root ast.Node) map[token.Pos]struct{} {
	attached := map[token.Pos]struct{}{}
	ast.Inspect(root, func(node ast.Node) bool {
		if node == nil {
			return true
		}
		value := reflect.ValueOf(node)
		if value.Kind() == reflect.Ptr {
			value = value.Elem()
		}
		if !value.IsValid() || value.Kind() != reflect.Struct {
			return true
		}
		valueType := value.Type()
		for fieldIndex := 0; fieldIndex < value.NumField(); fieldIndex++ {
			structField := valueType.Field(fieldIndex)
			if !structField.IsExported() || (structField.Name != "Doc" && structField.Name != "Comment") {
				continue
			}
			field := value.Field(fieldIndex)
			if !field.IsValid() || field.IsNil() || !field.CanInterface() {
				continue
			}
			if group, ok := field.Interface().(*ast.CommentGroup); ok && group != nil {
				attached[group.Pos()] = struct{}{}
			}
		}
		return true
	})
	return attached
}

func (builder *projectionBuilder) projectField(value reflect.Value, field syntaxField, depth int) (projectionField, error) {
	builder.stats.FieldCount++
	projected := projectionField{
		Name:     field.Name,
		Required: field.Required,
		Multiple: field.Multiple,
	}
	value = unwrapInterfaceValue(value)
	if field.Multiple {
		if value.Kind() != reflect.Slice && value.Kind() != reflect.Array {
			return projectionField{}, fmt.Errorf("multiple metadata field has non-slice Go value %s", value.Type())
		}
		if value.Len() == 0 {
			builder.stats.RepeatedEmptyCount++
			if field.Required {
				return projectionField{}, errors.New("required repeated field is empty")
			}
			projected.Values = []projectionValue{}
			return projected, nil
		}
		for index := 0; index < value.Len(); index++ {
			projectedValue, err := builder.projectFieldValue(value.Index(index), field.Alternatives, depth)
			if err != nil {
				return projectionField{}, fmt.Errorf("index %d: %w", index, err)
			}
			if projectedValue.Kind == "terminal" {
				if projected.Terminal == "" {
					projected.Terminal = projectedValue.Terminal
				} else if projected.Terminal != projectedValue.Terminal {
					return projectionField{}, fmt.Errorf("mixed terminal payload kinds %q and %q", projected.Terminal, projectedValue.Terminal)
				}
			}
			projected.Values = append(projected.Values, projectedValue)
		}
		return projected, nil
	}
	if isNilReflectValue(value) {
		if field.Required {
			return projectionField{}, errors.New("required field is nil")
		}
		builder.stats.OptionalMissingCount++
		projected.Missing = true
		return projected, nil
	}
	projectedValue, err := builder.projectFieldValue(value, field.Alternatives, depth)
	if err != nil {
		return projectionField{}, err
	}
	projected.Terminal = projectedValue.Terminal
	projected.Values = []projectionValue{projectedValue}
	return projected, nil
}

func (builder *projectionBuilder) projectFieldValue(value reflect.Value, alternatives []syntaxAlternative, depth int) (projectionValue, error) {
	value = unwrapInterfaceValue(value)
	if isNilReflectValue(value) {
		return projectionValue{}, errors.New("nil field value")
	}
	if domainValue, ok, err := projectFiniteDomainTerminal(value, alternatives); ok || err != nil {
		if err != nil {
			return projectionValue{}, err
		}
		builder.recordTerminal(domainValue.Terminal)
		return domainValue, nil
	}
	if node, ok := reflectInterface(value).(ast.Node); ok {
		nodeKind := astNodeKind(node)
		if hasNonterminalAlternative(alternatives, nodeKind) {
			projectedNode, err := builder.projectNode(reflect.ValueOf(node), depth)
			if err != nil {
				return projectionValue{}, err
			}
			builder.stats.FieldValueCount++
			return projectionValue{
				Kind:     "node",
				NodeKind: nodeKind,
				Node:     &projectedNode,
			}, nil
		}
		if alternative, ok := chooseTerminalAlternative(alternatives, value); ok {
			payload, tokenName, err := terminalPayload(value, alternative.Terminal)
			if err != nil {
				return projectionValue{}, err
			}
			builder.recordTerminal(alternative.Terminal)
			return projectionValue{
				Kind:      "terminal",
				NodeKind:  alternative.NodeKind,
				Terminal:  alternative.Terminal,
				Payload:   stringPtr(payload),
				TokenName: tokenName,
			}, nil
		}
		return projectionValue{}, fmt.Errorf("no alternative accepts AST node kind %q", nodeKind)
	}
	alternative, ok := chooseTerminalAlternative(alternatives, value)
	if !ok {
		return projectionValue{}, fmt.Errorf("no terminal alternative accepts Go value %s", value.Type())
	}
	payload, tokenName, err := terminalPayload(value, alternative.Terminal)
	if err != nil {
		return projectionValue{}, err
	}
	builder.recordTerminal(alternative.Terminal)
	return projectionValue{
		Kind:      "terminal",
		NodeKind:  alternative.NodeKind,
		Terminal:  alternative.Terminal,
		Payload:   stringPtr(payload),
		TokenName: tokenName,
	}, nil
}

func (builder *projectionBuilder) recordTerminal(terminal string) {
	builder.stats.FieldValueCount++
	builder.stats.TerminalPayloadCount++
	builder.stats.TerminalPayloadsByKind[terminal]++
}

func (builder *projectionBuilder) finishStats() {
	nodeKinds := make([]string, 0, len(builder.nodeKinds))
	for nodeKind := range builder.nodeKinds {
		nodeKinds = append(nodeKinds, nodeKind)
	}
	sort.Strings(nodeKinds)
	builder.stats.DistinctNodeKinds = nodeKinds
}

func hasNonterminalAlternative(alternatives []syntaxAlternative, nodeKind string) bool {
	for _, alternative := range alternatives {
		if alternative.Terminal == "nonterminal" && alternative.NodeKind == nodeKind {
			return true
		}
	}
	return false
}

func chooseTerminalAlternative(alternatives []syntaxAlternative, value reflect.Value) (syntaxAlternative, bool) {
	terminalAlternatives := make([]syntaxAlternative, 0, len(alternatives))
	for _, alternative := range alternatives {
		if alternative.Terminal != "nonterminal" {
			terminalAlternatives = append(terminalAlternatives, alternative)
		}
	}
	if len(terminalAlternatives) == 0 {
		return syntaxAlternative{}, false
	}
	if len(terminalAlternatives) == 1 {
		return terminalAlternatives[0], true
	}
	payload := reflectPayloadString(value)
	for _, alternative := range terminalAlternatives {
		if alternative.Terminal == "integer_payload" && looksLikeIntegerLiteral(payload) {
			return alternative, true
		}
	}
	for _, alternative := range terminalAlternatives {
		if alternative.Terminal == "identifier_payload" && looksLikeIdentifier(payload) {
			return alternative, true
		}
	}
	for _, alternative := range terminalAlternatives {
		if alternative.Terminal == "string_payload" {
			return alternative, true
		}
	}
	return terminalAlternatives[0], true
}

func terminalPayload(value reflect.Value, terminal string) (string, string, error) {
	value = unwrapInterfaceValue(value)
	if !value.IsValid() {
		return "", "", errors.New("invalid terminal value")
	}
	if value.CanInterface() {
		switch typed := value.Interface().(type) {
		case *ast.Ident:
			if typed == nil {
				return "", "", errors.New("nil identifier terminal")
			}
			return typed.Name, "", nil
		case ast.Ident:
			return typed.Name, "", nil
		case *ast.BasicLit:
			if typed == nil {
				return "", "", errors.New("nil literal terminal")
			}
			return typed.Value, tokenName(typed.Kind), nil
		case ast.BasicLit:
			return typed.Value, tokenName(typed.Kind), nil
		case token.Token:
			return typed.String(), tokenName(typed), nil
		}
	}
	value = unwrapReflectValue(value)
	switch value.Kind() {
	case reflect.String:
		return value.String(), "", nil
	case reflect.Bool:
		return fmt.Sprintf("%t", value.Bool()), "", nil
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		if value.Type().PkgPath() == "go/token" && value.Type().Name() == "Token" {
			tokenValue := token.Token(value.Int())
			return tokenValue.String(), tokenName(tokenValue), nil
		}
		return fmt.Sprintf("%d", value.Int()), "", nil
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		return fmt.Sprintf("%d", value.Uint()), "", nil
	default:
		return "", "", fmt.Errorf("unsupported %s terminal value %s", terminal, value.Type())
	}
}

func canonicalRenderEvidenceForSource(sourcePath string, sourceBytes []byte) (canonicalRenderEvidence, error) {
	formatted, err := format.Source(sourceBytes)
	if err != nil {
		return canonicalRenderEvidence{}, fmt.Errorf("gofmt source: %w", err)
	}
	if _, err := parser.ParseFile(token.NewFileSet(), sourcePath, formatted, parser.ParseComments); err != nil {
		return canonicalRenderEvidence{}, fmt.Errorf("reparse formatted source: %w", err)
	}
	return canonicalRenderEvidence{
		Formatter:          "go/format.Source",
		Gofmt:              "passed",
		GoParser:           "passed",
		MatchesSource:      bytes.Equal(formatted, sourceBytes),
		CanonicalSHA256:    sha256Hex(formatted),
		CanonicalByteCount: len(formatted),
	}, nil
}

// writeGofmtCanonicalWorkingCopy persists format.Source output so projection
// Source.Path re-reads match the gofmt-canonical hash/bytes used for tokens.
// Historical working copies still inject a `/src/` segment for GOROOT-shaped
// paths; the unified grammar validator accepts any readable *.go path (fixture
// corpus, GOROOT src/, external module checkouts, or these working copies).
func writeGofmtCanonicalWorkingCopy(options sourceInstanceOptions, formatted []byte) (string, error) {
	baseDir := ""
	if options.projectionOutput != "" {
		baseDir = filepath.Dir(options.projectionOutput)
	} else if options.reportOutput != "" {
		baseDir = filepath.Dir(options.reportOutput)
	} else {
		tmp, err := os.MkdirTemp("", "tct-gofmt-canonical-*")
		if err != nil {
			return "", fmt.Errorf("create gofmt-canonical temp dir: %w", err)
		}
		baseDir = tmp
	}
	rel := options.manifestPath
	if rel == "" {
		rel = filepath.Base(options.sourcePath)
	}
	rel = strings.TrimPrefix(filepath.ToSlash(rel), "/")
	// Keep historical GOROOT-shaped working-copy layout when the relative path
	// has no /src/ segment; not required for unified-grammar validation.
	if !strings.Contains("/"+rel, "/src/") && !strings.HasPrefix(rel, "src/") {
		rel = pathJoinSrc(rel)
	}
	canonicalPath := filepath.Join(baseDir, "gofmt-canonical-sources", filepath.FromSlash(rel))
	if err := os.MkdirAll(filepath.Dir(canonicalPath), 0o755); err != nil {
		return "", fmt.Errorf("create gofmt-canonical parent for %s: %w", canonicalPath, err)
	}
	if err := os.WriteFile(canonicalPath, formatted, 0o644); err != nil {
		return "", fmt.Errorf("write gofmt-canonical working copy %s: %w", canonicalPath, err)
	}
	return canonicalPath, nil
}

func pathJoinSrc(rel string) string {
	rel = strings.TrimPrefix(rel, "./")
	if strings.HasPrefix(rel, "src/") {
		return rel
	}
	return "src/" + rel
}

func validateSourceInstanceReport(path string, expected sourceInstanceReport) error {
	var actual sourceInstanceReport
	if err := readJSON(path, &actual); err != nil {
		return err
	}
	actualBytes, err := json.Marshal(actual)
	if err != nil {
		return fmt.Errorf("encode actual report: %w", err)
	}
	expectedBytes, err := json.Marshal(expected)
	if err != nil {
		return fmt.Errorf("encode expected report: %w", err)
	}
	if !bytes.Equal(actualBytes, expectedBytes) {
		return fmt.Errorf("source instance report drift for %s", path)
	}
	return nil
}

func writeJSON(path string, value interface{}) error {
	if parent := filepath.Dir(path); parent != "." {
		if err := os.MkdirAll(parent, 0o755); err != nil {
			return fmt.Errorf("create parent directory for %s: %w", path, err)
		}
	}
	encoded, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		return fmt.Errorf("encode %s: %w", path, err)
	}
	encoded = append(encoded, '\n')
	if err := os.WriteFile(path, encoded, 0o644); err != nil {
		return fmt.Errorf("write %s: %w", path, err)
	}
	return nil
}

func readJSON(path string, value interface{}) error {
	handle, err := os.Open(path)
	if err != nil {
		return fmt.Errorf("open %s: %w", path, err)
	}
	defer handle.Close()
	if err := json.NewDecoder(handle).Decode(value); err != nil {
		return fmt.Errorf("decode %s: %w", path, err)
	}
	return nil
}

func validationCommands(options sourceInstanceOptions) []string {
	manifestFlag := ""
	if options.manifestPath != "" {
		manifestFlag = " --manifest-path " + options.manifestPath
	}
	metadataSmokeFixture := metadataSmokeFixtureForCommand(options)
	metadataSmokeFixtureFlag := ""
	if options.metadataSmokeFixture != "" {
		metadataSmokeFixtureFlag = " --metadata-smoke-fixture " + options.metadataSmokeFixture
	}
	reportIngestionManifestFlag := ""
	if options.reportIngestionManifestPath != "" {
		reportIngestionManifestFlag = " --report-ingestion-manifest " + options.reportIngestionManifestPath
	}
	routeRegisteredFlag := ""
	if options.routeRegistered {
		routeRegisteredFlag = " --route-registered"
	}
	metadataValidationCommand := fmt.Sprintf(
		"go run scripts/analysis/go_syntax_metadata_smoke.go --fixture %s --source %s --checked-metadata %s --self-test",
		metadataSmokeFixture,
		options.sourcePath,
		options.metadataPath,
	)
	if options.inventoryMetadata {
		metadataValidationCommand = fmt.Sprintf(
			"go run scripts/analysis/go_ast_schema_inventory.go --validate %s --self-test",
			options.metadataPath,
		)
	}
	return []string{
		metadataValidationCommand,
		fmt.Sprintf(
			"go run scripts/analysis/go_syntax_source_instance_project.go --source %s --metadata %s%s%s --candidate-grammar-id %s%s%s --projection-output %s --report-output %s",
			options.sourcePath,
			options.metadataPath,
			metadataSmokeFixtureFlag,
			manifestFlag,
			options.candidateGrammarID,
			reportIngestionManifestFlag,
			routeRegisteredFlag,
			options.projectionOutput,
			options.reportOutput,
		),
		fmt.Sprintf(
			"go run scripts/analysis/go_syntax_source_instance_project.go --source %s --metadata %s%s%s --candidate-grammar-id %s%s%s --projection-output %s --validate-report %s",
			options.sourcePath,
			options.metadataPath,
			metadataSmokeFixtureFlag,
			manifestFlag,
			options.candidateGrammarID,
			reportIngestionManifestFlag,
			routeRegisteredFlag,
			options.projectionOutput,
			options.reportOutput,
		),
		"go test scripts/analysis/go_syntax_source_instance_project.go scripts/analysis/go_syntax_source_instance_project_test.go",
		"git diff --check",
	}
}

func metadataSmokeFixtureForCommand(options sourceInstanceOptions) string {
	if options.metadataSmokeFixture != "" {
		return options.metadataSmokeFixture
	}
	return defaultNetLookupPlan9MetadataFixture
}

func reportIngestionManifestPathForBlocker(options sourceInstanceOptions) string {
	if options.reportIngestionManifestPath != "" {
		return options.reportIngestionManifestPath
	}
	return defaultReportIngestionManifestPath
}

func projectionClaimBoundary() string {
	return "parser-backed source-instance projection evidence only; generated-codec token production, report ingestion, training-corpus promotion, source-validity-by-construction, type-correctness, model-quality, paper-facing, generated-hot-path, arena-IR, and stable-backend claims remain blocked"
}

func noStableSourceInstanceClaims() []string {
	return []string{
		"no_generated_codec_report_claim",
		"no_training_corpus_manifest_promotion",
		"no_report_ingestion_manifest_promotion",
		"no_source_validity_by_construction_claim",
		"no_type_correctness_claim",
		"no_model_quality_claim",
		"no_paper_facing_claim",
		"no_generated_hot_path_claim",
		"no_arena_ir_promotion",
		"no_stable_backend_promotion",
	}
}

func unwrapReflectValue(value reflect.Value) reflect.Value {
	for value.IsValid() && (value.Kind() == reflect.Interface || value.Kind() == reflect.Pointer) {
		if value.IsNil() {
			return value
		}
		value = value.Elem()
	}
	return value
}

func unwrapInterfaceValue(value reflect.Value) reflect.Value {
	for value.IsValid() && value.Kind() == reflect.Interface {
		if value.IsNil() {
			return value
		}
		value = value.Elem()
	}
	return value
}

func isNilReflectValue(value reflect.Value) bool {
	if !value.IsValid() {
		return true
	}
	switch value.Kind() {
	case reflect.Chan, reflect.Func, reflect.Interface, reflect.Map, reflect.Pointer, reflect.Slice:
		return value.IsNil()
	default:
		return false
	}
}

func reflectInterface(value reflect.Value) interface{} {
	if !value.IsValid() || !value.CanInterface() {
		return nil
	}
	return value.Interface()
}

func astNodeKind(node ast.Node) string {
	typeName := reflect.TypeOf(node).String()
	typeName = strings.TrimPrefix(typeName, "*")
	typeName = strings.TrimPrefix(typeName, "ast.")
	return typeName
}

func reflectPayloadString(value reflect.Value) string {
	value = unwrapInterfaceValue(value)
	if !value.IsValid() {
		return ""
	}
	if value.CanInterface() {
		switch typed := value.Interface().(type) {
		case *ast.Ident:
			if typed == nil {
				return ""
			}
			return typed.Name
		case ast.Ident:
			return typed.Name
		case *ast.BasicLit:
			if typed == nil {
				return ""
			}
			return typed.Value
		case ast.BasicLit:
			return typed.Value
		case token.Token:
			return typed.String()
		}
	}
	value = unwrapReflectValue(value)
	switch value.Kind() {
	case reflect.String:
		return value.String()
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		if value.Type().PkgPath() == "go/token" && value.Type().Name() == "Token" {
			return token.Token(value.Int()).String()
		}
		return fmt.Sprintf("%d", value.Int())
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		return fmt.Sprintf("%d", value.Uint())
	default:
		return ""
	}
}

func tokenName(value token.Token) string {
	rendered := value.String()
	if rendered == "" {
		return "ILLEGAL"
	}
	return strings.ToUpper(strings.ReplaceAll(rendered, " ", "_"))
}

// projectFiniteDomainTerminal maps inventory finite-domain terminal values
// onto their classified alternatives. The checked inventory grammar encodes
// token.Token fields as one identifier_payload alternative per token constant
// (`<domain>.<NAME>`), token.Pos syntax-presence fields as boolean_payload
// PRESENT/ABSENT alternatives, and ast.ChanDir as the channel-direction
// domain. Per-file metadata uses untyped single alternatives instead, so this
// mapping fires only when a suffix-classified alternative exists and falls
// through untouched otherwise.
func projectFiniteDomainTerminal(value reflect.Value, alternatives []syntaxAlternative) (projectionValue, bool, error) {
	unwrapped := unwrapReflectValue(value)
	if !unwrapped.IsValid() || unwrapped.Kind() < reflect.Int || unwrapped.Kind() > reflect.Int64 {
		return projectionValue{}, false, nil
	}
	switch {
	case unwrapped.Type().PkgPath() == "go/token" && unwrapped.Type().Name() == "Pos":
		alternative, ok := suffixAlternative(alternatives, "boolean_payload", "PRESENT", "ABSENT")
		if !ok {
			return projectionValue{}, false, nil
		}
		name, payload := "ABSENT", "false"
		if token.Pos(unwrapped.Int()).IsValid() {
			name, payload = "PRESENT", "true"
		}
		selected, ok := suffixAlternative(alternatives, "boolean_payload", name)
		if !ok {
			return projectionValue{}, false, fmt.Errorf("presence domain %q is missing the %s alternative", alternative.NodeKind, name)
		}
		return projectionValue{
			Kind:     "terminal",
			NodeKind: selected.NodeKind,
			Terminal: selected.Terminal,
			Payload:  stringPtr(payload),
		}, true, nil
	case unwrapped.Type().PkgPath() == "go/token" && unwrapped.Type().Name() == "Token":
		tokenValue := token.Token(unwrapped.Int())
		name := tokenConstantName(tokenValue)
		selected, ok := suffixAlternative(alternatives, "identifier_payload", name)
		if !ok {
			return projectionValue{}, false, nil
		}
		return projectionValue{
			Kind:      "terminal",
			NodeKind:  selected.NodeKind,
			Terminal:  selected.Terminal,
			Payload:   stringPtr(tokenValue.String()),
			TokenName: name,
		}, true, nil
	case unwrapped.Type().PkgPath() == "go/ast" && unwrapped.Type().Name() == "ChanDir":
		name := chanDirName(ast.ChanDir(unwrapped.Int()))
		if name == "" {
			return projectionValue{}, false, fmt.Errorf("unsupported channel direction %d", unwrapped.Int())
		}
		selected, ok := suffixAlternative(alternatives, "identifier_payload", name)
		if !ok {
			return projectionValue{}, false, nil
		}
		return projectionValue{
			Kind:      "terminal",
			NodeKind:  selected.NodeKind,
			Terminal:  selected.Terminal,
			Payload:   stringPtr(name),
			TokenName: name,
		}, true, nil
	default:
		return projectionValue{}, false, nil
	}
}

// suffixAlternative selects the alternative whose node_kind ends with
// `.<name>` for any of the given names and matches the terminal kind.
func suffixAlternative(alternatives []syntaxAlternative, terminal string, names ...string) (syntaxAlternative, bool) {
	for _, alternative := range alternatives {
		if alternative.Terminal != terminal {
			continue
		}
		for _, name := range names {
			if strings.HasSuffix(alternative.NodeKind, "."+name) {
				return alternative, true
			}
		}
	}
	return syntaxAlternative{}, false
}

func chanDirName(direction ast.ChanDir) string {
	switch direction {
	case ast.SEND:
		return "SEND"
	case ast.RECV:
		return "RECV"
	case ast.SEND | ast.RECV:
		return "SEND_RECV"
	default:
		return ""
	}
}

// tokenConstantName names the go/token constant for suffix-classified
// inventory domain alternatives (token.Token.String() renders operator
// lexemes, not constant names, so the table is explicit; pinned go1.24.3).
func tokenConstantName(value token.Token) string {
	names := map[token.Token]string{
		token.ILLEGAL: "ILLEGAL", token.IDENT: "IDENT",
		token.INT: "INT", token.FLOAT: "FLOAT", token.IMAG: "IMAG", token.CHAR: "CHAR", token.STRING: "STRING",
		token.ADD: "ADD", token.SUB: "SUB", token.MUL: "MUL", token.QUO: "QUO", token.REM: "REM",
		token.AND: "AND", token.OR: "OR", token.XOR: "XOR", token.SHL: "SHL", token.SHR: "SHR", token.AND_NOT: "AND_NOT",
		token.ADD_ASSIGN: "ADD_ASSIGN", token.SUB_ASSIGN: "SUB_ASSIGN", token.MUL_ASSIGN: "MUL_ASSIGN",
		token.QUO_ASSIGN: "QUO_ASSIGN", token.REM_ASSIGN: "REM_ASSIGN", token.AND_ASSIGN: "AND_ASSIGN",
		token.OR_ASSIGN: "OR_ASSIGN", token.XOR_ASSIGN: "XOR_ASSIGN", token.SHL_ASSIGN: "SHL_ASSIGN",
		token.SHR_ASSIGN: "SHR_ASSIGN", token.AND_NOT_ASSIGN: "AND_NOT_ASSIGN",
		token.LAND: "LAND", token.LOR: "LOR", token.ARROW: "ARROW", token.INC: "INC", token.DEC: "DEC",
		token.EQL: "EQL", token.LSS: "LSS", token.GTR: "GTR", token.ASSIGN: "ASSIGN", token.NOT: "NOT",
		token.NEQ: "NEQ", token.LEQ: "LEQ", token.GEQ: "GEQ", token.DEFINE: "DEFINE", token.ELLIPSIS: "ELLIPSIS",
		token.BREAK: "BREAK", token.CASE: "CASE", token.CHAN: "CHAN", token.CONST: "CONST", token.CONTINUE: "CONTINUE",
		token.DEFAULT: "DEFAULT", token.DEFER: "DEFER", token.ELSE: "ELSE", token.FALLTHROUGH: "FALLTHROUGH",
		token.FOR: "FOR", token.FUNC: "FUNC", token.GO: "GO", token.GOTO: "GOTO", token.IF: "IF",
		token.IMPORT: "IMPORT", token.INTERFACE: "INTERFACE", token.MAP: "MAP", token.PACKAGE: "PACKAGE",
		token.RANGE: "RANGE", token.RETURN: "RETURN", token.SELECT: "SELECT", token.STRUCT: "STRUCT",
		token.SWITCH: "SWITCH", token.TYPE: "TYPE", token.VAR: "VAR", token.TILDE: "TILDE",
	}
	if name, ok := names[value]; ok {
		return name
	}
	return tokenName(value)
}

func sha256Hex(data []byte) string {
	sum := sha256.Sum256(data)
	return hex.EncodeToString(sum[:])
}

func fatal(err error) {
	fmt.Fprintln(os.Stderr, err)
	os.Exit(1)
}

var (
	integerLiteralPattern = regexp.MustCompile(`^[-+]?((0[xX][0-9a-fA-F_]+)|(0[oO][0-7_]+)|(0[bB][01_]+)|([0-9][0-9_]*))$`)
	identifierPattern     = regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_]*$`)
)

func looksLikeIntegerLiteral(value string) bool {
	return integerLiteralPattern.MatchString(value)
}

func looksLikeIdentifier(value string) bool {
	return identifierPattern.MatchString(value)
}

func runSourceInstanceSelfTest() error {
	dir, err := os.MkdirTemp("", "tct-go-source-instance-project-self-test-")
	if err != nil {
		return err
	}
	defer os.RemoveAll(dir)
	sourcePath := filepath.Join(dir, "main.go")
	metadataPath := filepath.Join(dir, "metadata.json")
	source := []byte("package main\n\nconst answer = 42\n")
	if err := os.WriteFile(sourcePath, source, 0o644); err != nil {
		return err
	}
	metadata := syntheticSourceInstanceMetadata(sourcePath, sha256Hex(source))
	if err := writeJSON(metadataPath, metadata); err != nil {
		return err
	}
	options := sourceInstanceOptions{
		sourcePath:         sourcePath,
		metadataPath:       metadataPath,
		manifestPath:       "main.go",
		candidateGrammarID: "go_self_test_source_file_syntax_v0",
		projectionOutput:   filepath.Join(dir, "projection.json"),
		reportOutput:       filepath.Join(dir, "report.json"),
	}
	projection, report, err := buildSourceInstanceProjectionAndReport(options)
	if err != nil {
		return err
	}
	if projection.Root.NodeKind != "File" {
		return fmt.Errorf("self-test root node kind drift: %s", projection.Root.NodeKind)
	}
	if projection.Stats.TerminalPayloadCount == 0 {
		return errors.New("self-test projection did not record terminal payloads")
	}
	if !report.CurrentEvidence.SourceInstanceProjectionReady || report.CurrentEvidence.GeneratedCodecReportReady {
		return errors.New("self-test report status flags drifted")
	}
	if err := writeJSON(options.reportOutput, report); err != nil {
		return err
	}
	return validateSourceInstanceReport(options.reportOutput, report)
}

func syntheticSourceInstanceMetadata(sourcePath string, sourceHash string) syntaxMetadata {
	return syntaxMetadata{
		ArtifactKind: "tct_go_syntax_metadata_v0",
		Version:      0,
		Source: metadataSource{
			Path:   sourcePath,
			SHA256: sourceHash,
		},
		Parser: metadataParser{
			Tool:           "go",
			Version:        runtime.Version(),
			Package:        "go/parser",
			PackageVersion: runtime.Version(),
		},
		Rules: []syntaxRule{
			rule("File",
				field("Name", true, false, alt("Ident", "nonterminal")),
				field("Decls", true, true, alt("GenDecl", "nonterminal")),
			),
			rule("Ident", field("Name", true, false, alt("identifier", "identifier_payload"))),
			rule("GenDecl",
				field("Tok", true, false, alt("token", "string_payload")),
				field("Specs", true, true, alt("ValueSpec", "nonterminal")),
			),
			rule("ValueSpec",
				field("Names", true, true, alt("Ident", "nonterminal")),
				field("Values", true, true, alt("BasicLit", "nonterminal")),
			),
			rule("BasicLit",
				field("Kind", true, false, alt("literal_kind", "string_payload")),
				field("Value", true, false, alt("integer_literal", "integer_payload"), alt("string_literal", "string_payload")),
			),
		},
	}
}

func rule(nodeKind string, fields ...syntaxField) syntaxRule {
	return syntaxRule{Name: nodeKind, NodeKind: nodeKind, Fields: fields}
}

func field(name string, required bool, multiple bool, alternatives ...syntaxAlternative) syntaxField {
	return syntaxField{Name: name, Required: required, Multiple: multiple, Alternatives: alternatives}
}

func alt(nodeKind string, terminal string) syntaxAlternative {
	return syntaxAlternative{NodeKind: nodeKind, Terminal: terminal}
}
