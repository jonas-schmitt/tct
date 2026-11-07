package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestGoRawTreeAstPrinterOracleStructureOnlyPasses(t *testing.T) {
	dir := t.TempDir()
	inputPath := filepath.Join(dir, "decoded.json")
	inventoryPath := filepath.Join(dir, "inventory.json")
	outputPath := filepath.Join(dir, "report.json")
	writeOracleTestJSON(t, inventoryPath, oracleInventoryFixture())
	writeOracleTestJSON(t, inputPath, oracleInputFixture(simpleOracleFileRoot(nil)))

	if err := runGoRawTreeAstPrinterOracle([]string{
		"--decoded-tree", inputPath,
		"--ast-inventory", inventoryPath,
		"--output", outputPath,
	}); err != nil {
		t.Fatal(err)
	}
	if err := runGoRawTreeAstPrinterOracle([]string{"--validate-report", outputPath}); err != nil {
		t.Fatal(err)
	}
	report := readOracleTestJSON[goRawTreeAstPrinterOracleReport](t, outputPath)
	if report.Status != goRawTreeAstPrinterOracleStatusPassed {
		t.Fatalf("status = %s, want passed", report.Status)
	}
	if report.StructuralPassedCount != 1 || report.StructuralBlockedCount != 0 {
		t.Fatalf("unexpected structural counts: %+v", report)
	}
	record := report.Records[0]
	if record.GoFormatValidation != "passed" || record.GoParserReparseValidation != "passed" {
		t.Fatalf("expected format/parser pass: %+v", record)
	}
	if record.Materialization.InferredBasicLitKindCount == 0 {
		t.Fatalf("expected BasicLit.Kind to be inferred from integer payload")
	}
	if !report.Decision.MateriallyReducesManualPolicy {
		t.Fatalf("passing structural oracle should reduce manual renderer policy")
	}
}

func TestGoRawTreeAstPrinterOracleRecordsEssentialCommentPolicyBlocker(t *testing.T) {
	dir := t.TempDir()
	inputPath := filepath.Join(dir, "decoded.json")
	inventoryPath := filepath.Join(dir, "inventory.json")
	outputPath := filepath.Join(dir, "report.json")
	writeOracleTestJSON(t, inventoryPath, oracleInventoryFixture())
	writeOracleTestJSON(t, inputPath, oracleInputFixture(simpleOracleFileRoot([]string{"//go:build linux"})))

	if err := runGoRawTreeAstPrinterOracle([]string{
		"--decoded-tree", inputPath,
		"--ast-inventory", inventoryPath,
		"--output", outputPath,
	}); err != nil {
		t.Fatal(err)
	}
	report := readOracleTestJSON[goRawTreeAstPrinterOracleReport](t, outputPath)
	if report.Status != goRawTreeAstPrinterOracleStatusCommentPolicyBlocked {
		t.Fatalf("status = %s, want comment-policy blocked", report.Status)
	}
	if report.StructuralPassedCount != 1 || report.CommentDirectiveBlockedCount != 1 {
		t.Fatalf("unexpected counts: %+v", report)
	}
	record := report.Records[0]
	if record.CommentDirectiveReadiness != "essential_comments_or_directives_unsupported_structure_only" {
		t.Fatalf("unexpected comment readiness: %s", record.CommentDirectiveReadiness)
	}
	if len(record.EssentialCommentCategories) != 1 || record.EssentialCommentCategories[0] != "go_build_constraint" {
		t.Fatalf("unexpected essential categories: %+v", record.EssentialCommentCategories)
	}
	if report.Decision.Recommendation != "add_essential_comment_policy_before_oracle_cutover" {
		t.Fatalf("unexpected decision: %+v", report.Decision)
	}
}

