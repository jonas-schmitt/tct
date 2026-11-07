// go_ast_schema_inventory emits a checked inventory of the pinned go/ast
// parser surface used by the Go syntax tokenizer-generator workstream.
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
	goAstSchemaInventoryTool              = "go_ast_schema_inventory"
	goAstSchemaInventoryVersion           = 1
	goAstSchemaInventoryArtifactKind      = "tct_go_ast_schema_inventory_v1"
	goAstSchemaInventoryCheckedFixture    = "tests/fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json"
	goAstSchemaInventoryGrammarID         = "go_ast_schema_inventory_go1_24_3"
	goAstSchemaInventoryNoStablePromotion = "parser-backed go/ast schema inventory evidence only; no TctCompiler route and no stable backend promotion"

	goAstSchemaPositionPolicyBadNodeSourceRange    = "bad_node_source_range_unsupported"
	goAstSchemaPositionPolicyCommentTriviaSidecar  = "comment_trivia_position_sidecar"
	goAstSchemaPositionPolicyParserPositionSidecar = "parser_source_position_sidecar"
	goAstSchemaPositionPolicyFixedSyntaxToken      = "fixed_syntax_token_renderer_metadata"
	goAstSchemaPositionPolicyFiniteDomainToken     = "finite_domain_token_renderer_metadata"
	goAstSchemaPositionPolicySyntaxPresence        = "syntax_presence_renderer_metadata"
	goAstSchemaPositionPolicyUnclassified          = "unclassified_token_pos_field"
)

var goAstSchemaPositionFieldPolicies = map[string]string{
	"ArrayType.Lbrack":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"AssignStmt.TokPos":       goAstSchemaPositionPolicyFiniteDomainToken,
	"BadDecl.From":            goAstSchemaPositionPolicyBadNodeSourceRange,
	"BadDecl.To":              goAstSchemaPositionPolicyBadNodeSourceRange,
	"BadExpr.From":            goAstSchemaPositionPolicyBadNodeSourceRange,
	"BadExpr.To":              goAstSchemaPositionPolicyBadNodeSourceRange,
	"BadStmt.From":            goAstSchemaPositionPolicyBadNodeSourceRange,
	"BadStmt.To":              goAstSchemaPositionPolicyBadNodeSourceRange,
	"BasicLit.ValuePos":       goAstSchemaPositionPolicyParserPositionSidecar,
	"BinaryExpr.OpPos":        goAstSchemaPositionPolicyFiniteDomainToken,
	"BlockStmt.Lbrace":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"BlockStmt.Rbrace":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"BranchStmt.TokPos":       goAstSchemaPositionPolicyFiniteDomainToken,
	"CallExpr.Lparen":         goAstSchemaPositionPolicyFixedSyntaxToken,
	"CallExpr.Rparen":         goAstSchemaPositionPolicyFixedSyntaxToken,
	"CaseClause.Case":         goAstSchemaPositionPolicySyntaxPresence,
	"CaseClause.Colon":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"ChanType.Begin":          goAstSchemaPositionPolicyFiniteDomainToken,
	"ChanType.Arrow":          goAstSchemaPositionPolicySyntaxPresence,
	"CommClause.Case":         goAstSchemaPositionPolicySyntaxPresence,
	"CommClause.Colon":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"Comment.Slash":           goAstSchemaPositionPolicyCommentTriviaSidecar,
	"CompositeLit.Lbrace":     goAstSchemaPositionPolicyFixedSyntaxToken,
	"CompositeLit.Rbrace":     goAstSchemaPositionPolicyFixedSyntaxToken,
	"DeferStmt.Defer":         goAstSchemaPositionPolicyFixedSyntaxToken,
	"Ellipsis.Ellipsis":       goAstSchemaPositionPolicyFixedSyntaxToken,
	"EmptyStmt.Semicolon":     goAstSchemaPositionPolicySyntaxPresence,
	"FieldList.Opening":       goAstSchemaPositionPolicySyntaxPresence,
	"FieldList.Closing":       goAstSchemaPositionPolicySyntaxPresence,
	"File.Package":            goAstSchemaPositionPolicyFixedSyntaxToken,
	"File.FileStart":          goAstSchemaPositionPolicyParserPositionSidecar,
	"File.FileEnd":            goAstSchemaPositionPolicyParserPositionSidecar,
	"ForStmt.For":             goAstSchemaPositionPolicyFixedSyntaxToken,
	"GenDecl.TokPos":          goAstSchemaPositionPolicyFiniteDomainToken,
	"GoStmt.Go":               goAstSchemaPositionPolicyFixedSyntaxToken,
	"Ident.NamePos":           goAstSchemaPositionPolicyParserPositionSidecar,
	"IfStmt.If":               goAstSchemaPositionPolicyFixedSyntaxToken,
	"ImportSpec.EndPos":       goAstSchemaPositionPolicyParserPositionSidecar,
	"IncDecStmt.TokPos":       goAstSchemaPositionPolicyFiniteDomainToken,
	"IndexExpr.Lbrack":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"IndexExpr.Rbrack":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"IndexListExpr.Lbrack":    goAstSchemaPositionPolicyFixedSyntaxToken,
	"IndexListExpr.Rbrack":    goAstSchemaPositionPolicyFixedSyntaxToken,
	"InterfaceType.Interface": goAstSchemaPositionPolicyFixedSyntaxToken,
	"KeyValueExpr.Colon":      goAstSchemaPositionPolicyFixedSyntaxToken,
	"LabeledStmt.Colon":       goAstSchemaPositionPolicyFixedSyntaxToken,
	"MapType.Map":             goAstSchemaPositionPolicyFixedSyntaxToken,
	"ParenExpr.Lparen":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"ParenExpr.Rparen":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"RangeStmt.For":           goAstSchemaPositionPolicyFixedSyntaxToken,
	"RangeStmt.TokPos":        goAstSchemaPositionPolicySyntaxPresence,
	"RangeStmt.Range":         goAstSchemaPositionPolicyFixedSyntaxToken,
	"ReturnStmt.Return":       goAstSchemaPositionPolicyFixedSyntaxToken,
	"SelectStmt.Select":       goAstSchemaPositionPolicyFixedSyntaxToken,
	"SendStmt.Arrow":          goAstSchemaPositionPolicyFixedSyntaxToken,
	"SliceExpr.Lbrack":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"SliceExpr.Rbrack":        goAstSchemaPositionPolicyFixedSyntaxToken,
	"StarExpr.Star":           goAstSchemaPositionPolicyFixedSyntaxToken,
	"StructType.Struct":       goAstSchemaPositionPolicyFixedSyntaxToken,
	"SwitchStmt.Switch":       goAstSchemaPositionPolicyFixedSyntaxToken,
	"TypeAssertExpr.Lparen":   goAstSchemaPositionPolicyFixedSyntaxToken,
	"TypeAssertExpr.Rparen":   goAstSchemaPositionPolicyFixedSyntaxToken,
	"TypeSwitchStmt.Switch":   goAstSchemaPositionPolicyFixedSyntaxToken,
	"UnaryExpr.OpPos":         goAstSchemaPositionPolicyFiniteDomainToken,
}

