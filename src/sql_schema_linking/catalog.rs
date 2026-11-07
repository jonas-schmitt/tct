//! Spider-style schema catalog loader for the SQL schema-linking frontend (T1).
//!
//! Loads the pinned in-repo `tables.json`, verifies content SHA-256 against
//! `fixture_pin.json`, drops the Spider `[-1, "*"]` pseudo-column, and resolves
//! foreign-key column indices to named table/column pairs.
//!
//! Fail-closed contract (T1 audit repair):
//! - Public load paths revalidate `FixturePin` (callers cannot skip validation
//!   by constructing a public-field struct).
//! - Both pin digests must be 64-digit hex (case-agnostic; compared lowercase).
//! - Pin relative paths reject absolute paths and parent-directory traversal.
//! - Primary-key indices must refer to real columns (not `*`, not OOB);
//!   duplicate PK indices are rejected.
//! - `load_fixture_catalog` / `load_fixture_bundle` require
//!   `db_id == SQL_SCHEMA_LINKING_FIXTURE_DB_ID`.
//! - Gold-query digest is part of the pin; `load_fixture_bundle` verifies it.

use crate::provenance::{file_provenance, sha256_hex};
use serde::Deserialize;
use std::error::Error;
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// Claim boundary carried by catalog-facing surfaces.
pub const SQL_SCHEMA_LINKING_CLAIM_BOUNDARY: &str = "compiler_sql_schema_linking_only";
/// Fixture pin artifact type.
pub const SQL_SCHEMA_LINKING_FIXTURE_PIN_ARTIFACT: &str = "sql_schema_linking_fixture_pin_v0";
/// In-repo fixture `db_id`.
pub const SQL_SCHEMA_LINKING_FIXTURE_DB_ID: &str = "tct_fixture_join";
/// Wide-menu capacity synthetic fixture `db_id` (T-D). Not loaded by
/// [`load_fixture_bundle`] so T11/T12 report hashes stay pinned to the join
/// fixture only.
pub const SQL_SCHEMA_LINKING_WIDE_FIXTURE_DB_ID: &str = "tct_fixture_wide_menu";

/// Absolute path to `tests/fixtures/sql_schema_linking/`.
pub fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sql_schema_linking")
}

/// Load the pinned in-repo fixture catalog (`tct_fixture_join`).
///
/// Requires the pin's `db_id` to equal [`SQL_SCHEMA_LINKING_FIXTURE_DB_ID`].
/// Does **not** verify gold-query bytes; use [`load_fixture_bundle`] when gold
/// provenance must be checked alongside the catalog.
pub fn load_fixture_catalog() -> Result<SqlCatalog, SqlCatalogError> {
    let pin = load_in_repo_fixture_pin()?;
    let tables_path = resolve_repo_relative_path(&pin.paths.tables_json)?;
    SqlCatalog::load_pinned_tables_json(&tables_path, &pin)
}

/// Load the full checked-in fixture bundle: pin + catalog + gold-query digest.
///
/// Verifies both `tables.json` and `gold_queries.json` content digests against
/// the pin. Prefer this over catalog-only load when gold provenance matters.
///
/// Always loads [`SQL_SCHEMA_LINKING_FIXTURE_DB_ID`] only — never the wide
/// capacity fixture (`tct_fixture_wide_menu`).
pub fn load_fixture_bundle() -> Result<SqlFixtureBundle, SqlCatalogError> {
    let pin = load_in_repo_fixture_pin()?;
    let tables_path = resolve_repo_relative_path(&pin.paths.tables_json)?;
    let catalog = SqlCatalog::load_pinned_tables_json(&tables_path, &pin)?;
    let gold_path = resolve_repo_relative_path(&pin.paths.gold_queries_json)?;
    let gold_bytes = std::fs::read(&gold_path).map_err(|error| {
        SqlCatalogError::new(
            "read",
            Some(gold_path.display().to_string()),
            format!("failed to read gold_queries.json: {error}"),
        )
    })?;
    let gold_sha = verify_gold_queries_bytes(&gold_bytes, &pin, Some(gold_path.as_path()))?;
    Ok(SqlFixtureBundle {
        pin,
        catalog,
        gold_queries_json_sha256: gold_sha,
        gold_queries_json_path: gold_path.display().to_string(),
        gold_queries_bytes: gold_bytes,
    })
}