func TestGoRawTreeAstPrinterOracleGrammarModePreservesEssentialComments(t *testing.T) {
	dir := t.TempDir()
	inputPath := filepath.Join(dir, "decoded.json")
	inventoryPath := filepath.Join(dir, "inventory.json")
	outputPath := filepath.Join(dir, "report.json")
	writeOracleTestJSON(t, inventoryPath, oracleInventoryFixture())
	writeOracleTestJSON(t, inputPath, oracleInputFixture(simpleOracleFileRoot([]string{"//go:build linux"})))

	if err := runGoRawTreeAstPrinterOracle([]string{
		"--decoded-tree", inputPath,
		"--ast-inventory", inventoryPath,
		"--mode", goRawTreeAstPrinterOracleGrammarMode,
		"--output", outputPath,
	}); err != nil {
		t.Fatal(err)
	}
	if err := runGoRawTreeAstPrinterOracle([]string{"--validate-report", outputPath}); err != nil {
		t.Fatal(err)
	}
	report := readOracleTestJSON[goRawTreeAstPrinterOracleReport](t, outputPath)
	if report.Status != goRawTreeAstPrinterOracleStatusPassed {
		t.Fatalf("status = %s, want passed", report.Status)
	}
	if !report.FullAdoptionPossible {
		t.Fatalf("grammar-mode fixture should be full-adoption possible: %+v", report)
	}
	if report.CommentDirectiveBlockedCount != 0 || len(report.EssentialCommentBlockers) != 0 {
		t.Fatalf("grammar mode should preserve essential comments: %+v", report)
	}
	record := report.Records[0]
	if record.CommentDirectiveReadiness != "essential_comments_preserved_or_absent" {
		t.Fatalf("unexpected comment readiness: %s", record.CommentDirectiveReadiness)
	}
	if record.GoFormatValidation != "passed" || record.GoParserReparseValidation != "passed" {
		t.Fatalf("expected format/parser pass: %+v", record)
	}
}

func TestGoRawTreeAstPrinterOracleGrammarModePreservesDeclarationDirectives(t *testing.T) {
	cases := []struct {
		name      string
		directive string
		funcName  string
	}{
		{
			name:      "linkname",
			directive: "//go:linkname setUpdate",
			funcName:  "setUpdate",
		},
		{
			name:      "noescape",
			directive: "//go:noescape",
			funcName:  "gcmAuth",
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			dir := t.TempDir()
			inputPath := filepath.Join(dir, "decoded.json")
			inventoryPath := filepath.Join(dir, "inventory.json")
			outputPath := filepath.Join(dir, "report.json")
			root := externalFunctionDirectiveOracleFileRoot(tc.directive, tc.funcName)
			writeOracleTestJSON(t, inventoryPath, oracleInventoryFixture())
			writeOracleTestJSON(t, inputPath, oracleInputFixture(root))

			if err := runGoRawTreeAstPrinterOracle([]string{
				"--decoded-tree", inputPath,
				"--ast-inventory", inventoryPath,
				"--mode", goRawTreeAstPrinterOracleGrammarMode,
				"--output", outputPath,
			}); err != nil {
				t.Fatal(err)
			}
			if err := runGoRawTreeAstPrinterOracle([]string{"--validate-report", outputPath}); err != nil {
				t.Fatal(err)
			}
			report := readOracleTestJSON[goRawTreeAstPrinterOracleReport](t, outputPath)
			if report.Status != goRawTreeAstPrinterOracleStatusPassed {
				t.Fatalf("status = %s, want passed", report.Status)
			}
			if report.CommentDirectiveBlockedCount != 0 || len(report.EssentialCommentBlockers) != 0 {
				t.Fatalf("directive should be preserved: %+v", report)
			}
			record := report.Records[0]
			if len(record.EssentialCommentCategories) != 1 || record.EssentialCommentCategories[0] != "go_directive" {
				t.Fatalf("unexpected essential categories: %+v", record.EssentialCommentCategories)
			}

			formatted := formattedGrammarModeOracleSource(t, root)
			expectedPlacement := "\n" + tc.directive + "\nfunc " + tc.funcName + "()"
			if !strings.Contains(formatted, expectedPlacement) {
				t.Fatalf("formatted source missing directive before function\nwant substring: %q\nsource:\n%s", expectedPlacement, formatted)
			}
		})
	}
}