type goAstSchemaInventoryReport struct {
	Tool                      string                                `json:"tool"`
	Version                   int                                   `json:"version"`
	ArtifactKind              string                                `json:"artifact_kind"`
	Status                    string                                `json:"status"`
	GrammarID                 string                                `json:"grammar_id"`
	Go                        goAstSchemaInventoryToolchain         `json:"go"`
	Sources                   []goAstSchemaInventorySource          `json:"sources"`
	NodeStructs               []goAstSchemaInventoryNodeStruct      `json:"node_structs"`
	InterfaceFamilies         []goAstSchemaInventoryFamily          `json:"interface_families"`
	FiniteDomains             []goAstSchemaInventoryDomain          `json:"finite_domains"`
	GrammarIRRules            []goAstSchemaInventoryRule            `json:"grammar_ir_rules"`
	LoweringSamples           []goAstSchemaInventorySample          `json:"lowering_samples"`
	Coverage                  goAstSchemaInventoryCoverage          `json:"coverage"`
	RendererPolicy            goAstSchemaInventoryPolicy            `json:"renderer_policy"`
	CommentTriviaPolicy       goAstSchemaInventoryPolicy            `json:"comment_trivia_policy"`
	PrecedencePolicy          goAstSchemaInventoryPolicy            `json:"precedence_policy"`
	StructuralTokenAssignment *goAstSchemaStructuralTokenAssignment `json:"structural_token_assignment,omitempty"`
	StablePromotionBlockers   []string                              `json:"stable_promotion_blockers"`
	NoStablePromotionClaim    string                                `json:"no_stable_promotion_claim"`
}

// goAstSchemaStructuralTokenAssignment is the pinned alternative node_start
// token table. It is append-only history, not derivable from go/ast, so a
// rebuild preserves it from the checked fixture and validates coverage
// against the rebuilt grammar rules.
type goAstSchemaStructuralTokenAssignment struct {
	AssignmentVersion          int                                    `json:"assignment_version"`
	AlternativeNodeStartOffset int                                    `json:"alternative_node_start_offset"`
	AppendRule                 string                                 `json:"append_rule"`
	AlternativeNodeStartTokens []goAstSchemaAlternativeNodeStartToken `json:"alternative_node_start_tokens"`
}

type goAstSchemaAlternativeNodeStartToken struct {
	NodeKind string `json:"node_kind"`
	TokenID  int    `json:"token_id"`
}

type goAstSchemaInventoryToolchain struct {
	Version       string `json:"version"`
	RuntimeGOOS   string `json:"runtime_goos"`
	RuntimeGOARCH string `json:"runtime_goarch"`
	GOROOT        string `json:"goroot"`
	ParserPackage string `json:"parser_package"`
	ASTPackage    string `json:"ast_package"`
	TokenPackage  string `json:"token_package"`
	SourceParser  string `json:"source_parser"`
}

type goAstSchemaInventorySource struct {
	Role   string `json:"role"`
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int    `json:"bytes"`
}

type goAstSchemaInventoryNodeStruct struct {
	Name              string                      `json:"name"`
	Families          []string                    `json:"families"`
	TokenizableFields []goAstSchemaInventoryField `json:"tokenizable_fields"`
	PositionFields    []goAstSchemaInventoryField `json:"position_fields"`
	SkippedFields     []goAstSchemaInventorySkip  `json:"skipped_fields"`
	Tokenizable       bool                        `json:"tokenizable"`
	GrammarIRRule     *goAstSchemaInventoryRule   `json:"grammar_ir_rule,omitempty"`
}

type goAstSchemaInventoryField struct {
	Name         string                            `json:"name"`
	GoType       string                            `json:"go_type"`
	Required     bool                              `json:"required"`
	Multiple     bool                              `json:"multiple"`
	Policy       string                            `json:"policy"`
	Alternatives []goAstSchemaInventoryAlternative `json:"alternatives,omitempty"`
	Domain       string                            `json:"domain,omitempty"`
}

type goAstSchemaInventoryAlternative struct {
	NodeKind string `json:"node_kind"`
	Terminal string `json:"terminal"`
}

type goAstSchemaInventorySkip struct {
	Name   string `json:"name"`
	GoType string `json:"go_type"`
	Reason string `json:"reason"`
}

type goAstSchemaInventoryFamily struct {
	Name    string   `json:"name"`
	Members []string `json:"members"`
}

type goAstSchemaInventoryDomain struct {
	Name   string                      `json:"name"`
	Source string                      `json:"source"`
	Values []goAstSchemaInventoryValue `json:"values"`
}

type goAstSchemaInventoryValue struct {
	Name   string `json:"name"`
	Lexeme string `json:"lexeme"`
}

type goAstSchemaInventoryRule struct {
	Name     string                      `json:"name"`
	NodeKind string                      `json:"node_kind"`
	Fields   []goAstSchemaInventoryField `json:"fields"`
}

type goAstSchemaInventorySample struct {
	Rule                 string   `json:"rule"`
	NodeKind             string   `json:"node_kind"`
	RequiredFields       []string `json:"required_fields"`
	RepeatedFields       []string `json:"repeated_fields"`
	FiniteDomainFields   []string `json:"finite_domain_fields"`
	InterfaceFields      []string `json:"interface_fields"`
	PayloadFields        []string `json:"payload_fields"`
	StructuralModelProbe string   `json:"structural_model_probe"`
}

type goAstSchemaInventoryCoverage struct {
	NodeStructCount       int      `json:"node_struct_count"`
	TokenizableNodeCount  int      `json:"tokenizable_node_count"`
	GrammarIRRuleCount    int      `json:"grammar_ir_rule_count"`
	TokenizableFieldCount int      `json:"tokenizable_field_count"`
	PositionFieldCount    int      `json:"position_field_count"`
	SkippedFieldCount     int      `json:"skipped_field_count"`
	FiniteDomainCount     int      `json:"finite_domain_count"`
	RequiredFamilies      []string `json:"required_families"`
}

type goAstSchemaInventoryPolicy struct {
	Status   string   `json:"status"`
	Blockers []string `json:"blockers"`
}

type goAstSchemaTypeInfo struct {
	Name   string
	Fields []goAstSchemaFieldInfo
}

type goAstSchemaFieldInfo struct {
	Name    string
	Type    string
	Comment string
}

func main() {
	output := flag.String("output", "", "write inventory JSON to this path instead of stdout")
	validate := flag.String("validate", "", "validate a checked inventory fixture")
	selfTest := flag.Bool("self-test", false, "validate the checked go1.24.3 inventory fixture")
	flag.Parse()

	if *selfTest && *validate == "" {
		*validate = goAstSchemaInventoryCheckedFixture
	}
	if *validate != "" {
		report, err := loadGoAstSchemaInventory(*validate)
		if err != nil {
			exitGoAstSchemaInventory("%v", err)
		}
		if err := validateGoAstSchemaInventory(report); err != nil {
			exitGoAstSchemaInventory("%v", err)
		}
		if *selfTest {
			fmt.Println("go_ast_schema_inventory self-test passed")
			return
		}
		if err := writeGoAstSchemaInventoryJSON(*output, report); err != nil {
			exitGoAstSchemaInventory("%v", err)
		}
		return
	}

	report, err := buildGoAstSchemaInventory()
	if err != nil {
		exitGoAstSchemaInventory("%v", err)
	}
	if err := validateGoAstSchemaInventory(report); err != nil {
		exitGoAstSchemaInventory("%v", err)
	}
	if err := writeGoAstSchemaInventoryJSON(*output, report); err != nil {
		exitGoAstSchemaInventory("%v", err)
	}
}