/// Load the T-D wide synthetic catalog (`tct_fixture_wide_menu`).
///
/// Separate pin under `wide_menu_capacity/`; intentionally **not** wired into
/// [`load_fixture_bundle`] so checked-in T11/T12 report digests stay stable.
pub fn load_wide_menu_capacity_catalog() -> Result<SqlCatalog, SqlCatalogError> {
    let pin_path = fixture_dir().join("wide_menu_capacity/fixture_pin.json");
    let pin = FixturePin::load(&pin_path)?;
    if pin.db_id != SQL_SCHEMA_LINKING_WIDE_FIXTURE_DB_ID {
        return Err(SqlCatalogError::new(
            "validate",
            Some(pin_path.display().to_string()),
            format!(
                "wide fixture pin requires db_id {SQL_SCHEMA_LINKING_WIDE_FIXTURE_DB_ID}, got {}",
                pin.db_id
            ),
        ));
    }
    let tables_path = resolve_repo_relative_path(&pin.paths.tables_json)?;
    SqlCatalog::load_pinned_tables_json(&tables_path, &pin)
}

fn load_in_repo_fixture_pin() -> Result<FixturePin, SqlCatalogError> {
    let pin_path = fixture_dir().join("fixture_pin.json");
    let pin = FixturePin::load(&pin_path)?;
    require_in_repo_fixture_db_id(&pin, Some(pin_path.as_path()))?;
    Ok(pin)
}

/// `load_fixture_catalog` / `load_fixture_bundle` only accept the checked-in fixture db.
fn require_in_repo_fixture_db_id(
    pin: &FixturePin,
    path: Option<&Path>,
) -> Result<(), SqlCatalogError> {
    if pin.db_id != SQL_SCHEMA_LINKING_FIXTURE_DB_ID {
        return Err(SqlCatalogError::new(
            "validate",
            path.map(|p| p.display().to_string()),
            format!(
                "load_fixture_* requires db_id {SQL_SCHEMA_LINKING_FIXTURE_DB_ID}, got {}",
                pin.db_id
            ),
        ));
    }
    Ok(())
}

/// Resolve a repo-relative fixture path under `CARGO_MANIFEST_DIR` after
/// rejecting absolute paths and parent-directory traversal.
pub fn resolve_repo_relative_path(relative: &str) -> Result<PathBuf, SqlCatalogError> {
    validate_relative_fixture_path(relative, "path")?;
    Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
}

/// Reject absolute paths and any `..` / root / prefix component.
pub fn validate_relative_fixture_path(relative: &str, field: &str) -> Result<(), SqlCatalogError> {
    if relative.is_empty() {
        return Err(SqlCatalogError::new(
            "validate",
            None,
            format!("{field} must be a non-empty repo-relative path"),
        ));
    }
    // Explicit absolute-string guards (also covered by RootDir/Prefix below).
    if relative.starts_with('/') || relative.starts_with('\\') {
        return Err(SqlCatalogError::new(
            "validate",
            None,
            format!("{field} must be relative (absolute path rejected): {relative}"),
        ));
    }
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err(SqlCatalogError::new(
            "validate",
            None,
            format!("{field} must be relative (absolute path rejected): {relative}"),
        ));
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => {
                return Err(SqlCatalogError::new(
                    "validate",
                    None,
                    format!("{field} must not contain parent-directory traversal: {relative}"),
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(SqlCatalogError::new(
                    "validate",
                    None,
                    format!("{field} must be relative (absolute path rejected): {relative}"),
                ));
            }
        }
    }
    Ok(())
}

/// Validate a SHA-256 digest field: exactly 64 hex digits, case-agnostic.
///
/// Returns the lowercase form used for comparison against [`sha256_hex`].
pub fn normalize_sha256_hex(value: &str, field: &str) -> Result<String, SqlCatalogError> {
    if value.len() != 64 {
        return Err(SqlCatalogError::new(
            "validate",
            None,
            format!(
                "{field} must be a 64-digit hex SHA-256 digest (got length {})",
                value.len()
            ),
        ));
    }
    if !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SqlCatalogError::new(
            "validate",
            None,
            format!("{field} must be a 64-digit hex SHA-256 digest (non-hex character)"),
        ));
    }
    Ok(value.to_ascii_lowercase())
}

/// Verify gold-query bytes against a revalidated pin digest.
pub fn verify_gold_queries_bytes(
    bytes: &[u8],
    pin: &FixturePin,
    path: Option<&Path>,
) -> Result<String, SqlCatalogError> {
    pin.revalidate()?;
    let actual = sha256_hex(bytes);
    let expected = normalize_sha256_hex(&pin.gold_queries_json_sha256, "gold_queries_json_sha256")?;
    if actual != expected {
        return Err(SqlCatalogError::new(
            "pin",
            path.map(|p| p.display().to_string()),
            format!("gold_queries.json sha256 mismatch: expected {expected}, got {actual}"),
        ));
    }
    Ok(actual)
}

/// Checked-in fixture pin: paths + content digests for catalog and gold queries.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FixturePin {
    pub artifact_type: String,
    pub db_id: String,
    pub paths: FixturePinPaths,
    pub tables_json_sha256: String,
    pub gold_queries_json_sha256: String,
    pub version: u32,
}

