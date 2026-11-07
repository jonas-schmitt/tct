//! C# source rendering from the reusable raw syntax decoded tree.
//!
//! This slice is intentionally narrow: it renders checked Roslyn metadata
//! shapes from the target-neutral raw decoded tree. Parser validation stays
//! sidecar-only.

mod decoded_tree_access;

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_state_machine::{RawSyntaxDecodedNode, RawSyntaxDecodedTree};
use decoded_tree_access::{
    csharp_string_literal, node_values_field, optional_node_field_any, render_error, require_node,
    require_pair_slice, require_string, require_string_slice, require_string_vec,
    single_node_field, single_node_field_any, single_terminal_payload, single_terminal_payload_any,
};

pub const CSHARP_RAW_TREE_RENDERER_ID: &str = "csharp-raw-decoded-tree-renderer-v1";
pub const CSHARP_MINIMAL_CLASS_RAW_TREE_RENDERER_SCOPE: &str =
    "csharp_minimal_class_decoded_tree_source_render_v1";
pub const CSHARP_MINIMAL_CLASS_RAW_TREE_RENDERED_SOURCE: &str = "namespace Tct.Contracts;\n\npublic sealed class Greeter\n{\n    public string Name { get; init; } = \"world\";\n}\n";
pub const CSHARP_LOCAL_INTEGER_RETURN_LITERAL_RAW_TREE_RENDERER_SCOPE: &str =
    "csharp_local_integer_return_literal_decoded_tree_source_render_v1";
pub const CSHARP_LOCAL_INTEGER_RETURN_LITERAL_RAW_TREE_RENDERED_SOURCE: &str = "namespace Tct.Contracts;\n\npublic sealed class Calculator\n{\n    public int Build()\n    {\n        return 7;\n    }\n}\n";
pub const CSHARP_LOCAL_UNARY_INTEGER_RETURN_RAW_TREE_RENDERER_SCOPE: &str =
    "csharp_local_unary_integer_return_decoded_tree_source_render_v1";
pub const CSHARP_LOCAL_UNARY_INTEGER_RETURN_RAW_TREE_RENDERED_SOURCE: &str = "namespace Tct.Contracts;\n\npublic sealed class Calculator\n{\n    public int Build()\n    {\n        return -1;\n    }\n}\n";
pub const CSHARP_MEMBER_ACCESS_EXPRESSION_RAW_TREE_RENDERER_SCOPE: &str =
    "csharp_member_access_expression_decoded_tree_source_render_v1";
pub const CSHARP_MEMBER_ACCESS_EXPRESSION_RAW_TREE_RENDERED_SOURCE: &str = "namespace Tct.Contracts;\n\npublic sealed class ClockContract\n{\n    public string Read()\n    {\n        return DateTime.Now;\n    }\n}\n";
pub const CSHARP_COMPILATION_UNIT_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE: &str =
    "csharp_compilation_unit_source_file_decoded_tree_source_render_v1";
pub const CSHARP_COMPILATION_UNIT_SOURCE_FILE_RAW_TREE_RENDERED_SOURCE: &str = "\u{feff}using System;\nusing System.Collections.Generic;\nusing System.Linq;\nusing System.Text;\nusing System.Threading.Tasks;\n\nnamespace PatternMatching\n{\n    #region 01_ShapeDefinitions\n    public class Square\n    {\n        public double Side { get; }\n\n        public Square(double side)\n        {\n            Side = side;\n        }\n    }\n    public class Circle\n    {\n        public double Radius { get; }\n\n        public Circle(double radius)\n        {\n            Radius = radius;\n        }\n    }\n    public struct Rectangle\n    {\n        public double Length { get; }\n        public double Height { get; }\n\n        public Rectangle(double length, double height)\n        {\n            Length = length;\n            Height = height;\n        }\n    }\n    public class Triangle\n    {\n        public double Base { get; }\n        public double Height { get; }\n\n        public Triangle(double @base, double height)\n        {\n            Base = @base;\n            Height = height;\n        }\n    }\n    #endregion\n\n}\n";
pub const CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE: &str =
    "csharp_compilation_unit_logger_source_file_decoded_tree_source_render_v1";
pub const CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_RAW_TREE_RENDERED_SOURCE: &str = "\u{feff}namespace DelegatesAndEvents;\n\npublic enum Severity\n{\n    Verbose,\n    Trace,\n    Information,\n    Warning,\n    Error,\n    Critical\n}\n\npublic static class Logger\n{\n    public static Action<string>? WriteMessage;\n\n    public static Severity LogLevel { get; set; } = Severity.Warning;\n\n    public static void LogMessage(Severity s, string component, string msg)\n    {\n        if ((s < LogLevel) || (WriteMessage is null))\n            return;\n\n        var outputMsg = $\"{DateTime.Now}\\t{s}\\t{component}\\t{msg}\";\n        WriteMessage(outputMsg);\n    }\n}\n";
pub const CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE: &str =
    "csharp_compilation_unit_version_one_source_file_decoded_tree_source_render_v1";