func buildGoAstSchemaInventory() (*goAstSchemaInventoryReport, error) {
	goroot := runtime.GOROOT()
	astPath := filepath.Join(goroot, "src", "go", "ast", "ast.go")
	tokenPath := filepath.Join(goroot, "src", "go", "token", "token.go")

	astFile, astSource, err := parseGoAstSchemaInventoryFile(astPath)
	if err != nil {
		return nil, err
	}
	tokenFile, tokenSource, err := parseGoAstSchemaInventoryFile(tokenPath)
	if err != nil {
		return nil, err
	}

	structs := collectGoAstSchemaStructs(astFile)
	families := collectGoAstSchemaFamilies(astFile)
	finiteDomains := collectGoAstSchemaFiniteDomains(tokenFile)
	addGoAstSchemaDomains(&finiteDomains)

	nodeStructs := make([]goAstSchemaInventoryNodeStruct, 0, len(structs))
	for _, name := range sortedGoAstSchemaTypeNames(structs) {
		node := buildGoAstSchemaNodeStruct(name, structs[name], families, finiteDomains)
		nodeStructs = append(nodeStructs, node)
	}
	sort.Slice(nodeStructs, func(left, right int) bool {
		return nodeStructs[left].Name < nodeStructs[right].Name
	})

	rules := make([]goAstSchemaInventoryRule, 0)
	for _, node := range nodeStructs {
		if node.GrammarIRRule != nil {
			rules = append(rules, *node.GrammarIRRule)
		}
	}
	sort.Slice(rules, func(left, right int) bool {
		return rules[left].Name < rules[right].Name
	})

	familyReports := make([]goAstSchemaInventoryFamily, 0, len(families))
	for _, name := range []string{"Expr", "Stmt", "Decl", "Spec"} {
		members := append([]string{}, families[name]...)
		sort.Strings(members)
		familyReports = append(familyReports, goAstSchemaInventoryFamily{
			Name:    name,
			Members: members,
		})
	}

	report := &goAstSchemaInventoryReport{
		Tool:         goAstSchemaInventoryTool,
		Version:      goAstSchemaInventoryVersion,
		ArtifactKind: goAstSchemaInventoryArtifactKind,
		Status:       "ready_with_blockers",
		GrammarID:    goAstSchemaInventoryGrammarID,
		Go: goAstSchemaInventoryToolchain{
			Version:       runtime.Version(),
			RuntimeGOOS:   runtime.GOOS,
			RuntimeGOARCH: runtime.GOARCH,
			GOROOT:        "$GOROOT",
			ParserPackage: "go/parser",
			ASTPackage:    "go/ast",
			TokenPackage:  "go/token",
			SourceParser:  "parser.ParseFile",
		},
		Sources: []goAstSchemaInventorySource{
			{Role: "go_ast", Path: "$GOROOT/src/go/ast/ast.go", SHA256: sha256GoAstSchemaInventory(astSource), Bytes: len(astSource)},
			{Role: "go_token", Path: "$GOROOT/src/go/token/token.go", SHA256: sha256GoAstSchemaInventory(tokenSource), Bytes: len(tokenSource)},
		},
		NodeStructs:       nodeStructs,
		InterfaceFamilies: familyReports,
		FiniteDomains:     finiteDomains,
		GrammarIRRules:    rules,
		LoweringSamples:   buildGoAstSchemaLoweringSamples(rules),
		RendererPolicy: goAstSchemaInventoryPolicy{
			Status: "classified_with_blockers",
			Blockers: []string{
				"token.Pos fields are split into fixed-token, finite-domain-token, syntax-presence, bad-node, parser-position, and comment/trivia policy categories, but the raw Go tokenizer generator does not yet consume those categories as renderer authority",
				"go/printer and gofmt remain sidecar validation for canonical source rendering",
			},
		},
		CommentTriviaPolicy: goAstSchemaInventoryPolicy{
			Status: "explicit_sidecar_metadata",
			Blockers: []string{
				"File.Comments plus node Doc/Comment fields are inventoried, but trivia placement remains parser/printer sidecar policy",
			},
		},
		PrecedencePolicy: goAstSchemaInventoryPolicy{
			Status: "blocked",
			Blockers: []string{
				"operator precedence and parenthesized-expression preservation are inventoried through token domains and ParenExpr, but canonical render precedence is not yet generated-codec authority",
			},
		},
		StablePromotionBlockers: []string{
			"raw Go syntax state machine must consume this inventory's tokenizable fields and classified renderer-position policies through token layout and generated-codec encode/decode/render/reparse evidence",
			"classified renderer-position, precedence, and comment/trivia policies are not yet generated-codec authority",
			"C# syntax metadata must remain the cross-target check before stable syntax backend promotion",
		},
		NoStablePromotionClaim: goAstSchemaInventoryNoStablePromotion,
	}
	assignment, err := preservedGoAstSchemaStructuralTokenAssignment()
	if err != nil {
		return nil, err
	}
	report.StructuralTokenAssignment = assignment
	report.Coverage = summarizeGoAstSchemaInventory(report)
	return report, nil
}

// preservedGoAstSchemaStructuralTokenAssignment carries the pinned
// append-only alternative node_start token table forward from the checked
// fixture: pinned ids are layout history and must never be re-derived. The
// fixture is resolved from the repository root, walking up from the working
// directory so `go test` inside scripts/analysis keeps finding it.
func preservedGoAstSchemaStructuralTokenAssignment() (*goAstSchemaStructuralTokenAssignment, error) {
	fixturePath := goAstSchemaInventoryCheckedFixture
	if _, err := os.Stat(fixturePath); err != nil {
		for _, prefix := range []string{"..", filepath.Join("..", "..")} {
			candidate := filepath.Join(prefix, goAstSchemaInventoryCheckedFixture)
			if _, err := os.Stat(candidate); err == nil {
				fixturePath = candidate
				break
			}
		}
	}
	existing, err := loadGoAstSchemaInventory(fixturePath)
	if err != nil {
		return nil, fmt.Errorf("read checked fixture for the pinned structural token assignment: %w", err)
	}
	if existing.StructuralTokenAssignment == nil {
		return nil, fmt.Errorf("checked fixture %s carries no structural_token_assignment to preserve", fixturePath)
	}
	return existing.StructuralTokenAssignment, nil
}

func parseGoAstSchemaInventoryFile(path string) (*ast.File, []byte, error) {
	source, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, fmt.Errorf("read %s: %w", path, err)
	}
	parsed, err := parser.ParseFile(token.NewFileSet(), path, source, parser.ParseComments)
	if err != nil {
		return nil, nil, fmt.Errorf("parse %s: %w", path, err)
	}
	return parsed, source, nil
}