func TestGoRawTreeAstPrinterOracleBlocksUnsupportedNodeKind(t *testing.T) {
	dir := t.TempDir()
	inputPath := filepath.Join(dir, "decoded.json")
	inventoryPath := filepath.Join(dir, "inventory.json")
	outputPath := filepath.Join(dir, "report.json")
	writeOracleTestJSON(t, inventoryPath, oracleInventoryFixture())
	root := simpleOracleFileRoot(nil)
	root.Fields = append(root.Fields, rawDecodedField{
		Name: "Decls",
		Values: []rawDecodedValue{
			nodeValue(rawDecodedNode{NodeKind: "NotARealGoAstDecl", RuleName: "NotARealGoAstDecl"}),
		},
	})
	writeOracleTestJSON(t, inputPath, oracleInputFixture(root))

	if err := runGoRawTreeAstPrinterOracle([]string{
		"--decoded-tree", inputPath,
		"--ast-inventory", inventoryPath,
		"--output", outputPath,
	}); err != nil {
		t.Fatal(err)
	}
	report := readOracleTestJSON[goRawTreeAstPrinterOracleReport](t, outputPath)
	if report.Status != goRawTreeAstPrinterOracleStatusStructuralBlocked {
		t.Fatalf("status = %s, want structural blocked", report.Status)
	}
	if report.StructuralBlockedCount != 1 {
		t.Fatalf("expected one structural blocker: %+v", report)
	}
	if !hasOracleDiagnostic(report.Records[0].Diagnostics, "unsupported_node_kind") {
		t.Fatalf("missing unsupported_node_kind diagnostic: %+v", report.Records[0].Diagnostics)
	}
}

func formattedGrammarModeOracleSource(t *testing.T, root rawDecodedNode) string {
	t.Helper()
	materializer := newAstMaterializer(goRawTreeAstPrinterOracleGrammarMode)
	file, ok := materializer.materializeFile(&root)
	if !ok {
		t.Fatalf("materialization failed: %+v", materializer.diagnostics)
	}
	prepareGrammarModeAstFile(file)
	formatted, err := safeFormatGoAstFile(file)
	if err != nil {
		t.Fatal(err)
	}
	return formatted
}

func oracleInputFixture(root rawDecodedNode) goRawTreeAstPrinterOracleInput {
	return goRawTreeAstPrinterOracleInput{
		ArtifactKind: goRawTreeAstPrinterOracleInputKind,
		Version:      1,
		Tool:         "go_raw_tree_ast_printer_oracle_test",
		Mode:         goRawTreeAstPrinterOracleStructureMode,
		Records: []goRawTreeAstPrinterOracleInputRecord{{
			SourceLabel: "fixture.go",
			Category:    "unit_fixture",
			CurrentRustRendererGrammarMode: currentRustRendererGrammarMode{
				Status:                    "go_tokenizer_generator_raw_decoded_tree_render_ready",
				RawTreeGrammarModeReady:   true,
				RawTreeCanonicalReady:     true,
				GofmtValidation:           "passed",
				GoParserReparseValidation: "passed",
			},
			DecodedTree: rawDecodedTreeEnvelope{
				RootNodeKind: root.NodeKind,
				Root:         root,
			},
		}},
	}
}

func externalFunctionDirectiveOracleFileRoot(directive string, funcName string) rawDecodedNode {
	directiveGroup := commentGroupNode(directive)
	return rawDecodedNode{
		NodeKind: "File",
		RuleName: "File",
		Fields: []rawDecodedField{
			{
				Name: "Name",
				Values: []rawDecodedValue{
					nodeValue(rawDecodedNode{
						NodeKind: "Ident",
						RuleName: "Ident",
						Fields: []rawDecodedField{{
							Name:   "Name",
							Values: []rawDecodedValue{terminalValue("identifier", "identifier_payload", "main")},
						}},
					}),
				},
			},
			{
				Name: "Decls",
				Values: []rawDecodedValue{
					nodeValue(rawDecodedNode{
						NodeKind: "FuncDecl",
						RuleName: "FuncDecl",
						Fields: []rawDecodedField{
							{
								Name:   "Doc",
								Values: []rawDecodedValue{nodeValue(directiveGroup)},
							},
							{
								Name: "Name",
								Values: []rawDecodedValue{nodeValue(rawDecodedNode{
									NodeKind: "Ident",
									RuleName: "Ident",
									Fields: []rawDecodedField{{
										Name:   "Name",
										Values: []rawDecodedValue{terminalValue("identifier", "identifier_payload", funcName)},
									}},
								})},
							},
							{
								Name: "Type",
								Values: []rawDecodedValue{nodeValue(rawDecodedNode{
									NodeKind: "FuncType",
									RuleName: "FuncType",
									Fields: []rawDecodedField{{
										Name: "Params",
										Values: []rawDecodedValue{nodeValue(rawDecodedNode{
											NodeKind: "FieldList",
											RuleName: "FieldList",
										})},
									}},
								})},
							},
						},
					}),
				},
			},
			{
				Name:   "Comments",
				Values: []rawDecodedValue{nodeValue(directiveGroup)},
			},
		},
	}
}