/// Relative paths recorded in the fixture pin.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FixturePinPaths {
    pub tables_json: String,
    pub gold_queries_json: String,
}

/// Pin + catalog + gold-query provenance for the checked-in fixture bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlFixtureBundle {
    pub pin: FixturePin,
    pub catalog: SqlCatalog,
    pub gold_queries_json_sha256: String,
    pub gold_queries_json_path: String,
    pub gold_queries_bytes: Vec<u8>,
}

impl FixturePin {
    /// Load and validate a fixture pin JSON file.
    pub fn load(path: &Path) -> Result<Self, SqlCatalogError> {
        let bytes = std::fs::read(path).map_err(|error| {
            SqlCatalogError::new(
                "read",
                Some(path.display().to_string()),
                format!("failed to read fixture pin: {error}"),
            )
        })?;
        let pin: Self = serde_json::from_slice(&bytes).map_err(|error| {
            SqlCatalogError::new(
                "parse",
                Some(path.display().to_string()),
                format!("failed to parse fixture pin: {error}"),
            )
        })?;
        // Revalidate without mutating; digests are compared case-insensitively
        // at content-check time via normalize_sha256_hex.
        pin.revalidate_at(Some(path))?;
        Ok(pin)
    }

    /// Revalidate this pin (used by every public load path).
    ///
    /// Callers can construct a public-field `FixturePin`; load paths must not
    /// trust that construction without revalidation.
    pub fn revalidate(&self) -> Result<(), SqlCatalogError> {
        self.revalidate_at(None)
    }

    fn revalidate_at(&self, path: Option<&Path>) -> Result<(), SqlCatalogError> {
        let ctx = path.map(|p| p.display().to_string());
        if self.artifact_type != SQL_SCHEMA_LINKING_FIXTURE_PIN_ARTIFACT {
            return Err(SqlCatalogError::new(
                "validate",
                ctx,
                format!(
                    "expected artifact_type {SQL_SCHEMA_LINKING_FIXTURE_PIN_ARTIFACT}, got {}",
                    self.artifact_type
                ),
            ));
        }
        if self.version != 1 {
            return Err(SqlCatalogError::new(
                "validate",
                ctx,
                format!("expected fixture pin version 1, got {}", self.version),
            ));
        }
        if self.db_id.is_empty() {
            return Err(SqlCatalogError::new(
                "validate",
                ctx,
                "fixture pin db_id must be non-empty",
            ));
        }
        normalize_sha256_hex(&self.tables_json_sha256, "tables_json_sha256").map_err(
            |mut err| {
                err.path = ctx.clone();
                err
            },
        )?;
        normalize_sha256_hex(&self.gold_queries_json_sha256, "gold_queries_json_sha256").map_err(
            |mut err| {
                err.path = ctx.clone();
                err
            },
        )?;
        validate_relative_fixture_path(&self.paths.tables_json, "paths.tables_json").map_err(
            |mut err| {
                err.path = ctx.clone();
                err
            },
        )?;
        validate_relative_fixture_path(&self.paths.gold_queries_json, "paths.gold_queries_json")
            .map_err(|mut err| {
                err.path = ctx;
                err
            })?;
        Ok(())
    }
}

/// One real column in a catalog table (Spider `[-1, "*"]` is never retained).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlColumn {
    /// Catalog-local index into the raw Spider `column_names` array (includes `*`).
    pub raw_column_index: usize,
    pub name: String,
    pub name_original: String,
    pub column_type: String,
    pub is_primary_key: bool,
}

/// One table in catalog order (menu table ordinal = index in `SqlCatalog::tables`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlTable {
    /// Zero-based table index matching Spider `column_names[*][0]`.
    pub table_index: usize,
    pub name: String,
    pub name_original: String,
    pub columns: Vec<SqlColumn>,
}

/// Resolved foreign key: from column → to column, both real catalog columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignKeyRef {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
    pub from_raw_column_index: usize,
    pub to_raw_column_index: usize,
}

/// Loaded, pin-verified Spider-style database catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlCatalog {
    pub db_id: String,
    pub tables: Vec<SqlTable>,
    pub foreign_keys: Vec<ForeignKeyRef>,
    pub tables_json_sha256: String,
    pub tables_json_path: String,
}

impl SqlCatalog {
    /// Load `tables.json` and verify its content digest against the fixture pin.
    pub fn load_pinned_tables_json(
        tables_path: &Path,
        pin: &FixturePin,
    ) -> Result<Self, SqlCatalogError> {
        pin.revalidate()?;
        let bytes = std::fs::read(tables_path).map_err(|error| {
            SqlCatalogError::new(
                "read",
                Some(tables_path.display().to_string()),
                format!("failed to read tables.json: {error}"),
            )
        })?;
        Self::from_tables_json_bytes(bytes.as_slice(), tables_path, pin)
    }