func collectGoAstSchemaStructs(file *ast.File) map[string]goAstSchemaTypeInfo {
	structs := map[string]goAstSchemaTypeInfo{}
	for _, decl := range file.Decls {
		gen, ok := decl.(*ast.GenDecl)
		if !ok || gen.Tok != token.TYPE {
			continue
		}
		for _, spec := range gen.Specs {
			typeSpec, ok := spec.(*ast.TypeSpec)
			if !ok {
				continue
			}
			switch typeExpr := typeSpec.Type.(type) {
			case *ast.StructType:
				structs[typeSpec.Name.Name] = goAstSchemaTypeInfo{
					Name:   typeSpec.Name.Name,
					Fields: collectGoAstSchemaStructFields(typeExpr),
				}
			}
		}
	}
	return structs
}

func collectGoAstSchemaStructFields(structType *ast.StructType) []goAstSchemaFieldInfo {
	fields := make([]goAstSchemaFieldInfo, 0)
	if structType.Fields == nil {
		return fields
	}
	for _, field := range structType.Fields.List {
		typeName := goAstSchemaExprString(field.Type)
		comment := goAstSchemaFieldComment(field)
		if len(field.Names) == 0 {
			fields = append(fields, goAstSchemaFieldInfo{
				Name:    typeName,
				Type:    typeName,
				Comment: comment,
			})
			continue
		}
		for _, name := range field.Names {
			fields = append(fields, goAstSchemaFieldInfo{
				Name:    name.Name,
				Type:    typeName,
				Comment: comment,
			})
		}
	}
	return fields
}

func collectGoAstSchemaFamilies(file *ast.File) map[string][]string {
	families := map[string][]string{
		"Expr": {},
		"Stmt": {},
		"Decl": {},
		"Spec": {},
	}
	for _, decl := range file.Decls {
		fn, ok := decl.(*ast.FuncDecl)
		if !ok || fn.Recv == nil || len(fn.Recv.List) != 1 {
			continue
		}
		receiver := fn.Recv.List[0].Type
		star, ok := receiver.(*ast.StarExpr)
		if !ok {
			continue
		}
		ident, ok := star.X.(*ast.Ident)
		if !ok {
			continue
		}
		switch fn.Name.Name {
		case "exprNode":
			families["Expr"] = append(families["Expr"], ident.Name)
		case "stmtNode":
			families["Stmt"] = append(families["Stmt"], ident.Name)
		case "declNode":
			families["Decl"] = append(families["Decl"], ident.Name)
		case "specNode":
			families["Spec"] = append(families["Spec"], ident.Name)
		}
	}
	for family := range families {
		sort.Strings(families[family])
	}
	return families
}

func collectGoAstSchemaFiniteDomains(tokenFile *ast.File) []goAstSchemaInventoryDomain {
	tokenValues := collectGoAstSchemaTokenValues(tokenFile)
	return []goAstSchemaInventoryDomain{
		{Name: "go_token_all_tokens", Source: "go/token.Token", Values: tokenValues},
		{Name: "go_token_basic_literal_kinds", Source: "go/token.Token", Values: filterGoAstSchemaTokenValues(tokenValues, "INT", "FLOAT", "IMAG", "CHAR", "STRING")},
		{Name: "go_token_declaration_tokens", Source: "go/token.Token", Values: filterGoAstSchemaTokenValues(tokenValues, "IMPORT", "CONST", "TYPE", "VAR")},
		{Name: "go_token_assignment_tokens", Source: "go/token.Token", Values: filterGoAstSchemaTokenValues(tokenValues, "ASSIGN", "DEFINE", "ADD_ASSIGN", "SUB_ASSIGN", "MUL_ASSIGN", "QUO_ASSIGN", "REM_ASSIGN", "AND_ASSIGN", "OR_ASSIGN", "XOR_ASSIGN", "SHL_ASSIGN", "SHR_ASSIGN", "AND_NOT_ASSIGN")},
		{Name: "go_token_inc_dec_tokens", Source: "go/token.Token", Values: filterGoAstSchemaTokenValues(tokenValues, "INC", "DEC")},
		{Name: "go_token_branch_tokens", Source: "go/token.Token", Values: filterGoAstSchemaTokenValues(tokenValues, "BREAK", "CONTINUE", "GOTO", "FALLTHROUGH")},
		{Name: "go_token_range_assignment_tokens", Source: "go/token.Token (RangeStmt.Tok; ILLEGAL marks a bare `for range` without Key/Value)", Values: filterGoAstSchemaTokenValues(tokenValues, "ILLEGAL", "ASSIGN", "DEFINE")},
		{Name: "go_token_unary_operator_tokens", Source: "go/token.Token", Values: filterGoAstSchemaTokenValues(tokenValues, "ADD", "SUB", "NOT", "XOR", "AND", "ARROW", "TILDE")},
		{Name: "go_token_binary_operator_tokens", Source: "go/token.Token", Values: filterGoAstSchemaTokenValues(tokenValues, "ADD", "SUB", "MUL", "QUO", "REM", "AND", "OR", "XOR", "SHL", "SHR", "AND_NOT", "LAND", "LOR", "EQL", "NEQ", "LSS", "LEQ", "GTR", "GEQ")},
		{Name: "go_ast_channel_directions", Source: "go/ast.ChanDir", Values: []goAstSchemaInventoryValue{
			{Name: "SEND", Lexeme: "send"},
			{Name: "RECV", Lexeme: "recv"},
			{Name: "SEND_RECV", Lexeme: "send_recv"},
		}},
		{Name: "go_syntax_presence_call_expr_ellipsis", Source: "token.Pos.IsValid on CallExpr.Ellipsis variadic call marker", Values: goAstSchemaPresenceValues()},
		{Name: "go_syntax_presence_func_type_func", Source: "token.Pos.IsValid on FuncType.Func keyword marker", Values: goAstSchemaPresenceValues()},
		{Name: "go_syntax_presence_gen_decl_group", Source: "token.Pos.IsValid on GenDecl grouped-declaration delimiters", Values: goAstSchemaPresenceValues()},
		{Name: "go_syntax_presence_type_spec_alias", Source: "token.Pos.IsValid on TypeSpec.Assign alias marker", Values: goAstSchemaPresenceValues()},
	}
}

func addGoAstSchemaDomains(domains *[]goAstSchemaInventoryDomain) {
	sort.Slice(*domains, func(left, right int) bool {
		return (*domains)[left].Name < (*domains)[right].Name
	})
}

func collectGoAstSchemaTokenValues(tokenFile *ast.File) []goAstSchemaInventoryValue {
	names := []string{}
	for _, decl := range tokenFile.Decls {
		gen, ok := decl.(*ast.GenDecl)
		if !ok || gen.Tok != token.CONST {
			continue
		}
		for _, spec := range gen.Specs {
			valueSpec, ok := spec.(*ast.ValueSpec)
			if !ok {
				continue
			}
			for _, name := range valueSpec.Names {
				if name.Name == "" || strings.HasSuffix(name.Name, "_beg") || strings.HasSuffix(name.Name, "_end") {
					continue
				}
				if name.Name != strings.ToUpper(name.Name) {
					continue
				}
				names = append(names, name.Name)
			}
		}
	}
	values := make([]goAstSchemaInventoryValue, 0, len(names))
	for _, name := range names {
		values = append(values, goAstSchemaInventoryValue{
			Name:   name,
			Lexeme: goAstSchemaTokenLexeme(name),
		})
	}
	return values
}