pub const CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_RAW_TREE_RENDERED_SOURCE: &str = "\u{feff}namespace Events;\n\npublic class FileFoundArgs : EventArgs\n{\n    public string FoundFile { get; }\n\n    public FileFoundArgs(string fileName) => FoundFile = fileName;\n}\n\npublic class FileSearcher\n{\n    public event EventHandler<FileFoundArgs>? FileFound;\n\n    public void Search(string directory, string searchPattern)\n    {\n        foreach (var file in Directory.EnumerateFiles(directory, searchPattern))\n        {\n            FileFound?.Invoke(this, new FileFoundArgs(file));\n        }\n    }\n}\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpRawTreeRenderedSource {
    pub renderer_id: String,
    pub scope: String,
    pub supported_construct: String,
    pub root_node_kind: String,
    pub selected_node_kinds: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpMinimalClassSyntax {
    namespace: String,
    class_name: String,
    property_type_keyword: String,
    property_name: String,
    initializer: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpLocalIntegerReturnLiteralSyntax {
    namespace: String,
    class_name: String,
    method_name: String,
    return_type_keyword: String,
    return_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpLocalUnaryIntegerReturnSyntax {
    namespace: String,
    class_name: String,
    method_name: String,
    return_type_keyword: String,
    unary_operator: String,
    return_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpMemberAccessExpressionSyntax {
    namespace: String,
    class_name: String,
    method_name: String,
    return_type: String,
    return_expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpCompilationUnitSourceFileSyntax {
    usings: Vec<String>,
    namespace: String,
    declarations: Vec<CSharpTypeDeclarationSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpTypeDeclarationSyntax {
    kind: CSharpTypeDeclarationKind,
    name: String,
    properties: Vec<CSharpPropertySyntax>,
    constructor: CSharpConstructorSyntax,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CSharpTypeDeclarationKind {
    Class,
    Struct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpPropertySyntax {
    type_keyword: String,
    name: String,
    accessor_keyword: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpConstructorSyntax {
    name: String,
    parameters: Vec<CSharpParameterSyntax>,
    assignments: Vec<CSharpAssignmentSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpParameterSyntax {
    type_keyword: String,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpAssignmentSyntax {
    left: String,
    right: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpCompilationUnitLoggerSourceFileSyntax {
    namespace: String,
    enum_declaration: CSharpEnumDeclarationSyntax,
    class_declaration: CSharpLoggerClassSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpEnumDeclarationSyntax {
    name: String,
    members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpLoggerClassSyntax {
    name: String,
    field: CSharpFieldDeclarationSyntax,
    property: CSharpLoggerPropertySyntax,
    method: CSharpMethodDeclarationSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpFieldDeclarationSyntax {
    declaration: CSharpVariableDeclarationSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpVariableDeclarationSyntax {
    type_name: String,
    variables: Vec<CSharpVariableDeclaratorSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpVariableDeclaratorSyntax {
    name: String,
    initializer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpLoggerPropertySyntax {
    type_name: String,
    name: String,
    accessors: Vec<String>,
    initializer: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpMethodDeclarationSyntax {
    return_type: String,
    name: String,
    parameters: Vec<CSharpParameterSyntax>,
    statements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpCompilationUnitVersionOneSourceFileSyntax {
    namespace: String,
    file_found_args_class: CSharpVersionOneFileFoundArgsClassSyntax,
    file_searcher_class: CSharpVersionOneFileSearcherClassSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpVersionOneFileFoundArgsClassSyntax {
    name: String,
    base_types: Vec<String>,
    property: CSharpPropertySyntax,
    constructor: CSharpArrowConstructorSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpArrowConstructorSyntax {
    name: String,
    parameters: Vec<CSharpParameterSyntax>,
    assignment: CSharpAssignmentSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpVersionOneFileSearcherClassSyntax {
    name: String,
    event: CSharpEventFieldDeclarationSyntax,
    method: CSharpMethodDeclarationSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CSharpEventFieldDeclarationSyntax {
    declaration: CSharpVariableDeclarationSyntax,
}

pub fn render_csharp_minimal_class_source_from_raw_tree(
    tree: &RawSyntaxDecodedTree,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    let root = tree.decoded_node_tree()?;
    render_csharp_minimal_class_source_from_decoded_node(&root)
}

pub fn render_csharp_minimal_class_source_from_decoded_node(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    require_node("raw_tree.root", root, "CompilationUnitSyntax")?;
    let syntax = decode_minimal_class(root)?;
    let source = render_minimal_class_source(&syntax);
    Ok(CSharpRawTreeRenderedSource {
        renderer_id: CSHARP_RAW_TREE_RENDERER_ID.to_string(),
        scope: CSHARP_MINIMAL_CLASS_RAW_TREE_RENDERER_SCOPE.to_string(),
        supported_construct: "minimal_file_scoped_namespace_class_string_property".to_string(),
        root_node_kind: root.node_kind.clone(),
        selected_node_kinds: vec![
            "CompilationUnitSyntax".to_string(),
            "FileScopedNamespaceDeclarationSyntax".to_string(),
            "ClassDeclarationSyntax".to_string(),
            "PropertyDeclarationSyntax".to_string(),
            "PredefinedTypeSyntax".to_string(),
        ],
        source,
    })
}

pub fn render_csharp_local_integer_return_literal_from_raw_tree(
    tree: &RawSyntaxDecodedTree,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    let root = tree.decoded_node_tree()?;
    render_csharp_local_integer_return_literal_from_decoded_node(&root)
}

pub fn render_csharp_local_integer_return_literal_from_decoded_node(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    require_node("raw_tree.root", root, "CompilationUnitSyntax")?;
    let syntax = decode_local_integer_return_literal(root)?;
    let source = render_local_integer_return_literal_source(&syntax)?;
    Ok(CSharpRawTreeRenderedSource {
        renderer_id: CSHARP_RAW_TREE_RENDERER_ID.to_string(),
        scope: CSHARP_LOCAL_INTEGER_RETURN_LITERAL_RAW_TREE_RENDERER_SCOPE.to_string(),
        supported_construct: "local_integer_return_literal".to_string(),
        root_node_kind: root.node_kind.clone(),
        selected_node_kinds: vec![
            "CompilationUnitSyntax".to_string(),
            "FileScopedNamespaceDeclarationSyntax".to_string(),
            "ClassDeclarationSyntax".to_string(),
            "MethodDeclarationSyntax".to_string(),
            "PredefinedTypeSyntax".to_string(),
            "BlockSyntax".to_string(),
            "ReturnStatementSyntax".to_string(),
            "LiteralExpressionSyntax".to_string(),
        ],
        source,
    })
}

pub fn render_csharp_local_unary_integer_return_from_raw_tree(
    tree: &RawSyntaxDecodedTree,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    let root = tree.decoded_node_tree()?;
    render_csharp_local_unary_integer_return_from_decoded_node(&root)
}

pub fn render_csharp_local_unary_integer_return_from_decoded_node(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    require_node("raw_tree.root", root, "CompilationUnitSyntax")?;
    let syntax = decode_local_unary_integer_return(root)?;
    let source = render_local_unary_integer_return_source(&syntax)?;
    Ok(CSharpRawTreeRenderedSource {
        renderer_id: CSHARP_RAW_TREE_RENDERER_ID.to_string(),
        scope: CSHARP_LOCAL_UNARY_INTEGER_RETURN_RAW_TREE_RENDERER_SCOPE.to_string(),
        supported_construct: "local_unary_integer_return".to_string(),
        root_node_kind: root.node_kind.clone(),
        selected_node_kinds: vec![
            "CompilationUnitSyntax".to_string(),
            "FileScopedNamespaceDeclarationSyntax".to_string(),
            "ClassDeclarationSyntax".to_string(),
            "MethodDeclarationSyntax".to_string(),
            "PredefinedTypeSyntax".to_string(),
            "BlockSyntax".to_string(),
            "ReturnStatementSyntax".to_string(),
            "PrefixUnaryExpressionSyntax".to_string(),
            "LiteralExpressionSyntax".to_string(),
        ],
        source,
    })
}

pub fn render_csharp_member_access_expression_from_raw_tree(
    tree: &RawSyntaxDecodedTree,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    let root = tree.decoded_node_tree()?;
    render_csharp_member_access_expression_from_decoded_node(&root)
}

pub fn render_csharp_member_access_expression_from_decoded_node(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    require_node("raw_tree.root", root, "CompilationUnitSyntax")?;
    let syntax = decode_member_access_expression_fixture(root)?;
    let source = render_member_access_expression_source(&syntax)?;
    Ok(CSharpRawTreeRenderedSource {
        renderer_id: CSHARP_RAW_TREE_RENDERER_ID.to_string(),
        scope: CSHARP_MEMBER_ACCESS_EXPRESSION_RAW_TREE_RENDERER_SCOPE.to_string(),
        supported_construct: "member_access_expression_return".to_string(),
        root_node_kind: root.node_kind.clone(),
        selected_node_kinds: vec![
            "CompilationUnitSyntax".to_string(),
            "FileScopedNamespaceDeclarationSyntax".to_string(),
            "ClassDeclarationSyntax".to_string(),
            "MethodDeclarationSyntax".to_string(),
            "PredefinedTypeSyntax".to_string(),
            "BlockSyntax".to_string(),
            "ReturnStatementSyntax".to_string(),
            "MemberAccessExpressionSyntax".to_string(),
            "IdentifierNameSyntax".to_string(),
        ],
        source,
    })
}

pub fn render_csharp_compilation_unit_source_file_from_raw_tree(
    tree: &RawSyntaxDecodedTree,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    let root = tree.decoded_node_tree()?;
    render_csharp_compilation_unit_source_file_from_decoded_node(&root)
}

pub fn render_csharp_compilation_unit_source_file_from_decoded_node(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    require_node("raw_tree.root", root, "CompilationUnitSyntax")?;
    let syntax = decode_compilation_unit_source_file(root)?;
    let source = render_compilation_unit_source_file_source(&syntax)?;
    Ok(CSharpRawTreeRenderedSource {
        renderer_id: CSHARP_RAW_TREE_RENDERER_ID.to_string(),
        scope: CSHARP_COMPILATION_UNIT_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE.to_string(),
        supported_construct: "compilation_unit_shapes_source_file".to_string(),
        root_node_kind: root.node_kind.clone(),
        selected_node_kinds: vec![
            "CompilationUnitSyntax".to_string(),
            "UsingDirectiveSyntax".to_string(),
            "QualifiedNameSyntax".to_string(),
            "IdentifierNameSyntax".to_string(),
            "NamespaceDeclarationSyntax".to_string(),
            "ClassDeclarationSyntax".to_string(),
            "StructDeclarationSyntax".to_string(),
            "PropertyDeclarationSyntax".to_string(),
            "AccessorListSyntax".to_string(),
            "AccessorDeclarationSyntax".to_string(),
            "ConstructorDeclarationSyntax".to_string(),
            "ParameterListSyntax".to_string(),
            "ParameterSyntax".to_string(),
            "BlockSyntax".to_string(),
            "ExpressionStatementSyntax".to_string(),
            "AssignmentExpressionSyntax".to_string(),
            "PredefinedTypeSyntax".to_string(),
        ],
        source,
    })
}

pub fn render_csharp_compilation_unit_logger_source_file_from_raw_tree(
    tree: &RawSyntaxDecodedTree,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    let root = tree.decoded_node_tree()?;
    render_csharp_compilation_unit_logger_source_file_from_decoded_node(&root)
}

pub fn render_csharp_compilation_unit_logger_source_file_from_decoded_node(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    require_node("raw_tree.root", root, "CompilationUnitSyntax")?;
    let syntax = decode_compilation_unit_logger_source_file(root)?;
    let source = render_compilation_unit_logger_source_file_source(&syntax)?;
    Ok(CSharpRawTreeRenderedSource {
        renderer_id: CSHARP_RAW_TREE_RENDERER_ID.to_string(),
        scope: CSHARP_COMPILATION_UNIT_LOGGER_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE.to_string(),
        supported_construct: "compilation_unit_logger_source_file".to_string(),
        root_node_kind: root.node_kind.clone(),
        selected_node_kinds: vec![
            "CompilationUnitSyntax".to_string(),
            "FileScopedNamespaceDeclarationSyntax".to_string(),
            "EnumDeclarationSyntax".to_string(),
            "EnumMemberDeclarationSyntax".to_string(),
            "ClassDeclarationSyntax".to_string(),
            "FieldDeclarationSyntax".to_string(),
            "PropertyDeclarationSyntax".to_string(),
            "MethodDeclarationSyntax".to_string(),
            "VariableDeclarationSyntax".to_string(),
            "VariableDeclaratorSyntax".to_string(),
            "NullableTypeSyntax".to_string(),
            "GenericNameSyntax".to_string(),
            "TypeArgumentListSyntax".to_string(),
            "AccessorListSyntax".to_string(),
            "AccessorDeclarationSyntax".to_string(),
            "ParameterListSyntax".to_string(),
            "ParameterSyntax".to_string(),
            "BlockSyntax".to_string(),
            "IfStatementSyntax".to_string(),
            "LocalDeclarationStatementSyntax".to_string(),
            "ExpressionStatementSyntax".to_string(),
            "InvocationExpressionSyntax".to_string(),
            "ArgumentListSyntax".to_string(),
            "ArgumentSyntax".to_string(),
            "MemberAccessExpressionSyntax".to_string(),
            "InterpolatedStringExpressionSyntax".to_string(),
        ],
        source,
    })
}

pub fn render_csharp_compilation_unit_version_one_source_file_from_raw_tree(
    tree: &RawSyntaxDecodedTree,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    let root = tree.decoded_node_tree()?;
    render_csharp_compilation_unit_version_one_source_file_from_decoded_node(&root)
}

pub fn render_csharp_compilation_unit_version_one_source_file_from_decoded_node(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpRawTreeRenderedSource, SyntaxBackendBoundaryError> {
    require_node("raw_tree.root", root, "CompilationUnitSyntax")?;
    let syntax = decode_compilation_unit_version_one_source_file(root)?;
    let source = render_compilation_unit_version_one_source_file_source(&syntax)?;
    Ok(CSharpRawTreeRenderedSource {
        renderer_id: CSHARP_RAW_TREE_RENDERER_ID.to_string(),
        scope: CSHARP_COMPILATION_UNIT_VERSION_ONE_SOURCE_FILE_RAW_TREE_RENDERER_SCOPE.to_string(),
        supported_construct: "compilation_unit_version_one_source_file".to_string(),
        root_node_kind: root.node_kind.clone(),
        selected_node_kinds: vec![
            "CompilationUnitSyntax".to_string(),
            "FileScopedNamespaceDeclarationSyntax".to_string(),
            "ClassDeclarationSyntax".to_string(),
            "BaseListSyntax".to_string(),
            "SimpleBaseTypeSyntax".to_string(),
            "PropertyDeclarationSyntax".to_string(),
            "ConstructorDeclarationSyntax".to_string(),
            "ArrowExpressionClauseSyntax".to_string(),
            "EventFieldDeclarationSyntax".to_string(),
            "NullableTypeSyntax".to_string(),
            "GenericNameSyntax".to_string(),
            "MethodDeclarationSyntax".to_string(),
            "ForEachStatementSyntax".to_string(),
            "ConditionalAccessExpressionSyntax".to_string(),
            "InvocationExpressionSyntax".to_string(),
            "MemberBindingExpressionSyntax".to_string(),
            "ObjectCreationExpressionSyntax".to_string(),
            "ThisExpressionSyntax".to_string(),
        ],
        source,
    })
}

fn decode_minimal_class(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpMinimalClassSyntax, SyntaxBackendBoundaryError> {
    let namespace_node = single_node_field(
        "CompilationUnitSyntax.Members",
        root,
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    )?;
    let namespace = single_terminal_payload(
        "FileScopedNamespaceDeclarationSyntax.Name",
        namespace_node,
        "Name",
        "qualified_identifier",
        "identifier_payload",
    )?;
    let class_node = single_node_field(
        "FileScopedNamespaceDeclarationSyntax.Members",
        namespace_node,
        "Members",
        "ClassDeclarationSyntax",
    )?;
    let class_name = single_terminal_payload(
        "ClassDeclarationSyntax.Identifier",
        class_node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let property_node = single_node_field(
        "ClassDeclarationSyntax.Members",
        class_node,
        "Members",
        "PropertyDeclarationSyntax",
    )?;
    let property_name = single_terminal_payload(
        "PropertyDeclarationSyntax.Identifier",
        property_node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let type_node = single_node_field(
        "PropertyDeclarationSyntax.Type",
        property_node,
        "Type",
        "PredefinedTypeSyntax",
    )?;
    let property_type_keyword = single_terminal_payload(
        "PredefinedTypeSyntax.Keyword",
        type_node,
        "Keyword",
        "string_keyword",
        "identifier_payload",
    )?;
    if property_type_keyword != "string" {
        return Err(render_error(
            "PredefinedTypeSyntax.Keyword",
            format!(
                "minimal-class renderer only supports string properties, got {property_type_keyword}"
            ),
        ));
    }
    let initializer = single_terminal_payload(
        "PropertyDeclarationSyntax.Initializer",
        property_node,
        "Initializer",
        "StringLiteralExpressionSyntax",
        "string_payload",
    )?;

    Ok(CSharpMinimalClassSyntax {
        namespace,
        class_name,
        property_type_keyword,
        property_name,
        initializer,
    })
}

fn render_minimal_class_source(syntax: &CSharpMinimalClassSyntax) -> String {
    format!(
        "namespace {};\n\npublic sealed class {}\n{{\n    public {} {} {{ get; init; }} = {};\n}}\n",
        syntax.namespace,
        syntax.class_name,
        syntax.property_type_keyword,
        syntax.property_name,
        csharp_string_literal(&syntax.initializer),
    )
}

fn decode_local_integer_return_literal(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpLocalIntegerReturnLiteralSyntax, SyntaxBackendBoundaryError> {
    let namespace_node = single_node_field(
        "CompilationUnitSyntax.Members",
        root,
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    )?;
    let namespace = single_terminal_payload(
        "FileScopedNamespaceDeclarationSyntax.Name",
        namespace_node,
        "Name",
        "qualified_identifier",
        "identifier_payload",
    )?;
    let class_node = single_node_field(
        "FileScopedNamespaceDeclarationSyntax.Members",
        namespace_node,
        "Members",
        "ClassDeclarationSyntax",
    )?;
    let class_name = single_terminal_payload(
        "ClassDeclarationSyntax.Identifier",
        class_node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let method_node = single_node_field(
        "ClassDeclarationSyntax.Members",
        class_node,
        "Members",
        "MethodDeclarationSyntax",
    )?;
    let method_name = single_terminal_payload(
        "MethodDeclarationSyntax.Identifier",
        method_node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let return_type_node = single_node_field(
        "MethodDeclarationSyntax.ReturnType",
        method_node,
        "ReturnType",
        "PredefinedTypeSyntax",
    )?;
    let return_type_keyword = single_terminal_payload(
        "PredefinedTypeSyntax.Keyword",
        return_type_node,
        "Keyword",
        "int_keyword",
        "identifier_payload",
    )?;
    let block_node = single_node_field(
        "MethodDeclarationSyntax.Body",
        method_node,
        "Body",
        "BlockSyntax",
    )?;
    let return_statement = single_node_field(
        "BlockSyntax.Statements",
        block_node,
        "Statements",
        "ReturnStatementSyntax",
    )?;
    let literal_expression = single_node_field(
        "ReturnStatementSyntax.Expression",
        return_statement,
        "Expression",
        "LiteralExpressionSyntax",
    )?;
    let return_value = single_integer_terminal_payload(
        "LiteralExpressionSyntax.Token",
        literal_expression,
        "Token",
        "integer_literal",
    )?;

    Ok(CSharpLocalIntegerReturnLiteralSyntax {
        namespace,
        class_name,
        method_name,
        return_type_keyword,
        return_value,
    })
}

fn single_integer_terminal_payload(
    path: &'static str,
    node: &RawSyntaxDecodedNode,
    field_name: &str,
    expected_node_kind: &str,
) -> Result<String, SyntaxBackendBoundaryError> {
    let payload = single_terminal_payload(
        path,
        node,
        field_name,
        expected_node_kind,
        "integer_payload",
    )?;
    // Bounded u8 integer payloads materialize as the decimal string of the
    // value token (0..=255). Accept that form directly; reject out-of-range
    // or non-decimal residue so render stays aligned with the raw SM contract.
    let value = payload.parse::<u16>().map_err(|_| {
        render_error(
            path,
            format!("u8 integer payload {payload:?} is not a decimal integer"),
        )
    })?;
    if value > 255 {
        return Err(render_error(
            path,
            format!("u8 integer payload {payload:?} is outside 0..=255"),
        ));
    }
    Ok(value.to_string())
}

fn render_local_integer_return_literal_source(
    syntax: &CSharpLocalIntegerReturnLiteralSyntax,
) -> Result<String, SyntaxBackendBoundaryError> {
    validate_local_integer_return_literal_syntax(syntax)?;
    Ok(format!(
        "namespace {};\n\npublic sealed class {}\n{{\n    public {} {}()\n    {{\n        return {};\n    }}\n}}\n",
        syntax.namespace,
        syntax.class_name,
        syntax.return_type_keyword,
        syntax.method_name,
        syntax.return_value,
    ))
}

fn validate_local_integer_return_literal_syntax(
    syntax: &CSharpLocalIntegerReturnLiteralSyntax,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "FileScopedNamespaceDeclarationSyntax.Name",
        &syntax.namespace,
        "Tct.Contracts",
    )?;
    require_string(
        "ClassDeclarationSyntax.Identifier",
        &syntax.class_name,
        "Calculator",
    )?;
    require_string(
        "MethodDeclarationSyntax.Identifier",
        &syntax.method_name,
        "Build",
    )?;
    require_string(
        "MethodDeclarationSyntax.ReturnType",
        &syntax.return_type_keyword,
        "int",
    )?;
    require_string("LiteralExpressionSyntax.Token", &syntax.return_value, "7")
}

fn decode_local_unary_integer_return(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpLocalUnaryIntegerReturnSyntax, SyntaxBackendBoundaryError> {
    let namespace_node = single_node_field(
        "CompilationUnitSyntax.Members",
        root,
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    )?;
    let namespace = single_terminal_payload(
        "FileScopedNamespaceDeclarationSyntax.Name",
        namespace_node,
        "Name",
        "qualified_identifier",
        "identifier_payload",
    )?;
    let class_node = single_node_field(
        "FileScopedNamespaceDeclarationSyntax.Members",
        namespace_node,
        "Members",
        "ClassDeclarationSyntax",
    )?;
    let class_name = single_terminal_payload(
        "ClassDeclarationSyntax.Identifier",
        class_node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let method_node = single_node_field(
        "ClassDeclarationSyntax.Members",
        class_node,
        "Members",
        "MethodDeclarationSyntax",
    )?;
    let method_name = single_terminal_payload(
        "MethodDeclarationSyntax.Identifier",
        method_node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let return_type_node = single_node_field(
        "MethodDeclarationSyntax.ReturnType",
        method_node,
        "ReturnType",
        "PredefinedTypeSyntax",
    )?;
    let return_type_keyword = single_terminal_payload(
        "PredefinedTypeSyntax.Keyword",
        return_type_node,
        "Keyword",
        "int_keyword",
        "identifier_payload",
    )?;
    let block_node = single_node_field(
        "MethodDeclarationSyntax.Body",
        method_node,
        "Body",
        "BlockSyntax",
    )?;
    let return_statement = single_node_field(
        "BlockSyntax.Statements",
        block_node,
        "Statements",
        "ReturnStatementSyntax",
    )?;
    let unary_expression = single_node_field(
        "ReturnStatementSyntax.Expression",
        return_statement,
        "Expression",
        "PrefixUnaryExpressionSyntax",
    )?;
    let unary_operator = single_terminal_payload(
        "PrefixUnaryExpressionSyntax.OperatorToken",
        unary_expression,
        "OperatorToken",
        "unary_operator",
        "string_payload",
    )?;
    let literal_expression = single_node_field(
        "PrefixUnaryExpressionSyntax.Operand",
        unary_expression,
        "Operand",
        "LiteralExpressionSyntax",
    )?;
    let return_value = single_integer_terminal_payload(
        "LiteralExpressionSyntax.Token",
        literal_expression,
        "Token",
        "integer_literal",
    )?;

    Ok(CSharpLocalUnaryIntegerReturnSyntax {
        namespace,
        class_name,
        method_name,
        return_type_keyword,
        unary_operator,
        return_value,
    })
}

fn render_local_unary_integer_return_source(
    syntax: &CSharpLocalUnaryIntegerReturnSyntax,
) -> Result<String, SyntaxBackendBoundaryError> {
    validate_local_unary_integer_return_syntax(syntax)?;
    Ok(format!(
        "namespace {};\n\npublic sealed class {}\n{{\n    public {} {}()\n    {{\n        return {}{};\n    }}\n}}\n",
        syntax.namespace,
        syntax.class_name,
        syntax.return_type_keyword,
        syntax.method_name,
        syntax.unary_operator,
        syntax.return_value,
    ))
}

fn validate_local_unary_integer_return_syntax(
    syntax: &CSharpLocalUnaryIntegerReturnSyntax,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "FileScopedNamespaceDeclarationSyntax.Name",
        &syntax.namespace,
        "Tct.Contracts",
    )?;
    require_string(
        "ClassDeclarationSyntax.Identifier",
        &syntax.class_name,
        "Calculator",
    )?;
    require_string(
        "MethodDeclarationSyntax.Identifier",
        &syntax.method_name,
        "Build",
    )?;
    require_string(
        "MethodDeclarationSyntax.ReturnType",
        &syntax.return_type_keyword,
        "int",
    )?;
    require_string(
        "PrefixUnaryExpressionSyntax.OperatorToken",
        &syntax.unary_operator,
        "-",
    )?;
    require_string("LiteralExpressionSyntax.Token", &syntax.return_value, "1")
}

fn decode_member_access_expression_fixture(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpMemberAccessExpressionSyntax, SyntaxBackendBoundaryError> {
    let namespace_node = single_node_field(
        "CompilationUnitSyntax.Members",
        root,
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    )?;
    let namespace = single_terminal_payload(
        "FileScopedNamespaceDeclarationSyntax.Name",
        namespace_node,
        "Name",
        "qualified_identifier",
        "identifier_payload",
    )?;
    let class_node = single_node_field(
        "FileScopedNamespaceDeclarationSyntax.Members",
        namespace_node,
        "Members",
        "ClassDeclarationSyntax",
    )?;
    let class_name = single_terminal_payload(
        "ClassDeclarationSyntax.Identifier",
        class_node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let method_node = single_node_field(
        "ClassDeclarationSyntax.Members",
        class_node,
        "Members",
        "MethodDeclarationSyntax",
    )?;
    let method_name = single_terminal_payload(
        "MethodDeclarationSyntax.Identifier",
        method_node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let return_type = decode_type_syntax(single_node_field(
        "MethodDeclarationSyntax.ReturnType",
        method_node,
        "ReturnType",
        "PredefinedTypeSyntax",
    )?)?;
    let block_node = single_node_field(
        "MethodDeclarationSyntax.Body",
        method_node,
        "Body",
        "BlockSyntax",
    )?;
    let return_statement = single_node_field(
        "BlockSyntax.Statements",
        block_node,
        "Statements",
        "ReturnStatementSyntax",
    )?;
    let expression = single_node_field(
        "ReturnStatementSyntax.Expression",
        return_statement,
        "Expression",
        "MemberAccessExpressionSyntax",
    )?;
    Ok(CSharpMemberAccessExpressionSyntax {
        namespace,
        class_name,
        method_name,
        return_type,
        return_expression: decode_expression(expression)?,
    })
}

fn render_member_access_expression_source(
    syntax: &CSharpMemberAccessExpressionSyntax,
) -> Result<String, SyntaxBackendBoundaryError> {
    validate_member_access_expression_syntax(syntax)?;
    Ok(format!(
        "namespace {};\n\npublic sealed class {}\n{{\n    public {} {}()\n    {{\n        return {};\n    }}\n}}\n",
        syntax.namespace,
        syntax.class_name,
        syntax.return_type,
        syntax.method_name,
        syntax.return_expression,
    ))
}

fn validate_member_access_expression_syntax(
    syntax: &CSharpMemberAccessExpressionSyntax,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "FileScopedNamespaceDeclarationSyntax.Name",
        &syntax.namespace,
        "Tct.Contracts",
    )?;
    require_string(
        "ClassDeclarationSyntax.Identifier",
        &syntax.class_name,
        "ClockContract",
    )?;
    require_string(
        "MethodDeclarationSyntax.Identifier",
        &syntax.method_name,
        "Read",
    )?;
    require_string(
        "MethodDeclarationSyntax.ReturnType",
        &syntax.return_type,
        "string",
    )?;
    require_string(
        "ReturnStatementSyntax.Expression",
        &syntax.return_expression,
        "DateTime.Now",
    )
}

fn decode_compilation_unit_source_file(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpCompilationUnitSourceFileSyntax, SyntaxBackendBoundaryError> {
    let usings = node_values_field(
        "CompilationUnitSyntax.Usings",
        root,
        "Usings",
        &["UsingDirectiveSyntax"],
    )?
    .into_iter()
    .map(decode_using_directive)
    .collect::<Result<Vec<_>, _>>()?;
    let namespace_node = single_node_field(
        "CompilationUnitSyntax.Members",
        root,
        "Members",
        "NamespaceDeclarationSyntax",
    )?;
    let namespace = single_terminal_payload(
        "NamespaceDeclarationSyntax.Name",
        namespace_node,
        "Name",
        "qualified_identifier",
        "identifier_payload",
    )?;
    let declarations = node_values_field(
        "NamespaceDeclarationSyntax.Members",
        namespace_node,
        "Members",
        &["ClassDeclarationSyntax", "StructDeclarationSyntax"],
    )?
    .into_iter()
    .map(decode_type_declaration)
    .collect::<Result<Vec<_>, _>>()?;
    Ok(CSharpCompilationUnitSourceFileSyntax {
        usings,
        namespace,
        declarations,
    })
}

fn decode_using_directive(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    let name_node = single_node_field_any(
        "UsingDirectiveSyntax.Name",
        node,
        "Name",
        &["QualifiedNameSyntax", "IdentifierNameSyntax"],
    )?;
    decode_name_syntax(name_node)
}

fn decode_name_syntax(node: &RawSyntaxDecodedNode) -> Result<String, SyntaxBackendBoundaryError> {
    match node.node_kind.as_str() {
        "IdentifierNameSyntax" => single_terminal_payload(
            "IdentifierNameSyntax.Identifier",
            node,
            "Identifier",
            "identifier",
            "identifier_payload",
        ),
        "QualifiedNameSyntax" => {
            let left = single_node_field_any(
                "QualifiedNameSyntax.Left",
                node,
                "Left",
                &["QualifiedNameSyntax", "IdentifierNameSyntax"],
            )?;
            let right = single_node_field(
                "QualifiedNameSyntax.Right",
                node,
                "Right",
                "IdentifierNameSyntax",
            )?;
            Ok(format!(
                "{}.{}",
                decode_name_syntax(left)?,
                decode_name_syntax(right)?
            ))
        }
        _ => Err(render_error(
            "NameSyntax",
            format!("unsupported C# name node kind {}", node.node_kind),
        )),
    }
}

fn decode_type_declaration(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpTypeDeclarationSyntax, SyntaxBackendBoundaryError> {
    let kind = match node.node_kind.as_str() {
        "ClassDeclarationSyntax" => CSharpTypeDeclarationKind::Class,
        "StructDeclarationSyntax" => CSharpTypeDeclarationKind::Struct,
        _ => {
            return Err(render_error(
                "NamespaceDeclarationSyntax.Members",
                format!("unsupported member node kind {}", node.node_kind),
            ));
        }
    };
    let name = single_terminal_payload(
        "TypeDeclarationSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let mut properties = Vec::new();
    let mut constructors = Vec::new();
    for member in node_values_field(
        "TypeDeclarationSyntax.Members",
        node,
        "Members",
        &["PropertyDeclarationSyntax", "ConstructorDeclarationSyntax"],
    )? {
        match member.node_kind.as_str() {
            "PropertyDeclarationSyntax" => properties.push(decode_property(member)?),
            "ConstructorDeclarationSyntax" => constructors.push(decode_constructor(member)?),
            _ => {
                return Err(render_error(
                    "TypeDeclarationSyntax.Members",
                    format!("unsupported type member node kind {}", member.node_kind),
                ));
            }
        }
    }
    if constructors.len() != 1 {
        return Err(render_error(
            "TypeDeclarationSyntax.Members",
            format!(
                "Shapes.cs renderer expects exactly one constructor for {name}, got {}",
                constructors.len()
            ),
        ));
    }
    Ok(CSharpTypeDeclarationSyntax {
        kind,
        name,
        properties,
        constructor: constructors.remove(0),
    })
}

fn decode_property(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpPropertySyntax, SyntaxBackendBoundaryError> {
    let name = single_terminal_payload(
        "PropertyDeclarationSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let type_node = single_node_field(
        "PropertyDeclarationSyntax.Type",
        node,
        "Type",
        "PredefinedTypeSyntax",
    )?;
    let type_keyword = decode_predefined_type(type_node)?;
    let accessor_list = single_node_field(
        "PropertyDeclarationSyntax.AccessorList",
        node,
        "AccessorList",
        "AccessorListSyntax",
    )?;
    let accessors = node_values_field(
        "AccessorListSyntax.Accessors",
        accessor_list,
        "Accessors",
        &["AccessorDeclarationSyntax"],
    )?;
    if accessors.len() != 1 {
        return Err(render_error(
            "AccessorListSyntax.Accessors",
            format!(
                "Shapes.cs renderer expects one accessor for property {name}, got {}",
                accessors.len()
            ),
        ));
    }
    let accessor_keyword = single_terminal_payload(
        "AccessorDeclarationSyntax.Keyword",
        accessors[0],
        "Keyword",
        "get_accessor_keyword",
        "string_payload",
    )?;
    Ok(CSharpPropertySyntax {
        type_keyword,
        name,
        accessor_keyword,
    })
}

fn decode_constructor(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpConstructorSyntax, SyntaxBackendBoundaryError> {
    let name = single_terminal_payload(
        "ConstructorDeclarationSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let parameter_list = single_node_field(
        "ConstructorDeclarationSyntax.ParameterList",
        node,
        "ParameterList",
        "ParameterListSyntax",
    )?;
    let parameters = decode_parameter_list(parameter_list)?;
    let body = single_node_field(
        "ConstructorDeclarationSyntax.Body",
        node,
        "Body",
        "BlockSyntax",
    )?;
    let assignments = node_values_field(
        "BlockSyntax.Statements",
        body,
        "Statements",
        &["ExpressionStatementSyntax"],
    )?
    .into_iter()
    .map(decode_expression_statement_assignment)
    .collect::<Result<Vec<_>, _>>()?;
    Ok(CSharpConstructorSyntax {
        name,
        parameters,
        assignments,
    })
}

fn decode_parameter(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpParameterSyntax, SyntaxBackendBoundaryError> {
    let type_node = single_node_field_any(
        "ParameterSyntax.Type",
        node,
        "Type",
        &["IdentifierNameSyntax", "PredefinedTypeSyntax"],
    )?;
    let name = single_terminal_payload(
        "ParameterSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    Ok(CSharpParameterSyntax {
        type_keyword: decode_type_syntax(type_node)?,
        name,
    })
}

fn decode_expression_statement_assignment(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpAssignmentSyntax, SyntaxBackendBoundaryError> {
    let assignment = single_node_field(
        "ExpressionStatementSyntax.Expression",
        node,
        "Expression",
        "AssignmentExpressionSyntax",
    )?;
    decode_assignment_expression(assignment)
}

fn decode_assignment_expression(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpAssignmentSyntax, SyntaxBackendBoundaryError> {
    let left = single_node_field(
        "AssignmentExpressionSyntax.Left",
        node,
        "Left",
        "IdentifierNameSyntax",
    )?;
    let right = single_node_field(
        "AssignmentExpressionSyntax.Right",
        node,
        "Right",
        "IdentifierNameSyntax",
    )?;
    Ok(CSharpAssignmentSyntax {
        left: decode_name_syntax(left)?,
        right: decode_name_syntax(right)?,
    })
}

fn decode_predefined_type(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    single_terminal_payload_any(
        "PredefinedTypeSyntax.Keyword",
        node,
        "Keyword",
        &["double_keyword", "string_keyword", "void_keyword"],
        "identifier_payload",
    )
}

fn render_compilation_unit_source_file_source(
    syntax: &CSharpCompilationUnitSourceFileSyntax,
) -> Result<String, SyntaxBackendBoundaryError> {
    validate_shapes_source_file_syntax(syntax)?;
    let mut source = String::new();
    source.push('\u{feff}');
    for using_name in &syntax.usings {
        source.push_str("using ");
        source.push_str(using_name);
        source.push_str(";\n");
    }
    source.push_str("\nnamespace ");
    source.push_str(&syntax.namespace);
    source.push_str("\n{\n    #region 01_ShapeDefinitions\n");
    for declaration in &syntax.declarations {
        render_type_declaration(&mut source, declaration);
    }
    source.push_str("    #endregion\n\n}\n");
    Ok(source)
}

fn render_type_declaration(source: &mut String, declaration: &CSharpTypeDeclarationSyntax) {
    let type_keyword = match declaration.kind {
        CSharpTypeDeclarationKind::Class => "class",
        CSharpTypeDeclarationKind::Struct => "struct",
    };
    source.push_str("    public ");
    source.push_str(type_keyword);
    source.push(' ');
    source.push_str(&declaration.name);
    source.push_str("\n    {\n");
    for property in &declaration.properties {
        source.push_str("        public ");
        source.push_str(&property.type_keyword);
        source.push(' ');
        source.push_str(&property.name);
        source.push_str(" { ");
        source.push_str(&property.accessor_keyword);
        source.push_str("; }\n");
    }
    source.push('\n');
    source.push_str("        public ");
    source.push_str(&declaration.constructor.name);
    source.push('(');
    render_parameters(source, &declaration.constructor.parameters);
    source.push_str(")\n        {\n");
    for assignment in &declaration.constructor.assignments {
        source.push_str("            ");
        source.push_str(&assignment.left);
        source.push_str(" = ");
        source.push_str(&assignment.right);
        source.push_str(";\n");
    }
    source.push_str("        }\n    }\n");
}

fn validate_shapes_source_file_syntax(
    syntax: &CSharpCompilationUnitSourceFileSyntax,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string_vec(
        "CompilationUnitSyntax.Usings",
        &syntax.usings,
        &[
            "System",
            "System.Collections.Generic",
            "System.Linq",
            "System.Text",
            "System.Threading.Tasks",
        ],
    )?;
    require_string(
        "NamespaceDeclarationSyntax.Name",
        &syntax.namespace,
        "PatternMatching",
    )?;
    if syntax.declarations.len() != 4 {
        return Err(render_error(
            "NamespaceDeclarationSyntax.Members",
            format!(
                "Shapes.cs renderer expects four type declarations, got {}",
                syntax.declarations.len()
            ),
        ));
    }
    validate_shape_type(
        &syntax.declarations[0],
        CSharpTypeDeclarationKind::Class,
        "Square",
        &["Side"],
        &[("double", "side")],
        &[("Side", "side")],
    )?;
    validate_shape_type(
        &syntax.declarations[1],
        CSharpTypeDeclarationKind::Class,
        "Circle",
        &["Radius"],
        &[("double", "radius")],
        &[("Radius", "radius")],
    )?;
    validate_shape_type(
        &syntax.declarations[2],
        CSharpTypeDeclarationKind::Struct,
        "Rectangle",
        &["Length", "Height"],
        &[("double", "length"), ("double", "height")],
        &[("Length", "length"), ("Height", "height")],
    )?;
    validate_shape_type(
        &syntax.declarations[3],
        CSharpTypeDeclarationKind::Class,
        "Triangle",
        &["Base", "Height"],
        &[("double", "@base"), ("double", "height")],
        &[("Base", "@base"), ("Height", "height")],
    )
}

fn validate_shape_type(
    actual: &CSharpTypeDeclarationSyntax,
    expected_kind: CSharpTypeDeclarationKind,
    expected_name: &str,
    expected_properties: &[&str],
    expected_parameters: &[(&str, &str)],
    expected_assignments: &[(&str, &str)],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.kind != expected_kind {
        return Err(render_error(
            "TypeDeclarationSyntax.kind",
            format!("unexpected declaration kind for {}", actual.name),
        ));
    }
    require_string(
        "TypeDeclarationSyntax.Identifier",
        &actual.name,
        expected_name,
    )?;
    let property_names = actual
        .properties
        .iter()
        .map(|property| property.name.as_str())
        .collect::<Vec<_>>();
    require_string_slice(
        "PropertyDeclarationSyntax.Identifier",
        &property_names,
        expected_properties,
    )?;
    for property in &actual.properties {
        require_string(
            "PropertyDeclarationSyntax.Type",
            &property.type_keyword,
            "double",
        )?;
        require_string(
            "AccessorDeclarationSyntax.Keyword",
            &property.accessor_keyword,
            "get",
        )?;
    }
    require_string(
        "ConstructorDeclarationSyntax.Identifier",
        &actual.constructor.name,
        expected_name,
    )?;
    let parameters = parameter_pairs(&actual.constructor.parameters);
    require_pair_slice(
        "ParameterListSyntax.Parameters",
        &parameters,
        expected_parameters,
    )?;
    let assignments = actual
        .constructor
        .assignments
        .iter()
        .map(|assignment| (assignment.left.as_str(), assignment.right.as_str()))
        .collect::<Vec<_>>();
    require_pair_slice("BlockSyntax.Statements", &assignments, expected_assignments)
}

fn decode_compilation_unit_logger_source_file(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpCompilationUnitLoggerSourceFileSyntax, SyntaxBackendBoundaryError> {
    let (namespace, members) = decode_file_scoped_namespace_members(
        root,
        &["EnumDeclarationSyntax", "ClassDeclarationSyntax"],
        2,
        "Logger.cs",
        "enum plus class declarations",
    )?;
    let enum_declaration = decode_enum_declaration(members[0])?;
    let class_declaration = decode_logger_class_declaration(members[1])?;
    Ok(CSharpCompilationUnitLoggerSourceFileSyntax {
        namespace,
        enum_declaration,
        class_declaration,
    })
}

fn decode_compilation_unit_version_one_source_file(
    root: &RawSyntaxDecodedNode,
) -> Result<CSharpCompilationUnitVersionOneSourceFileSyntax, SyntaxBackendBoundaryError> {
    let (namespace, members) = decode_file_scoped_namespace_members(
        root,
        &["ClassDeclarationSyntax"],
        2,
        "VersionOne.cs",
        "two class declarations",
    )?;
    let file_found_args_class = decode_version_one_file_found_args_class(members[0])?;
    let file_searcher_class = decode_version_one_file_searcher_class(members[1])?;
    Ok(CSharpCompilationUnitVersionOneSourceFileSyntax {
        namespace,
        file_found_args_class,
        file_searcher_class,
    })
}

fn decode_file_scoped_namespace_members<'a>(
    root: &'a RawSyntaxDecodedNode,
    allowed_member_kinds: &[&str],
    expected_member_count: usize,
    source_label: &str,
    expected_description: &str,
) -> Result<(String, Vec<&'a RawSyntaxDecodedNode>), SyntaxBackendBoundaryError> {
    let namespace_node = single_node_field(
        "CompilationUnitSyntax.Members",
        root,
        "Members",
        "FileScopedNamespaceDeclarationSyntax",
    )?;
    let namespace = single_terminal_payload(
        "FileScopedNamespaceDeclarationSyntax.Name",
        namespace_node,
        "Name",
        "qualified_identifier",
        "identifier_payload",
    )?;
    let members = node_values_field(
        "FileScopedNamespaceDeclarationSyntax.Members",
        namespace_node,
        "Members",
        allowed_member_kinds,
    )?;
    if members.len() != expected_member_count {
        return Err(render_error(
            "FileScopedNamespaceDeclarationSyntax.Members",
            format!(
                "{source_label} renderer expects {expected_description}, got {}",
                members.len(),
            ),
        ));
    }
    Ok((namespace, members))
}

fn decode_version_one_file_found_args_class(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpVersionOneFileFoundArgsClassSyntax, SyntaxBackendBoundaryError> {
    require_node(
        "FileScopedNamespaceDeclarationSyntax.Members",
        node,
        "ClassDeclarationSyntax",
    )?;
    let name = single_terminal_payload(
        "ClassDeclarationSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let base_list = single_node_field(
        "ClassDeclarationSyntax.BaseList",
        node,
        "BaseList",
        "BaseListSyntax",
    )?;
    let base_types = decode_base_list(base_list)?;
    let members = node_values_field(
        "ClassDeclarationSyntax.Members",
        node,
        "Members",
        &["PropertyDeclarationSyntax", "ConstructorDeclarationSyntax"],
    )?;
    if members.len() != 2 {
        return Err(render_error(
            "ClassDeclarationSyntax.Members",
            format!(
                "VersionOne.cs FileFoundArgs renderer expects property plus constructor, got {}",
                members.len()
            ),
        ));
    }
    Ok(CSharpVersionOneFileFoundArgsClassSyntax {
        name,
        base_types,
        property: decode_property(members[0])?,
        constructor: decode_arrow_constructor(members[1])?,
    })
}

fn decode_version_one_file_searcher_class(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpVersionOneFileSearcherClassSyntax, SyntaxBackendBoundaryError> {
    require_node(
        "FileScopedNamespaceDeclarationSyntax.Members",
        node,
        "ClassDeclarationSyntax",
    )?;
    let name = single_terminal_payload(
        "ClassDeclarationSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    if optional_node_field_any(
        "ClassDeclarationSyntax.BaseList",
        node,
        "BaseList",
        &["BaseListSyntax"],
    )?
    .is_some()
    {
        return Err(render_error(
            "ClassDeclarationSyntax.BaseList",
            "VersionOne.cs FileSearcher should not declare a base list",
        ));
    }
    let members = node_values_field(
        "ClassDeclarationSyntax.Members",
        node,
        "Members",
        &["EventFieldDeclarationSyntax", "MethodDeclarationSyntax"],
    )?;
    if members.len() != 2 {
        return Err(render_error(
            "ClassDeclarationSyntax.Members",
            format!(
                "VersionOne.cs FileSearcher renderer expects event plus method, got {}",
                members.len()
            ),
        ));
    }
    Ok(CSharpVersionOneFileSearcherClassSyntax {
        name,
        event: decode_event_field_declaration(members[0])?,
        method: decode_method_declaration(members[1])?,
    })
}

fn decode_base_list(
    node: &RawSyntaxDecodedNode,
) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
    node_values_field(
        "BaseListSyntax.Types",
        node,
        "Types",
        &["SimpleBaseTypeSyntax"],
    )?
    .into_iter()
    .map(|base_type| {
        let type_node = single_node_field(
            "SimpleBaseTypeSyntax.Type",
            base_type,
            "Type",
            "IdentifierNameSyntax",
        )?;
        decode_type_syntax(type_node)
    })
    .collect()
}

fn decode_arrow_constructor(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpArrowConstructorSyntax, SyntaxBackendBoundaryError> {
    let name = single_terminal_payload(
        "ConstructorDeclarationSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let parameter_list = single_node_field(
        "ConstructorDeclarationSyntax.ParameterList",
        node,
        "ParameterList",
        "ParameterListSyntax",
    )?;
    let parameters = decode_parameter_list(parameter_list)?;
    let expression_body = single_node_field(
        "ConstructorDeclarationSyntax.ExpressionBody",
        node,
        "ExpressionBody",
        "ArrowExpressionClauseSyntax",
    )?;
    let assignment = single_node_field(
        "ArrowExpressionClauseSyntax.Expression",
        expression_body,
        "Expression",
        "AssignmentExpressionSyntax",
    )?;
    Ok(CSharpArrowConstructorSyntax {
        name,
        parameters,
        assignment: decode_assignment_expression(assignment)?,
    })
}

fn decode_event_field_declaration(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpEventFieldDeclarationSyntax, SyntaxBackendBoundaryError> {
    let declaration = single_node_field(
        "EventFieldDeclarationSyntax.Declaration",
        node,
        "Declaration",
        "VariableDeclarationSyntax",
    )?;
    Ok(CSharpEventFieldDeclarationSyntax {
        declaration: decode_variable_declaration(declaration)?,
    })
}

fn decode_enum_declaration(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpEnumDeclarationSyntax, SyntaxBackendBoundaryError> {
    require_node(
        "FileScopedNamespaceDeclarationSyntax.Members",
        node,
        "EnumDeclarationSyntax",
    )?;
    let name = single_terminal_payload(
        "EnumDeclarationSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let members = node_values_field(
        "EnumDeclarationSyntax.Members",
        node,
        "Members",
        &["EnumMemberDeclarationSyntax"],
    )?
    .into_iter()
    .map(|member| {
        single_terminal_payload(
            "EnumMemberDeclarationSyntax.Identifier",
            member,
            "Identifier",
            "identifier",
            "identifier_payload",
        )
    })
    .collect::<Result<Vec<_>, _>>()?;
    Ok(CSharpEnumDeclarationSyntax { name, members })
}

fn decode_logger_class_declaration(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpLoggerClassSyntax, SyntaxBackendBoundaryError> {
    require_node(
        "FileScopedNamespaceDeclarationSyntax.Members",
        node,
        "ClassDeclarationSyntax",
    )?;
    let name = single_terminal_payload(
        "ClassDeclarationSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let members = node_values_field(
        "ClassDeclarationSyntax.Members",
        node,
        "Members",
        &[
            "FieldDeclarationSyntax",
            "PropertyDeclarationSyntax",
            "MethodDeclarationSyntax",
        ],
    )?;
    if members.len() != 3 {
        return Err(render_error(
            "ClassDeclarationSyntax.Members",
            format!(
                "Logger.cs renderer expects field, property, method; got {}",
                members.len()
            ),
        ));
    }
    let field = decode_field_declaration(members[0])?;
    let property = decode_logger_property(members[1])?;
    let method = decode_method_declaration(members[2])?;
    Ok(CSharpLoggerClassSyntax {
        name,
        field,
        property,
        method,
    })
}

fn decode_field_declaration(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpFieldDeclarationSyntax, SyntaxBackendBoundaryError> {
    let declaration = single_node_field(
        "FieldDeclarationSyntax.Declaration",
        node,
        "Declaration",
        "VariableDeclarationSyntax",
    )?;
    Ok(CSharpFieldDeclarationSyntax {
        declaration: decode_variable_declaration(declaration)?,
    })
}

fn decode_logger_property(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpLoggerPropertySyntax, SyntaxBackendBoundaryError> {
    let name = single_terminal_payload(
        "PropertyDeclarationSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let type_node = single_node_field(
        "PropertyDeclarationSyntax.Type",
        node,
        "Type",
        "IdentifierNameSyntax",
    )?;
    let type_name = decode_type_syntax(type_node)?;
    let accessor_list = single_node_field(
        "PropertyDeclarationSyntax.AccessorList",
        node,
        "AccessorList",
        "AccessorListSyntax",
    )?;
    let accessors = node_values_field(
        "AccessorListSyntax.Accessors",
        accessor_list,
        "Accessors",
        &["AccessorDeclarationSyntax"],
    )?
    .into_iter()
    .map(decode_accessor_keyword)
    .collect::<Result<Vec<_>, _>>()?;
    let initializer = single_node_field(
        "PropertyDeclarationSyntax.Initializer",
        node,
        "Initializer",
        "EqualsValueClauseSyntax",
    )?;
    Ok(CSharpLoggerPropertySyntax {
        type_name,
        name,
        accessors,
        initializer: decode_equals_value_clause(initializer)?,
    })
}

fn decode_method_declaration(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpMethodDeclarationSyntax, SyntaxBackendBoundaryError> {
    let name = single_terminal_payload(
        "MethodDeclarationSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let return_type = decode_type_syntax(single_node_field_any(
        "MethodDeclarationSyntax.ReturnType",
        node,
        "ReturnType",
        &["PredefinedTypeSyntax", "IdentifierNameSyntax"],
    )?)?;
    let parameter_list = single_node_field(
        "MethodDeclarationSyntax.ParameterList",
        node,
        "ParameterList",
        "ParameterListSyntax",
    )?;
    let parameters = decode_parameter_list(parameter_list)?;
    let body = single_node_field("MethodDeclarationSyntax.Body", node, "Body", "BlockSyntax")?;
    let statements = node_values_field(
        "BlockSyntax.Statements",
        body,
        "Statements",
        &[
            "ForEachStatementSyntax",
            "IfStatementSyntax",
            "LocalDeclarationStatementSyntax",
            "ExpressionStatementSyntax",
        ],
    )?
    .into_iter()
    .map(decode_statement)
    .collect::<Result<Vec<_>, _>>()?;
    Ok(CSharpMethodDeclarationSyntax {
        return_type,
        name,
        parameters,
        statements,
    })
}

fn decode_variable_declaration(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpVariableDeclarationSyntax, SyntaxBackendBoundaryError> {
    let type_node = single_node_field_any(
        "VariableDeclarationSyntax.Type",
        node,
        "Type",
        &["NullableTypeSyntax", "IdentifierNameSyntax"],
    )?;
    let variables = node_values_field(
        "VariableDeclarationSyntax.Variables",
        node,
        "Variables",
        &["VariableDeclaratorSyntax"],
    )?
    .into_iter()
    .map(decode_variable_declarator)
    .collect::<Result<Vec<_>, _>>()?;
    Ok(CSharpVariableDeclarationSyntax {
        type_name: decode_type_syntax(type_node)?,
        variables,
    })
}

fn decode_variable_declarator(
    node: &RawSyntaxDecodedNode,
) -> Result<CSharpVariableDeclaratorSyntax, SyntaxBackendBoundaryError> {
    let name = single_terminal_payload(
        "VariableDeclaratorSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let initializer = if node.field("Initializer").is_some() {
        optional_node_field_any(
            "VariableDeclaratorSyntax.Initializer",
            node,
            "Initializer",
            &["EqualsValueClauseSyntax"],
        )?
        .map(decode_equals_value_clause)
        .transpose()?
    } else {
        None
    };
    Ok(CSharpVariableDeclaratorSyntax { name, initializer })
}

fn decode_parameter_list(
    node: &RawSyntaxDecodedNode,
) -> Result<Vec<CSharpParameterSyntax>, SyntaxBackendBoundaryError> {
    node_values_field(
        "ParameterListSyntax.Parameters",
        node,
        "Parameters",
        &["ParameterSyntax"],
    )?
    .into_iter()
    .map(decode_parameter)
    .collect()
}

fn decode_accessor_keyword(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    single_terminal_payload_any(
        "AccessorDeclarationSyntax.Keyword",
        node,
        "Keyword",
        &["get_accessor_keyword", "set_accessor_keyword"],
        "string_payload",
    )
}

fn decode_type_syntax(node: &RawSyntaxDecodedNode) -> Result<String, SyntaxBackendBoundaryError> {
    match node.node_kind.as_str() {
        "IdentifierNameSyntax" => decode_name_syntax(node),
        "PredefinedTypeSyntax" => single_terminal_payload_any(
            "PredefinedTypeSyntax.Keyword",
            node,
            "Keyword",
            &["double_keyword", "string_keyword", "void_keyword"],
            "identifier_payload",
        ),
        "NullableTypeSyntax" => {
            let element = single_node_field(
                "NullableTypeSyntax.ElementType",
                node,
                "ElementType",
                "GenericNameSyntax",
            )?;
            Ok(format!("{}?", decode_type_syntax(element)?))
        }
        "GenericNameSyntax" => {
            let name = single_terminal_payload(
                "GenericNameSyntax.Identifier",
                node,
                "Identifier",
                "identifier",
                "identifier_payload",
            )?;
            let args = single_node_field(
                "GenericNameSyntax.TypeArgumentList",
                node,
                "TypeArgumentList",
                "TypeArgumentListSyntax",
            )?;
            let arguments = node_values_field(
                "TypeArgumentListSyntax.Arguments",
                args,
                "Arguments",
                &["PredefinedTypeSyntax", "IdentifierNameSyntax"],
            )?
            .into_iter()
            .map(decode_type_syntax)
            .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("{}<{}>", name, arguments.join(", ")))
        }
        _ => Err(render_error(
            "TypeSyntax",
            format!("unsupported C# type node kind {}", node.node_kind),
        )),
    }
}

fn decode_statement(node: &RawSyntaxDecodedNode) -> Result<String, SyntaxBackendBoundaryError> {
    match node.node_kind.as_str() {
        "ForEachStatementSyntax" => decode_for_each_statement(node),
        "IfStatementSyntax" => decode_if_statement(node),
        "LocalDeclarationStatementSyntax" => decode_local_declaration_statement(node),
        "ExpressionStatementSyntax" => {
            let expression = single_node_field_any(
                "ExpressionStatementSyntax.Expression",
                node,
                "Expression",
                &[
                    "InvocationExpressionSyntax",
                    "ConditionalAccessExpressionSyntax",
                ],
            )?;
            Ok(format!("{};", decode_expression(expression)?))
        }
        _ => Err(render_error(
            "StatementSyntax",
            format!("unsupported C# statement node kind {}", node.node_kind),
        )),
    }
}

fn decode_for_each_statement(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    let type_node = single_node_field_any(
        "ForEachStatementSyntax.Type",
        node,
        "Type",
        &["IdentifierNameSyntax", "PredefinedTypeSyntax"],
    )?;
    let identifier = single_terminal_payload(
        "ForEachStatementSyntax.Identifier",
        node,
        "Identifier",
        "identifier",
        "identifier_payload",
    )?;
    let expression = single_node_field(
        "ForEachStatementSyntax.Expression",
        node,
        "Expression",
        "InvocationExpressionSyntax",
    )?;
    let statement = single_node_field(
        "ForEachStatementSyntax.Statement",
        node,
        "Statement",
        "BlockSyntax",
    )?;
    let statements = node_values_field(
        "BlockSyntax.Statements",
        statement,
        "Statements",
        &["ExpressionStatementSyntax"],
    )?
    .into_iter()
    .map(decode_statement)
    .collect::<Result<Vec<_>, _>>()?;
    if statements.len() != 1 {
        return Err(render_error(
            "ForEachStatementSyntax.Statement",
            format!(
                "VersionOne.cs foreach renderer expects one statement, got {}",
                statements.len()
            ),
        ));
    }
    Ok(format!(
        "foreach ({} {} in {})\n        {{\n            {}\n        }}",
        decode_type_syntax(type_node)?,
        identifier,
        decode_expression(expression)?,
        statements[0],
    ))
}

fn decode_if_statement(node: &RawSyntaxDecodedNode) -> Result<String, SyntaxBackendBoundaryError> {
    let condition = single_node_field(
        "IfStatementSyntax.Condition",
        node,
        "Condition",
        "ParenthesizedExpressionSyntax",
    )?;
    let statement = single_node_field(
        "IfStatementSyntax.Statement",
        node,
        "Statement",
        "ReturnStatementSyntax",
    )?;
    require_node(
        "IfStatementSyntax.Statement",
        statement,
        "ReturnStatementSyntax",
    )?;
    Ok(format!(
        "if {}\n            return;",
        decode_expression(condition)?
    ))
}

fn decode_local_declaration_statement(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    let declaration = single_node_field(
        "LocalDeclarationStatementSyntax.Declaration",
        node,
        "Declaration",
        "VariableDeclarationSyntax",
    )?;
    let declaration = decode_variable_declaration(declaration)?;
    let variable =
        single_variable_declarator("LocalDeclarationStatementSyntax.Declaration", &declaration)?;
    let initializer = variable.initializer.as_deref().ok_or_else(|| {
        render_error(
            "VariableDeclaratorSyntax.Initializer",
            "local declaration requires an initializer",
        )
    })?;
    Ok(format!(
        "{} {} = {};",
        declaration.type_name, variable.name, initializer
    ))
}

fn decode_equals_value_clause(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    let value = single_node_field_any(
        "EqualsValueClauseSyntax.Value",
        node,
        "Value",
        &[
            "MemberAccessExpressionSyntax",
            "InterpolatedStringExpressionSyntax",
        ],
    )?;
    decode_expression(value)
}

fn decode_expression(node: &RawSyntaxDecodedNode) -> Result<String, SyntaxBackendBoundaryError> {
    match node.node_kind.as_str() {
        "IdentifierNameSyntax" => decode_name_syntax(node),
        "AssignmentExpressionSyntax" => {
            let assignment = decode_assignment_expression(node)?;
            Ok(format!("{} = {}", assignment.left, assignment.right))
        }
        "MemberAccessExpressionSyntax" => {
            let expression = single_node_field_any(
                "MemberAccessExpressionSyntax.Expression",
                node,
                "Expression",
                &["IdentifierNameSyntax", "MemberAccessExpressionSyntax"],
            )?;
            let name = single_node_field(
                "MemberAccessExpressionSyntax.Name",
                node,
                "Name",
                "IdentifierNameSyntax",
            )?;
            Ok(format!(
                "{}.{}",
                decode_expression(expression)?,
                decode_name_syntax(name)?
            ))
        }
        "ParenthesizedExpressionSyntax" => {
            let expression = single_node_field_any(
                "ParenthesizedExpressionSyntax.Expression",
                node,
                "Expression",
                &["BinaryExpressionSyntax", "IsPatternExpressionSyntax"],
            )?;
            Ok(format!("({})", decode_expression(expression)?))
        }
        "BinaryExpressionSyntax" => decode_binary_expression(node),
        "IsPatternExpressionSyntax" => {
            let expression = single_node_field(
                "IsPatternExpressionSyntax.Expression",
                node,
                "Expression",
                "IdentifierNameSyntax",
            )?;
            let pattern = single_node_field(
                "IsPatternExpressionSyntax.Pattern",
                node,
                "Pattern",
                "ConstantPatternSyntax",
            )?;
            Ok(format!(
                "{} is {}",
                decode_expression(expression)?,
                decode_constant_pattern(pattern)?
            ))
        }
        "LiteralExpressionSyntax" => single_terminal_payload(
            "LiteralExpressionSyntax.Token",
            node,
            "Token",
            "null_literal",
            "identifier_payload",
        ),
        "ConditionalAccessExpressionSyntax" => decode_conditional_access_expression(node),
        "InterpolatedStringExpressionSyntax" => decode_interpolated_string_expression(node),
        "InvocationExpressionSyntax" => decode_invocation_expression(node),
        "MemberBindingExpressionSyntax" => {
            let name = single_node_field(
                "MemberBindingExpressionSyntax.Name",
                node,
                "Name",
                "IdentifierNameSyntax",
            )?;
            Ok(format!(".{}", decode_name_syntax(name)?))
        }
        "ObjectCreationExpressionSyntax" => decode_object_creation_expression(node),
        "ThisExpressionSyntax" => Ok("this".to_string()),
        _ => Err(render_error(
            "ExpressionSyntax",
            format!("unsupported C# expression node kind {}", node.node_kind),
        )),
    }
}

fn decode_conditional_access_expression(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    let expression = single_node_field(
        "ConditionalAccessExpressionSyntax.Expression",
        node,
        "Expression",
        "IdentifierNameSyntax",
    )?;
    let when_not_null = single_node_field(
        "ConditionalAccessExpressionSyntax.WhenNotNull",
        node,
        "WhenNotNull",
        "InvocationExpressionSyntax",
    )?;
    Ok(format!(
        "{}?{}",
        decode_expression(expression)?,
        decode_expression(when_not_null)?
    ))
}

fn decode_binary_expression(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    let left = decode_expression(single_node_field_any(
        "BinaryExpressionSyntax.Left",
        node,
        "Left",
        &["ParenthesizedExpressionSyntax", "IdentifierNameSyntax"],
    )?)?;
    let right = decode_expression(single_node_field_any(
        "BinaryExpressionSyntax.Right",
        node,
        "Right",
        &["ParenthesizedExpressionSyntax", "IdentifierNameSyntax"],
    )?)?;
    let operator = match (left.as_str(), right.as_str()) {
        ("s", "LogLevel") => "<",
        ("(s < LogLevel)", "(WriteMessage is null)") => "||",
        _ => {
            return Err(render_error(
                "BinaryExpressionSyntax",
                format!("unsupported Logger.cs binary expression {left} ? {right}"),
            ));
        }
    };
    Ok(format!("{left} {operator} {right}"))
}

fn decode_constant_pattern(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    let expression = single_node_field(
        "ConstantPatternSyntax.Expression",
        node,
        "Expression",
        "LiteralExpressionSyntax",
    )?;
    decode_expression(expression)
}

fn decode_interpolated_string_expression(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    let contents = node_values_field(
        "InterpolatedStringExpressionSyntax.Contents",
        node,
        "Contents",
        &["InterpolatedStringTextSyntax", "InterpolationSyntax"],
    )?;
    let mut rendered = String::from("$\"");
    for content in contents {
        match content.node_kind.as_str() {
            "InterpolatedStringTextSyntax" => {
                rendered.push_str(&single_terminal_payload(
                    "InterpolatedStringTextSyntax.TextToken",
                    content,
                    "TextToken",
                    "interpolated_text",
                    "string_payload",
                )?);
            }
            "InterpolationSyntax" => {
                let expression = single_node_field_any(
                    "InterpolationSyntax.Expression",
                    content,
                    "Expression",
                    &["MemberAccessExpressionSyntax", "IdentifierNameSyntax"],
                )?;
                rendered.push('{');
                rendered.push_str(&decode_expression(expression)?);
                rendered.push('}');
            }
            _ => {
                return Err(render_error(
                    "InterpolatedStringExpressionSyntax.Contents",
                    format!(
                        "unsupported interpolated string content {}",
                        content.node_kind
                    ),
                ));
            }
        }
    }
    rendered.push('"');
    Ok(rendered)
}

fn decode_invocation_expression(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    let expression = single_node_field_any(
        "InvocationExpressionSyntax.Expression",
        node,
        "Expression",
        &[
            "IdentifierNameSyntax",
            "MemberAccessExpressionSyntax",
            "MemberBindingExpressionSyntax",
        ],
    )?;
    let argument_list = single_node_field(
        "InvocationExpressionSyntax.ArgumentList",
        node,
        "ArgumentList",
        "ArgumentListSyntax",
    )?;
    let arguments = decode_argument_list(argument_list)?;
    Ok(format!(
        "{}({})",
        decode_expression(expression)?,
        arguments.join(", ")
    ))
}

fn decode_object_creation_expression(
    node: &RawSyntaxDecodedNode,
) -> Result<String, SyntaxBackendBoundaryError> {
    let type_node = single_node_field(
        "ObjectCreationExpressionSyntax.Type",
        node,
        "Type",
        "IdentifierNameSyntax",
    )?;
    let argument_list = single_node_field(
        "ObjectCreationExpressionSyntax.ArgumentList",
        node,
        "ArgumentList",
        "ArgumentListSyntax",
    )?;
    Ok(format!(
        "new {}({})",
        decode_type_syntax(type_node)?,
        decode_argument_list(argument_list)?.join(", ")
    ))
}

fn decode_argument_list(
    node: &RawSyntaxDecodedNode,
) -> Result<Vec<String>, SyntaxBackendBoundaryError> {
    node_values_field(
        "ArgumentListSyntax.Arguments",
        node,
        "Arguments",
        &["ArgumentSyntax"],
    )?
    .into_iter()
    .map(|argument| {
        let expression = single_node_field_any(
            "ArgumentSyntax.Expression",
            argument,
            "Expression",
            &[
                "IdentifierNameSyntax",
                "ThisExpressionSyntax",
                "ObjectCreationExpressionSyntax",
            ],
        )?;
        decode_expression(expression)
    })
    .collect()
}

fn render_compilation_unit_logger_source_file_source(
    syntax: &CSharpCompilationUnitLoggerSourceFileSyntax,
) -> Result<String, SyntaxBackendBoundaryError> {
    validate_logger_source_file_syntax(syntax)?;
    let mut source = String::new();
    source.push_str("\u{feff}namespace ");
    source.push_str(&syntax.namespace);
    source.push_str(";\n\npublic enum ");
    source.push_str(&syntax.enum_declaration.name);
    source.push_str("\n{\n");
    for (index, member) in syntax.enum_declaration.members.iter().enumerate() {
        source.push_str("    ");
        source.push_str(member);
        if index + 1 != syntax.enum_declaration.members.len() {
            source.push(',');
        }
        source.push('\n');
    }
    source.push_str("}\n\npublic static class ");
    source.push_str(&syntax.class_declaration.name);
    source.push_str("\n{\n");
    let field = single_variable_declarator(
        "FieldDeclarationSyntax.Declaration",
        &syntax.class_declaration.field.declaration,
    )?;
    source.push_str("    public static ");
    source.push_str(&syntax.class_declaration.field.declaration.type_name);
    source.push(' ');
    source.push_str(&field.name);
    source.push_str(";\n\n    public static ");
    source.push_str(&syntax.class_declaration.property.type_name);
    source.push(' ');
    source.push_str(&syntax.class_declaration.property.name);
    source.push_str(" { ");
    source.push_str(&syntax.class_declaration.property.accessors.join("; "));
    source.push_str("; } = ");
    source.push_str(&syntax.class_declaration.property.initializer);
    source.push_str(";\n\n    public static ");
    source.push_str(&syntax.class_declaration.method.return_type);
    source.push(' ');
    source.push_str(&syntax.class_declaration.method.name);
    source.push('(');
    render_parameters(&mut source, &syntax.class_declaration.method.parameters);
    source.push_str(")\n    {\n");
    for (index, statement) in syntax
        .class_declaration
        .method
        .statements
        .iter()
        .enumerate()
    {
        source.push_str("        ");
        source.push_str(statement);
        source.push('\n');
        if index == 0 {
            source.push('\n');
        }
    }
    source.push_str("    }\n}\n");
    Ok(source)
}

fn validate_logger_source_file_syntax(
    syntax: &CSharpCompilationUnitLoggerSourceFileSyntax,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "FileScopedNamespaceDeclarationSyntax.Name",
        &syntax.namespace,
        "DelegatesAndEvents",
    )?;
    require_string(
        "EnumDeclarationSyntax.Identifier",
        &syntax.enum_declaration.name,
        "Severity",
    )?;
    require_string_vec(
        "EnumDeclarationSyntax.Members",
        &syntax.enum_declaration.members,
        &[
            "Verbose",
            "Trace",
            "Information",
            "Warning",
            "Error",
            "Critical",
        ],
    )?;
    require_string(
        "ClassDeclarationSyntax.Identifier",
        &syntax.class_declaration.name,
        "Logger",
    )?;
    let field = single_variable_declarator(
        "FieldDeclarationSyntax.Declaration",
        &syntax.class_declaration.field.declaration,
    )?;
    require_string(
        "FieldDeclarationSyntax.Declaration.Type",
        &syntax.class_declaration.field.declaration.type_name,
        "Action<string>?",
    )?;
    require_string(
        "FieldDeclarationSyntax.Variable",
        &field.name,
        "WriteMessage",
    )?;
    if field.initializer.is_some() {
        return Err(render_error(
            "FieldDeclarationSyntax.Initializer",
            "Logger.cs WriteMessage field should not have an initializer",
        ));
    }
    require_string(
        "PropertyDeclarationSyntax.Type",
        &syntax.class_declaration.property.type_name,
        "Severity",
    )?;
    require_string(
        "PropertyDeclarationSyntax.Identifier",
        &syntax.class_declaration.property.name,
        "LogLevel",
    )?;
    require_string_vec(
        "AccessorListSyntax.Accessors",
        &syntax.class_declaration.property.accessors,
        &["get", "set"],
    )?;
    require_string(
        "PropertyDeclarationSyntax.Initializer",
        &syntax.class_declaration.property.initializer,
        "Severity.Warning",
    )?;
    require_string(
        "MethodDeclarationSyntax.ReturnType",
        &syntax.class_declaration.method.return_type,
        "void",
    )?;
    require_string(
        "MethodDeclarationSyntax.Identifier",
        &syntax.class_declaration.method.name,
        "LogMessage",
    )?;
    let parameters = parameter_pairs(&syntax.class_declaration.method.parameters);
    require_pair_slice(
        "ParameterListSyntax.Parameters",
        &parameters,
        &[
            ("Severity", "s"),
            ("string", "component"),
            ("string", "msg"),
        ],
    )?;
    require_string_vec(
        "BlockSyntax.Statements",
        &syntax.class_declaration.method.statements,
        &[
            "if ((s < LogLevel) || (WriteMessage is null))\n            return;",
            "var outputMsg = $\"{DateTime.Now}\\t{s}\\t{component}\\t{msg}\";",
            "WriteMessage(outputMsg);",
        ],
    )
}

fn render_compilation_unit_version_one_source_file_source(
    syntax: &CSharpCompilationUnitVersionOneSourceFileSyntax,
) -> Result<String, SyntaxBackendBoundaryError> {
    validate_version_one_source_file_syntax(syntax)?;
    let args_class = &syntax.file_found_args_class;
    let searcher_class = &syntax.file_searcher_class;
    let event = single_variable_declarator(
        "EventFieldDeclarationSyntax.Declaration",
        &searcher_class.event.declaration,
    )?;
    let mut source = String::new();
    source.push_str("\u{feff}namespace ");
    source.push_str(&syntax.namespace);
    source.push_str(";\n\npublic class ");
    source.push_str(&args_class.name);
    source.push_str(" : ");
    source.push_str(&args_class.base_types.join(", "));
    source.push_str("\n{\n    public ");
    source.push_str(&args_class.property.type_keyword);
    source.push(' ');
    source.push_str(&args_class.property.name);
    source.push_str(" { ");
    source.push_str(&args_class.property.accessor_keyword);
    source.push_str("; }\n\n    public ");
    source.push_str(&args_class.constructor.name);
    source.push('(');
    render_parameters(&mut source, &args_class.constructor.parameters);
    source.push_str(") => ");
    source.push_str(&args_class.constructor.assignment.left);
    source.push_str(" = ");
    source.push_str(&args_class.constructor.assignment.right);
    source.push_str(";\n}\n\npublic class ");
    source.push_str(&searcher_class.name);
    source.push_str("\n{\n    public event ");
    source.push_str(&searcher_class.event.declaration.type_name);
    source.push(' ');
    source.push_str(&event.name);
    source.push_str(";\n\n    public ");
    source.push_str(&searcher_class.method.return_type);
    source.push(' ');
    source.push_str(&searcher_class.method.name);
    source.push('(');
    render_parameters(&mut source, &searcher_class.method.parameters);
    source.push_str(")\n    {\n");
    for statement in &searcher_class.method.statements {
        source.push_str("        ");
        source.push_str(statement);
        source.push('\n');
    }
    source.push_str("    }\n}\n");
    Ok(source)
}

fn render_parameters(source: &mut String, parameters: &[CSharpParameterSyntax]) {
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            source.push_str(", ");
        }
        source.push_str(&parameter.type_keyword);
        source.push(' ');
        source.push_str(&parameter.name);
    }
}

fn parameter_pairs(parameters: &[CSharpParameterSyntax]) -> Vec<(&str, &str)> {
    parameters
        .iter()
        .map(|parameter| (parameter.type_keyword.as_str(), parameter.name.as_str()))
        .collect()
}

fn validate_version_one_source_file_syntax(
    syntax: &CSharpCompilationUnitVersionOneSourceFileSyntax,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "FileScopedNamespaceDeclarationSyntax.Name",
        &syntax.namespace,
        "Events",
    )?;
    let args_class = &syntax.file_found_args_class;
    require_string(
        "ClassDeclarationSyntax.Identifier",
        &args_class.name,
        "FileFoundArgs",
    )?;
    require_string_vec(
        "BaseListSyntax.Types",
        &args_class.base_types,
        &["EventArgs"],
    )?;
    require_string(
        "PropertyDeclarationSyntax.Type",
        &args_class.property.type_keyword,
        "string",
    )?;
    require_string(
        "PropertyDeclarationSyntax.Identifier",
        &args_class.property.name,
        "FoundFile",
    )?;
    require_string(
        "AccessorDeclarationSyntax.Keyword",
        &args_class.property.accessor_keyword,
        "get",
    )?;
    require_string(
        "ConstructorDeclarationSyntax.Identifier",
        &args_class.constructor.name,
        "FileFoundArgs",
    )?;
    let constructor_parameters = parameter_pairs(&args_class.constructor.parameters);
    require_pair_slice(
        "ConstructorDeclarationSyntax.ParameterList",
        &constructor_parameters,
        &[("string", "fileName")],
    )?;
    require_string(
        "ArrowExpressionClauseSyntax.Assignment.Left",
        &args_class.constructor.assignment.left,
        "FoundFile",
    )?;
    require_string(
        "ArrowExpressionClauseSyntax.Assignment.Right",
        &args_class.constructor.assignment.right,
        "fileName",
    )?;

    let searcher_class = &syntax.file_searcher_class;
    require_string(
        "ClassDeclarationSyntax.Identifier",
        &searcher_class.name,
        "FileSearcher",
    )?;
    let event = single_variable_declarator(
        "EventFieldDeclarationSyntax.Declaration",
        &searcher_class.event.declaration,
    )?;
    require_string(
        "EventFieldDeclarationSyntax.Declaration.Type",
        &searcher_class.event.declaration.type_name,
        "EventHandler<FileFoundArgs>?",
    )?;
    require_string(
        "EventFieldDeclarationSyntax.Variable",
        &event.name,
        "FileFound",
    )?;
    if event.initializer.is_some() {
        return Err(render_error(
            "EventFieldDeclarationSyntax.Initializer",
            "VersionOne.cs FileFound event should not have an initializer",
        ));
    }
    require_string(
        "MethodDeclarationSyntax.ReturnType",
        &searcher_class.method.return_type,
        "void",
    )?;
    require_string(
        "MethodDeclarationSyntax.Identifier",
        &searcher_class.method.name,
        "Search",
    )?;
    let method_parameters = parameter_pairs(&searcher_class.method.parameters);
    require_pair_slice(
        "ParameterListSyntax.Parameters",
        &method_parameters,
        &[("string", "directory"), ("string", "searchPattern")],
    )?;
    require_string_vec(
        "BlockSyntax.Statements",
        &searcher_class.method.statements,
        &[
            "foreach (var file in Directory.EnumerateFiles(directory, searchPattern))\n        {\n            FileFound?.Invoke(this, new FileFoundArgs(file));\n        }",
        ],
    )
}

fn single_variable_declarator<'a>(
    path: &'static str,
    declaration: &'a CSharpVariableDeclarationSyntax,
) -> Result<&'a CSharpVariableDeclaratorSyntax, SyntaxBackendBoundaryError> {
    if declaration.variables.len() != 1 {
        return Err(render_error(
            path,
            format!(
                "C# renderer expects exactly one variable declarator, got {}",
                declaration.variables.len()
            ),
        ));
    }
    Ok(&declaration.variables[0])
}