    /// Parse and pin-check in-memory tables JSON bytes (used by tests for mutation).
    pub fn from_tables_json_bytes(
        bytes: &[u8],
        tables_path: &Path,
        pin: &FixturePin,
    ) -> Result<Self, SqlCatalogError> {
        pin.revalidate()?;
        let actual_sha = sha256_hex(bytes);
        let expected_sha = normalize_sha256_hex(&pin.tables_json_sha256, "tables_json_sha256")?;
        if actual_sha != expected_sha {
            return Err(SqlCatalogError::new(
                "pin",
                Some(tables_path.display().to_string()),
                format!("tables.json sha256 mismatch: expected {expected_sha}, got {actual_sha}"),
            ));
        }
        // Record path provenance when the file exists; tests may pass a synthetic path.
        let _ = file_provenance(tables_path);
        Self::parse_tables_json(bytes, pin.db_id.clone(), actual_sha, tables_path)
    }

    /// Parse Spider-style tables JSON without pin verification.
    ///
    /// Used for host-mounted or reconstructed multi-db `tables.json` files
    /// (e.g. Spider 1.0-scale catalogs). Still fail-closed on schema shape.
    /// Not a claim-bearing official pin path — callers must label provenance.
    pub fn load_unpinned_tables_json(
        tables_path: &Path,
        db_id: &str,
    ) -> Result<Self, SqlCatalogError> {
        let bytes = std::fs::read(tables_path).map_err(|error| {
            SqlCatalogError::new(
                "read",
                Some(tables_path.display().to_string()),
                format!("failed to read tables.json: {error}"),
            )
        })?;
        let sha = sha256_hex(&bytes);
        Self::parse_tables_json(&bytes, db_id.to_string(), sha, tables_path)
    }

    /// Parse Spider-style tables JSON bytes without pin verification (tests/helpers).
    pub fn parse_tables_json_unpinned(
        bytes: &[u8],
        expected_db_id: &str,
    ) -> Result<Self, SqlCatalogError> {
        let sha = sha256_hex(bytes);
        Self::parse_tables_json(bytes, expected_db_id.to_string(), sha, Path::new("<bytes>"))
    }

    /// List `db_id` values present in a Spider-style multi-db tables.json.
    pub fn list_db_ids(tables_path: &Path) -> Result<Vec<String>, SqlCatalogError> {
        let bytes = std::fs::read(tables_path).map_err(|error| {
            SqlCatalogError::new(
                "read",
                Some(tables_path.display().to_string()),
                format!("failed to read tables.json: {error}"),
            )
        })?;
        let databases: Vec<RawSpiderDatabase> =
            serde_json::from_slice(&bytes).map_err(|error| {
                SqlCatalogError::new(
                    "parse",
                    Some(tables_path.display().to_string()),
                    format!("failed to parse tables.json: {error}"),
                )
            })?;
        Ok(databases.into_iter().map(|db| db.db_id).collect())
    }

    fn parse_tables_json(
        bytes: &[u8],
        expected_db_id: String,
        tables_json_sha256: String,
        tables_path: &Path,
    ) -> Result<Self, SqlCatalogError> {
        let databases: Vec<RawSpiderDatabase> = serde_json::from_slice(bytes).map_err(|error| {
            SqlCatalogError::new(
                "parse",
                Some(tables_path.display().to_string()),
                format!("failed to parse tables.json: {error}"),
            )
        })?;
        if databases.is_empty() {
            return Err(SqlCatalogError::new(
                "validate",
                Some(tables_path.display().to_string()),
                "tables.json must contain at least one database entry",
            ));
        }
        let raw = databases
            .into_iter()
            .find(|db| db.db_id == expected_db_id)
            .ok_or_else(|| {
                SqlCatalogError::new(
                    "validate",
                    Some(tables_path.display().to_string()),
                    format!("tables.json has no entry for db_id {expected_db_id}"),
                )
            })?;
        Self::from_raw(raw, tables_json_sha256, tables_path.display().to_string())
    }