func filterGoAstSchemaTokenValues(values []goAstSchemaInventoryValue, names ...string) []goAstSchemaInventoryValue {
	wanted := map[string]bool{}
	for _, name := range names {
		wanted[name] = true
	}
	filtered := make([]goAstSchemaInventoryValue, 0, len(names))
	for _, value := range values {
		if wanted[value.Name] {
			filtered = append(filtered, value)
		}
	}
	return filtered
}

func goAstSchemaTokenLexeme(name string) string {
	tokenLexemes := map[string]string{
		"ILLEGAL": "ILLEGAL", "EOF": "EOF", "COMMENT": "COMMENT",
		"IDENT": "IDENT", "INT": "INT", "FLOAT": "FLOAT", "IMAG": "IMAG", "CHAR": "CHAR", "STRING": "STRING",
		"ADD": "+", "SUB": "-", "MUL": "*", "QUO": "/", "REM": "%",
		"AND": "&", "OR": "|", "XOR": "^", "SHL": "<<", "SHR": ">>", "AND_NOT": "&^",
		"ADD_ASSIGN": "+=", "SUB_ASSIGN": "-=", "MUL_ASSIGN": "*=", "QUO_ASSIGN": "/=", "REM_ASSIGN": "%=",
		"AND_ASSIGN": "&=", "OR_ASSIGN": "|=", "XOR_ASSIGN": "^=", "SHL_ASSIGN": "<<=", "SHR_ASSIGN": ">>=", "AND_NOT_ASSIGN": "&^=",
		"LAND": "&&", "LOR": "||", "ARROW": "<-", "INC": "++", "DEC": "--",
		"EQL": "==", "LSS": "<", "GTR": ">", "ASSIGN": "=", "NOT": "!",
		"NEQ": "!=", "LEQ": "<=", "GEQ": ">=", "DEFINE": ":=", "ELLIPSIS": "...",
		"LPAREN": "(", "LBRACK": "[", "LBRACE": "{", "COMMA": ",", "PERIOD": ".",
		"RPAREN": ")", "RBRACK": "]", "RBRACE": "}", "SEMICOLON": ";", "COLON": ":",
		"BREAK": "break", "CASE": "case", "CHAN": "chan", "CONST": "const", "CONTINUE": "continue",
		"DEFAULT": "default", "DEFER": "defer", "ELSE": "else", "FALLTHROUGH": "fallthrough", "FOR": "for",
		"FUNC": "func", "GO": "go", "GOTO": "goto", "IF": "if", "IMPORT": "import",
		"INTERFACE": "interface", "MAP": "map", "PACKAGE": "package", "RANGE": "range", "RETURN": "return",
		"SELECT": "select", "STRUCT": "struct", "SWITCH": "switch", "TYPE": "type", "VAR": "var", "TILDE": "~",
	}
	if lexeme, ok := tokenLexemes[name]; ok {
		return lexeme
	}
	return name
}

func buildGoAstSchemaNodeStruct(name string, info goAstSchemaTypeInfo, families map[string][]string, domains []goAstSchemaInventoryDomain) goAstSchemaInventoryNodeStruct {
	node := goAstSchemaInventoryNodeStruct{
		Name:     name,
		Families: familiesForGoAstSchemaNode(name, families),
	}
	for _, field := range info.Fields {
		if !isExportedGoAstSchemaName(field.Name) {
			continue
		}
		classified := classifyGoAstSchemaField(name, field, families, domains)
		switch {
		case classified.tokenizable != nil:
			node.TokenizableFields = append(node.TokenizableFields, *classified.tokenizable)
		case classified.position != nil:
			node.PositionFields = append(node.PositionFields, *classified.position)
		case classified.skip != nil:
			node.SkippedFields = append(node.SkippedFields, *classified.skip)
		}
	}
	node.Tokenizable = len(node.TokenizableFields) > 0
	if node.Tokenizable {
		ruleFields := make([]goAstSchemaInventoryField, 0, len(node.TokenizableFields))
		ruleFields = append(ruleFields, node.TokenizableFields...)
		ruleFields = append(ruleFields, goAstSchemaSyntheticTriviaFields(node.Name)...)
		rule := goAstSchemaInventoryRule{
			Name:     node.Name,
			NodeKind: node.Name,
			Fields:   ruleFields,
		}
		node.GrammarIRRule = &rule
	}
	return node
}

// goAstSchemaSyntheticTriviaFields appends the parser-derived comment and
// layout trivia metadata fields to the grammar rules whose projections the
// source-instance projector synthesizes. These fields do not exist on the
// go/ast structs (node_structs tokenizable_fields stay go/ast-exact); they
// carry root-only comment anchors and blank-line/line-break separators as
// optional repeated integer index payloads so comments and layout stay
// representable in the unified inventory grammar. Single-alternative integer
// payloads never mint alternative node_start tokens, so the pinned
// structural token assignment is unaffected.
func goAstSchemaSyntheticTriviaFields(nodeName string) []goAstSchemaInventoryField {
	triviaField := func(fieldName string, nodeKind string) goAstSchemaInventoryField {
		return goAstSchemaInventoryField{
			Name:     fieldName,
			GoType:   "[]int",
			Required: false,
			Multiple: true,
			Policy:   "synthesized_comment_trivia_metadata",
			Alternatives: []goAstSchemaInventoryAlternative{
				{NodeKind: nodeKind, Terminal: "integer_payload"},
			},
		}
	}
	switch nodeName {
	case "File":
		// gofmt preserves at most one blank line between top-level
		// declarations and forces one whenever the declaration token changes,
		// the declaration carries a doc comment, or the declaration is a
		// multi-line function. The only free choice is therefore "no blank
		// line" between consecutive same-kind declarations, so the field
		// records the suppressed separators (sparse) rather than the present
		// ones. `CommentBefore` anchors the root-only comment groups that sit
		// between declarations; index `len(Decls)` anchors a trailing group.
		return []goAstSchemaInventoryField{
			triviaField("CommentBefore", "file_decl_comment_before_index"),
			triviaField("NoBlankLineBefore", "file_decl_no_blank_line_before_index"),
		}
	case "BlockStmt":
		return []goAstSchemaInventoryField{
			triviaField("CommentBefore", "block_stmt_comment_before_index"),
			triviaField("TrailingComment", "block_stmt_trailing_comment_index"),
			triviaField("BlankLineBefore", "block_stmt_blank_line_before_index"),
		}
	case "CaseClause":
		return []goAstSchemaInventoryField{
			triviaField("CommentBefore", "case_clause_body_comment_before_index"),
			triviaField("TrailingComment", "case_clause_body_trailing_comment_index"),
			triviaField("BlankLineBefore", "case_clause_body_blank_line_before_index"),
		}
	case "CommClause":
		return []goAstSchemaInventoryField{
			triviaField("CommentBefore", "comm_clause_body_comment_before_index"),
			triviaField("TrailingComment", "comm_clause_body_trailing_comment_index"),
			triviaField("BlankLineBefore", "comm_clause_body_blank_line_before_index"),
		}
	case "IfStmt":
		return []goAstSchemaInventoryField{
			triviaField("TrailingComment", "if_stmt_trailing_comment_index"),
		}
	case "FieldList":
		return []goAstSchemaInventoryField{
			triviaField("BlankLineBefore", "field_list_blank_line_before_index"),
		}
	case "CompositeLit":
		return []goAstSchemaInventoryField{
			triviaField("LineBreakBefore", "composite_lit_line_break_before_index"),
		}
	case "CallExpr":
		return []goAstSchemaInventoryField{
			triviaField("LineBreakBefore", "call_expr_line_break_before_index"),
		}
	default:
		return nil
	}
}