func simpleOracleFileRoot(comments []string) rawDecodedNode {
	fields := []rawDecodedField{
		{
			Name: "Name",
			Values: []rawDecodedValue{
				nodeValue(rawDecodedNode{
					NodeKind: "Ident",
					RuleName: "Ident",
					Fields: []rawDecodedField{{
						Name:   "Name",
						Values: []rawDecodedValue{terminalValue("identifier", "identifier_payload", "main")},
					}},
				}),
			},
		},
		{
			Name: "Decls",
			Values: []rawDecodedValue{
				nodeValue(rawDecodedNode{
					NodeKind: "FuncDecl",
					RuleName: "FuncDecl",
					Fields: []rawDecodedField{
						{
							Name:   "Name",
							Values: []rawDecodedValue{terminalValue("identifier", "identifier_payload", "build")},
						},
						{
							Name: "Type",
							Values: []rawDecodedValue{nodeValue(rawDecodedNode{
								NodeKind: "FuncType",
								RuleName: "FuncType",
								Fields: []rawDecodedField{{
									Name: "Params",
									Values: []rawDecodedValue{nodeValue(rawDecodedNode{
										NodeKind: "FieldList",
										RuleName: "FieldList",
									})},
								}},
							})},
						},
						{
							Name: "Body",
							Values: []rawDecodedValue{nodeValue(rawDecodedNode{
								NodeKind: "BlockStmt",
								RuleName: "BlockStmt",
								Fields: []rawDecodedField{{
									Name: "List",
									Values: []rawDecodedValue{nodeValue(rawDecodedNode{
										NodeKind: "ReturnStmt",
										RuleName: "ReturnStmt",
										Fields: []rawDecodedField{{
											Name: "Results",
											Values: []rawDecodedValue{nodeValue(rawDecodedNode{
												NodeKind: "BasicLit",
												RuleName: "BasicLit",
												Fields: []rawDecodedField{{
													Name:   "Value",
													Values: []rawDecodedValue{terminalValue("integer_literal", "integer_payload", "7")},
												}},
											})},
										}},
									})},
								}},
							})},
						},
					},
				}),
			},
		},
	}
	if len(comments) > 0 {
		var commentValues []rawDecodedValue
		for _, comment := range comments {
			commentValues = append(commentValues, nodeValue(commentGroupNode(comment)))
		}
		fields = append(fields, rawDecodedField{Name: "Comments", Values: commentValues})
	}
	return rawDecodedNode{
		NodeKind: "File",
		RuleName: "File",
		Fields:   fields,
	}
}

func commentGroupNode(comments ...string) rawDecodedNode {
	values := make([]rawDecodedValue, 0, len(comments))
	for _, comment := range comments {
		values = append(values, nodeValue(rawDecodedNode{
			NodeKind: "Comment",
			RuleName: "Comment",
			Fields: []rawDecodedField{{
				Name:   "Text",
				Values: []rawDecodedValue{terminalValue("comment_text", "string_payload", comment)},
			}},
		}))
	}
	return rawDecodedNode{
		NodeKind: "CommentGroup",
		RuleName: "CommentGroup",
		Fields: []rawDecodedField{{
			Name:   "List",
			Values: values,
		}},
	}
}

func nodeValue(node rawDecodedNode) rawDecodedValue {
	return rawDecodedValue{Kind: "node", NodeKind: node.NodeKind, Node: &node}
}

func terminalValue(nodeKind string, terminal string, payload string) rawDecodedValue {
	return rawDecodedValue{
		Kind:     "terminal",
		NodeKind: nodeKind,
		Terminal: terminal,
		Payload:  payload,
	}
}

func oracleInventoryFixture() map[string]any {
	return map[string]any{
		"artifact_kind": "tct_go_ast_schema_inventory_v1",
		"version":       1,
		"go": map[string]any{
			"version": "go1.24.3",
		},
	}
}

func hasOracleDiagnostic(diagnostics []oracleDiagnostic, kind string) bool {
	for _, diagnostic := range diagnostics {
		if diagnostic.DiagnosticKind == kind {
			return true
		}
	}
	return false
}

func writeOracleTestJSON(t *testing.T, path string, value any) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	data, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	data = append(data, '\n')
	if err := os.WriteFile(path, data, 0o644); err != nil {
		t.Fatal(err)
	}
}

func readOracleTestJSON[T any](t *testing.T, path string) T {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var value T
	if err := json.Unmarshal(data, &value); err != nil {
		t.Fatal(err)
	}
	return value
}