    fn from_raw(
        raw: RawSpiderDatabase,
        tables_json_sha256: String,
        tables_json_path: String,
    ) -> Result<Self, SqlCatalogError> {
        if raw.table_names.is_empty() {
            return Err(SqlCatalogError::new(
                "validate",
                Some(tables_json_path.clone()),
                "database must declare at least one table",
            ));
        }
        if raw.table_names.len() != raw.table_names_original.len() {
            return Err(SqlCatalogError::new(
                "validate",
                Some(tables_json_path.clone()),
                "table_names and table_names_original length mismatch",
            ));
        }
        if raw.column_names.len() != raw.column_names_original.len() {
            return Err(SqlCatalogError::new(
                "validate",
                Some(tables_json_path.clone()),
                "column_names and column_names_original length mismatch",
            ));
        }
        if raw.column_names.len() != raw.column_types.len() {
            return Err(SqlCatalogError::new(
                "validate",
                Some(tables_json_path.clone()),
                "column_names and column_types length mismatch",
            ));
        }

        // Fail closed on primary-key indices before building columns.
        // Decision: reject out-of-range, PK-on-`*`, and duplicate indices.
        let mut pk_set: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        for (pk_i, &pk_idx) in raw.primary_keys.iter().enumerate() {
            if pk_idx >= raw.column_names.len() {
                return Err(SqlCatalogError::new(
                    "validate",
                    Some(tables_json_path.clone()),
                    format!(
                        "primary_keys[{pk_i}] raw column index {pk_idx} is out of range for {} columns",
                        raw.column_names.len()
                    ),
                ));
            }
            if raw.column_names[pk_idx].0 == -1 {
                return Err(SqlCatalogError::new(
                    "validate",
                    Some(tables_json_path.clone()),
                    format!(
                        "primary_keys[{pk_i}] raw column index {pk_idx} refers to the \"*\" pseudo-column"
                    ),
                ));
            }
            if !pk_set.insert(pk_idx) {
                return Err(SqlCatalogError::new(
                    "validate",
                    Some(tables_json_path.clone()),
                    format!("primary_keys[{pk_i}] duplicates raw column index {pk_idx}"),
                ));
            }
        }

        let table_count = raw.table_names.len();
        let mut tables: Vec<SqlTable> = raw
            .table_names
            .iter()
            .zip(raw.table_names_original.iter())
            .enumerate()
            .map(|(table_index, (name, name_original))| SqlTable {
                table_index,
                name: name.clone(),
                name_original: name_original.clone(),
                columns: Vec::new(),
            })
            .collect();

        for (raw_column_index, ((table_and_name, original), column_type)) in raw
            .column_names
            .iter()
            .zip(raw.column_names_original.iter())
            .zip(raw.column_types.iter())
            .enumerate()
        {
            let table_index = table_and_name.0;
            let name = &table_and_name.1;
            let name_original = &original.1;

            // Spider pseudo-column: table index -1, name "*". Drop it; star is structural.
            if table_index == -1 {
                if name != "*" {
                    return Err(SqlCatalogError::new(
                        "validate",
                        Some(tables_json_path.clone()),
                        format!(
                            "raw column {raw_column_index} has table_index -1 but name {name:?} (expected \"*\")"
                        ),
                    ));
                }
                continue;
            }
            if table_index < 0 || table_index as usize >= table_count {
                return Err(SqlCatalogError::new(
                    "validate",
                    Some(tables_json_path.clone()),
                    format!(
                        "raw column {raw_column_index} table_index {table_index} out of range for {table_count} tables"
                    ),
                ));
            }
            if original.0 != table_index {
                return Err(SqlCatalogError::new(
                    "validate",
                    Some(tables_json_path.clone()),
                    format!(
                        "raw column {raw_column_index} original table_index {} disagrees with {}",
                        original.0, table_index
                    ),
                ));
            }

            tables[table_index as usize].columns.push(SqlColumn {
                raw_column_index,
                name: name.clone(),
                name_original: name_original.clone(),
                column_type: column_type.clone(),
                is_primary_key: pk_set.contains(&raw_column_index),
            });
        }

        // Build raw-index → (table_name, column_name) for FK resolution; skip "*".
        let mut raw_index_to_col: Vec<Option<(String, String)>> =
            vec![None; raw.column_names.len()];
        for table in &tables {
            for column in &table.columns {
                raw_index_to_col[column.raw_column_index] =
                    Some((table.name.clone(), column.name.clone()));
            }
        }

        let mut foreign_keys = Vec::with_capacity(raw.foreign_keys.len());
        for (fk_i, pair) in raw.foreign_keys.iter().enumerate() {
            if pair.len() != 2 {
                return Err(SqlCatalogError::new(
                    "validate",
                    Some(tables_json_path.clone()),
                    format!("foreign_keys[{fk_i}] must be a [from, to] pair"),
                ));
            }
            let from_idx = pair[0];
            let to_idx = pair[1];
            let from = raw_index_to_col
                .get(from_idx)
                .and_then(|v| v.as_ref())
                .ok_or_else(|| {
                    SqlCatalogError::new(
                        "fk",
                        Some(tables_json_path.clone()),
                        format!(
                            "foreign_keys[{fk_i}] from raw column index {from_idx} is missing or is the \"*\" pseudo-column"
                        ),
                    )
                })?;
            let to = raw_index_to_col
                .get(to_idx)
                .and_then(|v| v.as_ref())
                .ok_or_else(|| {
                    SqlCatalogError::new(
                        "fk",
                        Some(tables_json_path.clone()),
                        format!(
                            "foreign_keys[{fk_i}] to raw column index {to_idx} is missing or is the \"*\" pseudo-column"
                        ),
                    )
                })?;
            foreign_keys.push(ForeignKeyRef {
                from_table: from.0.clone(),
                from_column: from.1.clone(),
                to_table: to.0.clone(),
                to_column: to.1.clone(),
                from_raw_column_index: from_idx,
                to_raw_column_index: to_idx,
            });
        }

        Ok(Self {
            db_id: raw.db_id,
            tables,
            foreign_keys,
            tables_json_sha256,
            tables_json_path,
        })
    }