type goAstSchemaClassifiedField struct {
	tokenizable *goAstSchemaInventoryField
	position    *goAstSchemaInventoryField
	skip        *goAstSchemaInventorySkip
}

func classifyGoAstSchemaField(owner string, field goAstSchemaFieldInfo, families map[string][]string, domains []goAstSchemaInventoryDomain) goAstSchemaClassifiedField {
	typeInfo := parseGoAstSchemaType(field.Type)
	if typeInfo.base == "token.Pos" {
		if domain := presenceDomainForGoAstSchemaPositionField(owner, field.Name); domain != "" {
			return goAstSchemaClassifiedField{tokenizable: &goAstSchemaInventoryField{
				Name:         field.Name,
				GoType:       field.Type,
				Required:     false,
				Multiple:     false,
				Policy:       "syntax_presence_decision",
				Alternatives: presenceAlternatives(domain, domains),
				Domain:       domain,
			}}
		}
		return goAstSchemaClassifiedField{position: &goAstSchemaInventoryField{
			Name:     field.Name,
			GoType:   field.Type,
			Required: false,
			Multiple: false,
			Policy:   positionPolicyForGoAstSchemaPositionField(owner, field.Name),
		}}
	}
	if field.Name == "Obj" || field.Name == "Scope" || field.Name == "Imports" || field.Name == "Unresolved" || field.Name == "FileStart" || field.Name == "FileEnd" {
		return goAstSchemaClassifiedField{skip: &goAstSchemaInventorySkip{Name: field.Name, GoType: field.Type, Reason: "parser semantic index or source-position convenience field"}}
	}
	if typeInfo.unsupported {
		return goAstSchemaClassifiedField{skip: &goAstSchemaInventorySkip{Name: field.Name, GoType: field.Type, Reason: "unsupported non-syntax container for GrammarIR v1"}}
	}

	required := !typeInfo.optional && !containsGoAstSchemaNilComment(field.Comment)
	policy := "required_by_go_ast_field_type"
	if !required {
		policy = "optional_pointer_interface_slice_or_nil_comment"
	}
	if typeInfo.multiple {
		policy = "repeated_slice_field"
	}

	alternatives, domain := alternativesForGoAstSchemaField(owner, field, typeInfo, families, domains)
	if len(alternatives) == 0 {
		return goAstSchemaClassifiedField{skip: &goAstSchemaInventorySkip{Name: field.Name, GoType: field.Type, Reason: "no tokenizable GrammarIR alternative for field type"}}
	}
	return goAstSchemaClassifiedField{tokenizable: &goAstSchemaInventoryField{
		Name:         field.Name,
		GoType:       field.Type,
		Required:     required,
		Multiple:     typeInfo.multiple,
		Policy:       policy,
		Alternatives: alternatives,
		Domain:       domain,
	}}
}

type goAstSchemaParsedType struct {
	base        string
	optional    bool
	multiple    bool
	unsupported bool
}

func parseGoAstSchemaType(typeName string) goAstSchemaParsedType {
	parsed := goAstSchemaParsedType{base: typeName}
	for strings.HasPrefix(parsed.base, "[]") {
		parsed.multiple = true
		parsed.optional = true
		parsed.base = strings.TrimPrefix(parsed.base, "[]")
	}
	if strings.HasPrefix(parsed.base, "*") {
		parsed.optional = true
		parsed.base = strings.TrimPrefix(parsed.base, "*")
	}
	if strings.HasPrefix(parsed.base, "map[") {
		parsed.unsupported = true
	}
	return parsed
}

func alternativesForGoAstSchemaField(owner string, field goAstSchemaFieldInfo, typeInfo goAstSchemaParsedType, families map[string][]string, domains []goAstSchemaInventoryDomain) ([]goAstSchemaInventoryAlternative, string) {
	switch typeInfo.base {
	case "Expr", "Stmt", "Decl", "Spec":
		return familyAlternatives(families[typeInfo.base]), "go_ast_family_" + strings.ToLower(typeInfo.base)
	case "string":
		terminal := "string_payload"
		nodeKind := owner + "." + field.Name + ".string"
		if owner == "Ident" && field.Name == "Name" || owner == "File" && field.Name == "GoVersion" {
			terminal = "identifier_payload"
			nodeKind = owner + "." + field.Name + ".identifier"
		}
		return []goAstSchemaInventoryAlternative{{NodeKind: nodeKind, Terminal: terminal}}, ""
	case "bool":
		return []goAstSchemaInventoryAlternative{{NodeKind: owner + "." + field.Name + ".bool", Terminal: "boolean_payload"}}, "go_boolean"
	case "token.Token":
		domain := domainForGoAstSchemaTokenField(owner, field.Name)
		return domainAlternatives(domain, domains), domain
	case "ChanDir":
		return domainAlternatives("go_ast_channel_directions", domains), "go_ast_channel_directions"
	default:
		if isKnownGoAstSchemaConcreteNode(typeInfo.base, families) {
			return []goAstSchemaInventoryAlternative{{NodeKind: typeInfo.base, Terminal: "nonterminal"}}, ""
		}
	}
	return nil, ""
}

func presenceDomainForGoAstSchemaPositionField(owner string, field string) string {
	switch {
	case owner == "CallExpr" && field == "Ellipsis":
		return "go_syntax_presence_call_expr_ellipsis"
	case owner == "FuncType" && field == "Func":
		return "go_syntax_presence_func_type_func"
	case owner == "GenDecl" && (field == "Lparen" || field == "Rparen"):
		return "go_syntax_presence_gen_decl_group"
	case owner == "TypeSpec" && field == "Assign":
		return "go_syntax_presence_type_spec_alias"
	default:
		return ""
	}
}

func positionPolicyForGoAstSchemaPositionField(owner string, field string) string {
	if policy, ok := goAstSchemaPositionFieldPolicies[owner+"."+field]; ok {
		return policy
	}
	return goAstSchemaPositionPolicyUnclassified
}

func domainForGoAstSchemaTokenField(owner string, field string) string {
	switch {
	case owner == "BasicLit" && field == "Kind":
		return "go_token_basic_literal_kinds"
	case owner == "GenDecl" && field == "Tok":
		return "go_token_declaration_tokens"
	case owner == "BranchStmt" && field == "Tok":
		return "go_token_branch_tokens"
	case owner == "IncDecStmt" && field == "Tok":
		return "go_token_inc_dec_tokens"
	case owner == "AssignStmt" && field == "Tok":
		return "go_token_assignment_tokens"
	case owner == "RangeStmt" && field == "Tok":
		return "go_token_range_assignment_tokens"
	case owner == "UnaryExpr" && field == "Op":
		return "go_token_unary_operator_tokens"
	case owner == "BinaryExpr" && field == "Op":
		return "go_token_binary_operator_tokens"
	default:
		return "go_token_all_tokens"
	}
}

func familyAlternatives(members []string) []goAstSchemaInventoryAlternative {
	alternatives := make([]goAstSchemaInventoryAlternative, 0, len(members))
	for _, member := range members {
		if strings.HasPrefix(member, "Bad") {
			continue
		}
		alternatives = append(alternatives, goAstSchemaInventoryAlternative{
			NodeKind: member,
			Terminal: "nonterminal",
		})
	}
	return alternatives
}

func domainAlternatives(domain string, domains []goAstSchemaInventoryDomain) []goAstSchemaInventoryAlternative {
	for _, item := range domains {
		if item.Name != domain {
			continue
		}
		alternatives := make([]goAstSchemaInventoryAlternative, 0, len(item.Values))
		for _, value := range item.Values {
			alternatives = append(alternatives, goAstSchemaInventoryAlternative{
				NodeKind: domain + "." + value.Name,
				Terminal: "identifier_payload",
			})
		}
		return alternatives
	}
	return nil
}

func goAstSchemaPresenceValues() []goAstSchemaInventoryValue {
	return []goAstSchemaInventoryValue{
		{Name: "ABSENT", Lexeme: "false"},
		{Name: "PRESENT", Lexeme: "true"},
	}
}

func presenceAlternatives(domain string, domains []goAstSchemaInventoryDomain) []goAstSchemaInventoryAlternative {
	for _, item := range domains {
		if item.Name != domain {
			continue
		}
		alternatives := make([]goAstSchemaInventoryAlternative, 0, len(item.Values))
		for _, value := range item.Values {
			alternatives = append(alternatives, goAstSchemaInventoryAlternative{
				NodeKind: domain + "." + value.Name,
				Terminal: "boolean_payload",
			})
		}
		return alternatives
	}
	return nil
}

func buildGoAstSchemaLoweringSamples(rules []goAstSchemaInventoryRule) []goAstSchemaInventorySample {
	wanted := map[string]bool{
		"File": true, "FuncDecl": true, "GenDecl": true, "ValueSpec": true,
		"BasicLit": true, "BinaryExpr": true, "AssignStmt": true, "RangeStmt": true,
		"ChanType": true, "Field": true, "FuncType": true,
	}
	samples := make([]goAstSchemaInventorySample, 0, len(wanted))
	for _, rule := range rules {
		if !wanted[rule.Name] {
			continue
		}
		sample := goAstSchemaInventorySample{
			Rule:                 rule.Name,
			NodeKind:             rule.NodeKind,
			StructuralModelProbe: "lower_inventory_rule_to_grammar_ir_structural_model",
		}
		for _, field := range rule.Fields {
			if field.Required {
				sample.RequiredFields = append(sample.RequiredFields, field.Name)
			}
			if field.Multiple {
				sample.RepeatedFields = append(sample.RepeatedFields, field.Name)
			}
			if field.Domain != "" {
				sample.FiniteDomainFields = append(sample.FiniteDomainFields, field.Name)
			}
			if strings.HasPrefix(field.Domain, "go_ast_family_") {
				sample.InterfaceFields = append(sample.InterfaceFields, field.Name)
			}
			for _, alternative := range field.Alternatives {
				if alternative.Terminal != "nonterminal" {
					sample.PayloadFields = append(sample.PayloadFields, field.Name)
					break
				}
			}
		}
		samples = append(samples, sample)
	}
	sort.Slice(samples, func(left, right int) bool {
		return samples[left].Rule < samples[right].Rule
	})
	return samples
}

func summarizeGoAstSchemaInventory(report *goAstSchemaInventoryReport) goAstSchemaInventoryCoverage {
	coverage := goAstSchemaInventoryCoverage{
		NodeStructCount:    len(report.NodeStructs),
		GrammarIRRuleCount: len(report.GrammarIRRules),
		FiniteDomainCount:  len(report.FiniteDomains),
		RequiredFamilies:   []string{"Expr", "Stmt", "Decl", "Spec"},
	}
	for _, node := range report.NodeStructs {
		if node.Tokenizable {
			coverage.TokenizableNodeCount++
		}
		coverage.TokenizableFieldCount += len(node.TokenizableFields)
		coverage.PositionFieldCount += len(node.PositionFields)
		coverage.SkippedFieldCount += len(node.SkippedFields)
	}
	return coverage
}

func validateGoAstSchemaInventory(report *goAstSchemaInventoryReport) error {
	if report.ArtifactKind != goAstSchemaInventoryArtifactKind {
		return fmt.Errorf("expected artifact_kind %s, got %s", goAstSchemaInventoryArtifactKind, report.ArtifactKind)
	}
	if report.Version != goAstSchemaInventoryVersion {
		return fmt.Errorf("expected version %d, got %d", goAstSchemaInventoryVersion, report.Version)
	}
	if report.Go.Version != "go1.24.3" {
		return fmt.Errorf("expected pinned Go toolchain go1.24.3, got %s", report.Go.Version)
	}
	if report.GrammarID != goAstSchemaInventoryGrammarID {
		return fmt.Errorf("expected grammar_id %s, got %s", goAstSchemaInventoryGrammarID, report.GrammarID)
	}
	if len(report.NodeStructs) == 0 || len(report.GrammarIRRules) == 0 {
		return errors.New("inventory must contain node structs and GrammarIR rules")
	}
	families := map[string]bool{}
	for _, family := range report.InterfaceFamilies {
		if len(family.Members) == 0 {
			return fmt.Errorf("interface family %s has no members", family.Name)
		}
		families[family.Name] = true
	}
	for _, required := range []string{"Expr", "Stmt", "Decl", "Spec"} {
		if !families[required] {
			return fmt.Errorf("missing interface family %s", required)
		}
	}
	domains := map[string]bool{}
	for _, domain := range report.FiniteDomains {
		if len(domain.Values) == 0 {
			return fmt.Errorf("finite domain %s has no values", domain.Name)
		}
		domains[domain.Name] = true
	}
	for _, required := range []string{"go_token_declaration_tokens", "go_token_branch_tokens", "go_token_basic_literal_kinds", "go_ast_channel_directions", "go_syntax_presence_call_expr_ellipsis", "go_syntax_presence_func_type_func", "go_syntax_presence_gen_decl_group", "go_syntax_presence_type_spec_alias"} {
		if !domains[required] {
			return fmt.Errorf("missing finite domain %s", required)
		}
	}
	ruleNames := map[string]bool{}
	for _, rule := range report.GrammarIRRules {
		if rule.Name == "" || rule.NodeKind == "" {
			return errors.New("GrammarIR rule names and node kinds must be present")
		}
		if ruleNames[rule.Name] {
			return fmt.Errorf("duplicate GrammarIR rule %s", rule.Name)
		}
		ruleNames[rule.Name] = true
		for _, field := range rule.Fields {
			if field.Name == "" || len(field.Alternatives) == 0 {
				return fmt.Errorf("rule %s has a field without alternatives", rule.Name)
			}
		}
	}
	for _, required := range []string{"File", "FuncDecl", "GenDecl", "BasicLit", "BinaryExpr", "AssignStmt", "ChanType"} {
		if !ruleNames[required] {
			return fmt.Errorf("missing lowering rule %s", required)
		}
	}
	if err := validateGoAstSchemaPositionPolicies(report); err != nil {
		return err
	}
	if err := validateGoAstSchemaStructuralTokenAssignment(report); err != nil {
		return err
	}
	if len(report.StablePromotionBlockers) == 0 || report.NoStablePromotionClaim == "" {
		return errors.New("inventory must keep stable-promotion blockers explicit")
	}
	return nil
}