    /// Look up a table by catalog name.
    pub fn table(&self, name: &str) -> Option<&SqlTable> {
        self.tables.iter().find(|table| table.name == name)
    }

    /// Number of real columns across all tables (excludes Spider `*`).
    pub fn real_column_count(&self) -> usize {
        self.tables.iter().map(|table| table.columns.len()).sum()
    }
}

/// Phase-tagged catalog load / pin / FK error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlCatalogError {
    pub phase: &'static str,
    pub path: Option<String>,
    pub message: String,
}

impl SqlCatalogError {
    pub fn new(phase: &'static str, path: Option<String>, message: impl Into<String>) -> Self {
        Self {
            phase,
            path,
            message: message.into(),
        }
    }
}

impl fmt::Display for SqlCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SQL catalog {} error", self.phase)?;
        if let Some(path) = &self.path {
            write!(formatter, " at {path}")?;
        }
        write!(formatter, ": {}", self.message)
    }
}

impl Error for SqlCatalogError {}

/// Spider `tables.json` database entry (raw wire form).
#[derive(Debug, Deserialize)]
struct RawSpiderDatabase {
    db_id: String,
    table_names: Vec<String>,
    table_names_original: Vec<String>,
    column_names: Vec<(i64, String)>,
    column_names_original: Vec<(i64, String)>,
    column_types: Vec<String>,
    primary_keys: Vec<usize>,
    foreign_keys: Vec<Vec<usize>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_pin_from_disk() -> FixturePin {
        FixturePin::load(&fixture_dir().join("fixture_pin.json")).expect("pin load")
    }

    fn pin_with(
        tables_sha: &str,
        gold_sha: &str,
        tables_path: &str,
        gold_path: &str,
        db_id: &str,
    ) -> FixturePin {
        FixturePin {
            artifact_type: SQL_SCHEMA_LINKING_FIXTURE_PIN_ARTIFACT.to_string(),
            db_id: db_id.to_string(),
            paths: FixturePinPaths {
                tables_json: tables_path.to_string(),
                gold_queries_json: gold_path.to_string(),
            },
            tables_json_sha256: tables_sha.to_string(),
            gold_queries_json_sha256: gold_sha.to_string(),
            version: 1,
        }
    }

    #[test]
    fn fixture_catalog_loads_three_tables_with_real_columns_and_fks() {
        let catalog = load_fixture_catalog().expect("fixture catalog should load");

        assert_eq!(catalog.db_id, SQL_SCHEMA_LINKING_FIXTURE_DB_ID);
        assert_eq!(
            catalog.tables.len(),
            3,
            "expected orders, customers, products"
        );
        assert_eq!(
            catalog
                .tables
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>(),
            vec!["orders", "customers", "products"]
        );

        let orders = catalog.table("orders").expect("orders table");
        assert_eq!(
            orders.columns.len(),
            4,
            "orders must drop the Spider \"*\" pseudo-column and keep 4 real columns"
        );
        assert_eq!(
            orders
                .columns
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            vec!["order_id", "customer_id", "product_id", "amount"]
        );
        assert!(
            orders
                .columns
                .iter()
                .any(|c| c.name == "order_id" && c.is_primary_key),
            "order_id should be primary key"
        );

        let customers = catalog.table("customers").expect("customers table");
        assert_eq!(customers.columns.len(), 3);
        let products = catalog.table("products").expect("products table");
        assert_eq!(products.columns.len(), 3);
        assert_eq!(catalog.real_column_count(), 10);

        // No retained column may be named "*".
        for table in &catalog.tables {
            for column in &table.columns {
                assert_ne!(column.name, "*");
            }
        }

        assert_eq!(catalog.foreign_keys.len(), 2);
        assert!(
            catalog.foreign_keys.iter().any(|fk| {
                fk.from_table == "orders"
                    && fk.from_column == "customer_id"
                    && fk.to_table == "customers"
                    && fk.to_column == "customer_id"
            }),
            "orders.customer_id → customers.customer_id FK missing: {:?}",
            catalog.foreign_keys
        );
        assert!(
            catalog.foreign_keys.iter().any(|fk| {
                fk.from_table == "orders"
                    && fk.from_column == "product_id"
                    && fk.to_table == "products"
                    && fk.to_column == "product_id"
            }),
            "orders.product_id → products.product_id FK missing: {:?}",
            catalog.foreign_keys
        );

        // Pin digest must match the on-disk file.
        let expected = crate::provenance::file_provenance(fixture_dir().join("tables.json"))
            .expect("tables.json provenance")
            .sha256;
        assert_eq!(catalog.tables_json_sha256, expected);
    }

    #[test]
    fn fixture_bundle_verifies_tables_and_gold_digests() {
        let bundle = load_fixture_bundle().expect("fixture bundle should load");
        assert_eq!(bundle.catalog.db_id, SQL_SCHEMA_LINKING_FIXTURE_DB_ID);
        assert_eq!(bundle.pin.db_id, SQL_SCHEMA_LINKING_FIXTURE_DB_ID);
        let expected_gold =
            crate::provenance::file_provenance(fixture_dir().join("gold_queries.json"))
                .expect("gold provenance")
                .sha256;
        assert_eq!(bundle.gold_queries_json_sha256, expected_gold);
        assert!(!bundle.gold_queries_bytes.is_empty());
    }

    #[test]
    fn mutated_tables_json_bytes_fail_the_pin() {
        let pin = valid_pin_from_disk();
        let tables_path = resolve_repo_relative_path(&pin.paths.tables_json).expect("path");
        let mut bytes = std::fs::read(&tables_path).expect("read tables.json");
        assert!(!bytes.is_empty());
        // Flip one byte so the digest cannot match the pin.
        bytes[0] ^= 0xff;

        let err = SqlCatalog::from_tables_json_bytes(&bytes, &tables_path, &pin)
            .expect_err("mutated bytes must fail pin verification");
        assert_eq!(err.phase, "pin");
        assert!(
            err.message.contains("sha256 mismatch"),
            "unexpected pin error: {}",
            err.message
        );
    }