func validateGoAstSchemaStructuralTokenAssignment(report *goAstSchemaInventoryReport) error {
	assignment := report.StructuralTokenAssignment
	if assignment == nil {
		return errors.New("inventory must pin the structural token assignment")
	}
	multiAlternativeKinds := map[string]bool{}
	for _, rule := range report.GrammarIRRules {
		for _, field := range rule.Fields {
			if len(field.Alternatives) <= 1 {
				continue
			}
			for _, alternative := range field.Alternatives {
				if alternative.Terminal == "nonterminal" {
					multiAlternativeKinds[alternative.NodeKind] = true
				}
			}
		}
	}
	pinned := map[string]bool{}
	seenIDs := map[int]bool{}
	for _, token := range assignment.AlternativeNodeStartTokens {
		if token.NodeKind == "" {
			return errors.New("structural token assignment entries must carry a node kind")
		}
		if pinned[token.NodeKind] {
			return fmt.Errorf("duplicate pinned node kind %s", token.NodeKind)
		}
		pinned[token.NodeKind] = true
		if seenIDs[token.TokenID] {
			return fmt.Errorf("duplicate pinned token id %d for %s", token.TokenID, token.NodeKind)
		}
		seenIDs[token.TokenID] = true
		if !multiAlternativeKinds[token.NodeKind] {
			return fmt.Errorf("pinned node kind %s is not a multi-alternative nonterminal node kind in this inventory", token.NodeKind)
		}
	}
	for kind := range multiAlternativeKinds {
		if !pinned[kind] {
			return fmt.Errorf("multi-alternative nonterminal node kind %s has no pinned token id; %s", kind, assignment.AppendRule)
		}
	}
	return nil
}

func validateGoAstSchemaPositionPolicies(report *goAstSchemaInventoryReport) error {
	allowed := map[string]bool{
		goAstSchemaPositionPolicyBadNodeSourceRange:    true,
		goAstSchemaPositionPolicyCommentTriviaSidecar:  true,
		goAstSchemaPositionPolicyParserPositionSidecar: true,
		goAstSchemaPositionPolicyFixedSyntaxToken:      true,
		goAstSchemaPositionPolicyFiniteDomainToken:     true,
		goAstSchemaPositionPolicySyntaxPresence:        true,
	}
	seen := map[string]bool{}
	for _, node := range report.NodeStructs {
		for _, field := range node.PositionFields {
			path := node.Name + "." + field.Name
			if field.Policy == "" {
				return fmt.Errorf("position field %s must have a renderer-position policy", path)
			}
			if !allowed[field.Policy] {
				return fmt.Errorf("position field %s has unsupported renderer-position policy %s", path, field.Policy)
			}
			seen[field.Policy] = true
		}
	}
	for _, required := range []string{
		goAstSchemaPositionPolicyBadNodeSourceRange,
		goAstSchemaPositionPolicyCommentTriviaSidecar,
		goAstSchemaPositionPolicyParserPositionSidecar,
		goAstSchemaPositionPolicyFixedSyntaxToken,
		goAstSchemaPositionPolicyFiniteDomainToken,
		goAstSchemaPositionPolicySyntaxPresence,
	} {
		if !seen[required] {
			return fmt.Errorf("missing renderer-position policy category %s", required)
		}
	}
	return nil
}

func loadGoAstSchemaInventory(path string) (*goAstSchemaInventoryReport, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read inventory: %w", err)
	}
	var report goAstSchemaInventoryReport
	if err := json.Unmarshal(data, &report); err != nil {
		return nil, fmt.Errorf("parse inventory JSON: %w", err)
	}
	return &report, nil
}

func writeGoAstSchemaInventoryJSON(path string, report *goAstSchemaInventoryReport) error {
	encoded, err := json.MarshalIndent(report, "", "  ")
	if err != nil {
		return err
	}
	encoded = append(encoded, '\n')
	if path == "" {
		_, err := os.Stdout.Write(encoded)
		return err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	return os.WriteFile(path, encoded, 0o644)
}

func goAstSchemaExprString(expr ast.Expr) string {
	switch value := expr.(type) {
	case *ast.Ident:
		return value.Name
	case *ast.SelectorExpr:
		return goAstSchemaExprString(value.X) + "." + value.Sel.Name
	case *ast.StarExpr:
		return "*" + goAstSchemaExprString(value.X)
	case *ast.ArrayType:
		return "[]" + goAstSchemaExprString(value.Elt)
	case *ast.MapType:
		return "map[" + goAstSchemaExprString(value.Key) + "]" + goAstSchemaExprString(value.Value)
	default:
		return fmt.Sprintf("%T", expr)
	}
}

func goAstSchemaFieldComment(field *ast.Field) string {
	parts := []string{}
	if field.Doc != nil {
		parts = append(parts, strings.TrimSpace(field.Doc.Text()))
	}
	if field.Comment != nil {
		parts = append(parts, strings.TrimSpace(field.Comment.Text()))
	}
	return strings.Join(parts, " ")
}

func sortedGoAstSchemaTypeNames(types map[string]goAstSchemaTypeInfo) []string {
	names := make([]string, 0, len(types))
	for name := range types {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}

func familiesForGoAstSchemaNode(name string, families map[string][]string) []string {
	result := []string{}
	for _, family := range []string{"Expr", "Stmt", "Decl", "Spec"} {
		if containsGoAstSchemaString(families[family], name) {
			result = append(result, family)
		}
	}
	return result
}

func isKnownGoAstSchemaConcreteNode(name string, families map[string][]string) bool {
	if name == "File" || name == "Comment" || name == "CommentGroup" || name == "Field" || name == "FieldList" {
		return true
	}
	for _, members := range families {
		if containsGoAstSchemaString(members, name) {
			return true
		}
	}
	return false
}

func containsGoAstSchemaNilComment(comment string) bool {
	normalized := strings.ToLower(comment)
	return strings.Contains(normalized, "or nil") || strings.Contains(normalized, "may be nil") || strings.Contains(normalized, "nil means") || strings.Contains(normalized, "nil for") || strings.Contains(normalized, "non-go")
}

func containsGoAstSchemaString(values []string, target string) bool {
	for _, value := range values {
		if value == target {
			return true
		}
	}
	return false
}

func isExportedGoAstSchemaName(name string) bool {
	if name == "" {
		return false
	}
	first := name[0]
	return 'A' <= first && first <= 'Z'
}

func sha256GoAstSchemaInventory(data []byte) string {
	sum := sha256.Sum256(data)
	return hex.EncodeToString(sum[:])
}

func exitGoAstSchemaInventory(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}