    #[test]
    fn mutated_gold_queries_bytes_fail_the_pin() {
        let pin = valid_pin_from_disk();
        let gold_path = resolve_repo_relative_path(&pin.paths.gold_queries_json).expect("path");
        let mut bytes = std::fs::read(&gold_path).expect("read gold");
        assert!(!bytes.is_empty());
        bytes[0] ^= 0xff;

        let err = verify_gold_queries_bytes(&bytes, &pin, Some(gold_path.as_path()))
            .expect_err("mutated gold must fail pin");
        assert_eq!(err.phase, "pin");
        assert!(
            err.message.contains("gold_queries.json sha256 mismatch"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn fixture_pin_rejects_wrong_artifact_type() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("bad_pin.json");
        std::fs::write(
            &path,
            r#"{
              "artifact_type": "not_the_sql_pin",
              "db_id": "x",
              "paths": {
                "tables_json": "t.json",
                "gold_queries_json": "g.json"
              },
              "tables_json_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
              "gold_queries_json_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
              "version": 1
            }"#,
        )
        .unwrap();
        let err = FixturePin::load(&path).expect_err("wrong artifact_type");
        assert_eq!(err.phase, "validate");
    }

    #[test]
    fn public_load_path_revalidates_caller_constructed_pin() {
        // Public-field construction with a bad digest length must fail revalidation.
        let pin = pin_with(
            "not-a-digest",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "tests/fixtures/sql_schema_linking/tables.json",
            "tests/fixtures/sql_schema_linking/gold_queries.json",
            SQL_SCHEMA_LINKING_FIXTURE_DB_ID,
        );
        let tables_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sql_schema_linking/tables.json");
        let bytes = std::fs::read(&tables_path).unwrap();
        let err = SqlCatalog::from_tables_json_bytes(&bytes, &tables_path, &pin)
            .expect_err("invalid pin must fail revalidation");
        assert_eq!(err.phase, "validate");
        assert!(
            err.message.contains("tables_json_sha256"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn pin_accepts_uppercase_hex_digests_case_agnostically() {
        let mut pin = valid_pin_from_disk();
        pin.tables_json_sha256 = pin.tables_json_sha256.to_ascii_uppercase();
        pin.gold_queries_json_sha256 = pin.gold_queries_json_sha256.to_ascii_uppercase();
        pin.revalidate().expect("uppercase hex must validate");
        let tables_path = resolve_repo_relative_path(&pin.paths.tables_json).unwrap();
        let bytes = std::fs::read(&tables_path).unwrap();
        SqlCatalog::from_tables_json_bytes(&bytes, &tables_path, &pin)
            .expect("uppercase pin digests must match content");
        let gold = std::fs::read(resolve_repo_relative_path(&pin.paths.gold_queries_json).unwrap())
            .unwrap();
        verify_gold_queries_bytes(&gold, &pin, None).expect("uppercase gold digest");
    }

    #[test]
    fn pin_rejects_non_hex_digest() {
        let pin = pin_with(
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "tests/fixtures/sql_schema_linking/tables.json",
            "tests/fixtures/sql_schema_linking/gold_queries.json",
            SQL_SCHEMA_LINKING_FIXTURE_DB_ID,
        );
        let err = pin.revalidate().expect_err("non-hex digest");
        assert_eq!(err.phase, "validate");
        assert!(
            err.message.contains("non-hex") || err.message.contains("hex"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn pin_rejects_absolute_and_parent_traversal_paths() {
        let abs = pin_with(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "/etc/passwd",
            "tests/fixtures/sql_schema_linking/gold_queries.json",
            SQL_SCHEMA_LINKING_FIXTURE_DB_ID,
        );
        let err = abs.revalidate().expect_err("absolute path");
        assert!(
            err.message.contains("absolute") || err.message.contains("relative"),
            "unexpected: {}",
            err.message
        );

        let parent = pin_with(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "tests/../secrets/tables.json",
            "tests/fixtures/sql_schema_linking/gold_queries.json",
            SQL_SCHEMA_LINKING_FIXTURE_DB_ID,
        );
        let err = parent.revalidate().expect_err("parent traversal");
        assert!(
            err.message.contains("parent-directory"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn load_fixture_paths_reject_wrong_fixture_db_id() {
        let pin = pin_with(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "tests/fixtures/sql_schema_linking/tables.json",
            "tests/fixtures/sql_schema_linking/gold_queries.json",
            "not_the_fixture_db",
        );
        let err = require_in_repo_fixture_db_id(&pin, None).expect_err("wrong db_id");
        assert_eq!(err.phase, "validate");
        assert!(
            err.message.contains("load_fixture_* requires db_id"),
            "unexpected: {}",
            err.message
        );
        // Live path still succeeds with the checked-in pin.
        assert_eq!(
            load_fixture_catalog().unwrap().db_id,
            SQL_SCHEMA_LINKING_FIXTURE_DB_ID
        );
    }

    #[test]
    fn primary_key_out_of_range_is_rejected() {
        let mut raw: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture_dir().join("tables.json")).unwrap())
                .unwrap();
        raw[0]["primary_keys"] = serde_json::json!([999]);
        let bytes = serde_json::to_vec(&raw).unwrap();
        let err = SqlCatalog::parse_tables_json_unpinned(&bytes, SQL_SCHEMA_LINKING_FIXTURE_DB_ID)
            .expect_err("OOB PK");
        assert_eq!(err.phase, "validate");
        assert!(
            err.message.contains("out of range"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn primary_key_on_star_pseudo_column_is_rejected() {
        let mut raw: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture_dir().join("tables.json")).unwrap())
                .unwrap();
        // Index 0 is the Spider [-1, "*"] pseudo-column.
        raw[0]["primary_keys"] = serde_json::json!([0]);
        let bytes = serde_json::to_vec(&raw).unwrap();
        let err = SqlCatalog::parse_tables_json_unpinned(&bytes, SQL_SCHEMA_LINKING_FIXTURE_DB_ID)
            .expect_err("PK on *");
        assert_eq!(err.phase, "validate");
        assert!(
            err.message.contains("pseudo-column") || err.message.contains("\"*\""),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn duplicate_primary_key_indices_are_rejected() {
        let mut raw: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture_dir().join("tables.json")).unwrap())
                .unwrap();
        // Real column index 1 (order_id) listed twice.
        raw[0]["primary_keys"] = serde_json::json!([1, 1]);
        let bytes = serde_json::to_vec(&raw).unwrap();
        let err = SqlCatalog::parse_tables_json_unpinned(&bytes, SQL_SCHEMA_LINKING_FIXTURE_DB_ID)
            .expect_err("duplicate PK");
        assert_eq!(err.phase, "validate");
        assert!(
            err.message.contains("duplicates"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn resolve_repo_relative_path_rejects_unsafe_paths() {
        let err = resolve_repo_relative_path("/tmp/x").expect_err("absolute");
        assert_eq!(err.phase, "validate");
        let err = resolve_repo_relative_path("a/../b").expect_err("parent");
        assert!(err.message.contains("parent-directory"));
    }
}
