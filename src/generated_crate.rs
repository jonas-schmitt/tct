//! Shared Cargo.toml rendering for generated and temporary TCT crates.

mod env;
mod source_dir;

#[cfg(test)]
pub(crate) use env::generated_cargo_target_dir;
pub use env::{
    configure_generated_cargo_command, copy_workspace_cargo_lock, generated_rustup_toolchain,
};
pub use source_dir::tct_source_dir;

pub const GENERATED_RUST_VERSION: &str = "1.95";
pub const GENERATED_EDITION: &str = "2024";
pub const GENERATED_CARGO_RUSTFLAGS: &str = "--cap-lints=allow";
pub const GENERATED_EXTERNAL_BUILD_ENV: &str = "TCT_EXTERNAL_BUILD";

const SERDE_VERSION: &str = "=1.0.228";
const SERDE_JSON_VERSION: &str = "=1.0.149";
const PYO3_VERSION: &str = "=0.29.3";
const RUSTC_HASH_VERSION: &str = "=2.1.2";
const REGEX_VERSION: &str = "=1.12.3";
const LAZY_STATIC_VERSION: &str = "=1.5.0";
const JSONSCHEMA_VERSION: &str = "=0.46.2";
const RAYON_VERSION: &str = "=1.12.0";
const CHRONO_VERSION: &str = "=0.4.44";
const BASE64_VERSION: &str = "=0.22.1";
const SHA2_VERSION: &str = "=0.11.0";

#[derive(Debug, Clone)]
pub struct GeneratedCargoToml {
    package_name: String,
    package_version: String,
    lib: Option<GeneratedLibTarget>,
    bin: Option<GeneratedBinTarget>,
    tct_path: Option<String>,
    build_tct_path: Option<String>,
    preserve_order: bool,
    pyo3_extension: bool,
    rustc_hash: bool,
    regex: bool,
    lazy_static: bool,
    jsonschema: bool,
    rayon: bool,
    chrono: bool,
    base64: bool,
    sha2: bool,
    release_lto: bool,
    profiling_feature: bool,
}

#[derive(Debug, Clone)]
struct GeneratedLibTarget {
    name: Option<String>,
    path: Option<String>,
    crate_types: Vec<String>,
}

#[derive(Debug, Clone)]
struct GeneratedBinTarget {
    name: String,
    path: String,
}

impl GeneratedCargoToml {
    #[must_use]
    pub fn new(package_name: impl Into<String>) -> Self {
        Self {
            package_name: package_name.into(),
            package_version: "0.1.0".to_string(),
            lib: None,
            bin: None,
            tct_path: None,
            build_tct_path: None,
            preserve_order: false,
            pyo3_extension: false,
            rustc_hash: false,
            regex: false,
            lazy_static: false,
            jsonschema: false,
            rayon: false,
            chrono: false,
            base64: false,
            sha2: false,
            release_lto: false,
            profiling_feature: false,
        }
    }

    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.package_version = version.into();
        self
    }

    #[must_use]
    pub fn with_lib(
        mut self,
        name: Option<impl Into<String>>,
        path: Option<impl Into<String>>,
    ) -> Self {
        self.lib = Some(GeneratedLibTarget {
            name: name.map(Into::into),
            path: path.map(Into::into),
            crate_types: Vec::new(),
        });
        self
    }

    #[must_use]
    pub fn with_cdylib(mut self, name: impl Into<String>) -> Self {
        self.lib = Some(GeneratedLibTarget {
            name: Some(name.into()),
            path: None,
            crate_types: vec!["cdylib".to_string()],
        });
        self
    }

    #[must_use]
    pub fn with_bin(mut self, name: impl Into<String>, path: impl Into<String>) -> Self {
        self.bin = Some(GeneratedBinTarget {
            name: name.into(),
            path: path.into(),
        });
        self
    }

    #[must_use]
    pub fn with_tct_path(mut self, path: impl Into<String>) -> Self {
        self.tct_path = Some(path.into());
        self
    }

    #[must_use]
    pub fn with_build_tct_path(mut self, path: impl Into<String>) -> Self {
        self.build_tct_path = Some(path.into());
        self
    }

    #[must_use]
    pub fn with_preserve_order(mut self) -> Self {
        self.preserve_order = true;
        self
    }

    #[must_use]
    pub fn with_pyo3_extension(mut self) -> Self {
        self.pyo3_extension = true;
        self
    }

    #[must_use]
    pub fn with_rustc_hash(mut self) -> Self {
        self.rustc_hash = true;
        self
    }

    #[must_use]
    pub fn with_regex(mut self) -> Self {
        self.regex = true;
        self
    }

    #[must_use]
    pub fn with_lazy_static(mut self) -> Self {
        self.lazy_static = true;
        self
    }

    #[must_use]
    pub fn with_jsonschema(mut self) -> Self {
        self.jsonschema = true;
        self
    }

    #[must_use]
    pub fn with_rayon(mut self) -> Self {
        self.rayon = true;
        self
    }

    #[must_use]
    pub fn with_chrono(mut self) -> Self {
        self.chrono = true;
        self
    }

    #[must_use]
    pub fn with_base64(mut self) -> Self {
        self.base64 = true;
        self
    }

    #[must_use]
    pub fn with_sha2(mut self) -> Self {
        self.sha2 = true;
        self
    }

    #[must_use]
    pub fn with_release_lto(mut self) -> Self {
        self.release_lto = true;
        self
    }

    #[must_use]
    pub fn with_profiling_feature(mut self) -> Self {
        self.profiling_feature = true;
        self
    }

    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("[package]\n");
        push_key_value(&mut out, "name", &self.package_name);
        push_key_value(&mut out, "version", &self.package_version);
        push_key_value(&mut out, "edition", GENERATED_EDITION);
        push_key_value(&mut out, "rust-version", GENERATED_RUST_VERSION);
        push_key_value(&mut out, "license", "Apache-2.0");
        push_key_value(
            &mut out,
            "repository",
            "https://github.com/jonas-schmitt/tct",
        );

        if self.release_lto {
            out.push_str("\n[profile.release]\n");
            out.push_str("lto = true\n");
            out.push_str("codegen-units = 1\n");
        }

        out.push_str("\n[dependencies]\n");
        out.push_str(&format!(
            "serde = {{ version = {serde_version}, features = [\"derive\"] }}\n",
            serde_version = toml_quote(SERDE_VERSION)
        ));
        if self.preserve_order {
            out.push_str(&format!(
                "serde_json = {{ version = {serde_json_version}, features = [\"preserve_order\"] }}\n",
                serde_json_version = toml_quote(SERDE_JSON_VERSION)
            ));
        } else {
            out.push_str(&format!(
                "serde_json = {serde_json_version}\n",
                serde_json_version = toml_quote(SERDE_JSON_VERSION)
            ));
        }
        if self.pyo3_extension {
            out.push_str(&format!(
                "pyo3 = {{ version = {pyo3_version}, features = [\"extension-module\"] }}\n",
                pyo3_version = toml_quote(PYO3_VERSION)
            ));
        }
        if self.rustc_hash {
            push_dependency(&mut out, "rustc-hash", RUSTC_HASH_VERSION);
        }
        if self.regex {
            push_dependency(&mut out, "regex", REGEX_VERSION);
        }
        if self.lazy_static {
            push_dependency(&mut out, "lazy_static", LAZY_STATIC_VERSION);
        }
        if self.jsonschema {
            // The default features add the HTTP/TLS stack for fetching remote
            // `$ref`s. Generated validators register vendored resources and an
            // offline retriever instead, so none of it is ever used.
            out.push_str(&format!(
                "jsonschema = {{ version = {jsonschema_version}, default-features = false }}\n",
                jsonschema_version = toml_quote(JSONSCHEMA_VERSION)
            ));
        }
        if self.rayon {
            push_dependency(&mut out, "rayon", RAYON_VERSION);
        }
        if self.chrono {
            out.push_str(&format!(
                "chrono = {{ version = {chrono_version}, features = [\"serde\"] }}\n",
                chrono_version = toml_quote(CHRONO_VERSION)
            ));
        }
        if self.base64 {
            push_dependency(&mut out, "base64", BASE64_VERSION);
        }
        if self.sha2 {
            push_dependency(&mut out, "sha2", SHA2_VERSION);
        }
        if let Some(path) = &self.tct_path {
            out.push_str(&format!("tct = {{ path = {} }}\n", toml_quote(path)));
        }

        if let Some(path) = &self.build_tct_path {
            out.push_str("\n[build-dependencies]\n");
            out.push_str(&format!(
                "serde_json = {serde_json_version}\n",
                serde_json_version = toml_quote(SERDE_JSON_VERSION)
            ));
            out.push_str(&format!("tct = {{ path = {} }}\n", toml_quote(path)));
        }

        if let Some(lib) = &self.lib {
            out.push_str("\n[lib]\n");
            if let Some(name) = &lib.name {
                push_key_value(&mut out, "name", name);
            }
            if let Some(path) = &lib.path {
                push_key_value(&mut out, "path", path);
            }
            if !lib.crate_types.is_empty() {
                let crate_types = lib
                    .crate_types
                    .iter()
                    .map(|crate_type| toml_quote(crate_type))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("crate-type = [{}]\n", crate_types));
            }
        }

        if let Some(bin) = &self.bin {
            out.push_str("\n[[bin]]\n");
            push_key_value(&mut out, "name", &bin.name);
            push_key_value(&mut out, "path", &bin.path);
        }

        if self.profiling_feature {
            out.push_str("\n[features]\n");
            out.push_str("default = []\n");
            out.push_str("profiling = []\n");
        }

        out
    }
}

fn push_dependency(out: &mut String, name: &str, version: &str) {
    out.push_str(&format!("{name} = {}\n", toml_quote(version)));
}

fn push_key_value(out: &mut String, key: &str, value: &str) {
    out.push_str(&format!("{key} = {}\n", toml_quote(value)));
}

fn toml_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_and_generated_toolchain_versions_match() {
        let cargo_toml = include_str!("../Cargo.toml");
        let rust_toolchain = include_str!("../rust-toolchain.toml");

        assert!(cargo_toml.contains(&format!("edition = \"{GENERATED_EDITION}\"")));
        assert!(cargo_toml.contains(&format!("rust-version = \"{GENERATED_RUST_VERSION}\"")));
        assert!(rust_toolchain.contains(&format!("channel = \"{GENERATED_RUST_VERSION}.0\"")));
    }

    #[test]
    fn generated_binary_template_uses_pinned_dependency_policy() {
        let cargo_toml = GeneratedCargoToml::new("generated-bin")
            .with_bin("generated-bin", "src/main.rs")
            .with_tct_path("/repo/tct")
            .with_preserve_order()
            .with_rustc_hash()
            .with_regex()
            .with_lazy_static()
            .with_jsonschema()
            .with_rayon()
            .with_chrono()
            .with_base64()
            .with_sha2()
            .with_release_lto()
            .with_profiling_feature()
            .render();

        assert!(cargo_toml.contains("[package]"));
        assert!(cargo_toml.contains("name = \"generated-bin\""));
        assert!(cargo_toml.contains("edition = \"2024\""));
        assert!(cargo_toml.contains("rust-version = \"1.95\""));
        assert!(cargo_toml.contains("license = \"Apache-2.0\""));
        assert!(cargo_toml.contains("repository = \"https://github.com/jonas-schmitt/tct\""));
        assert!(cargo_toml.contains("serde = { version = \"=1.0.228\", features = [\"derive\"] }"));
        assert!(
            cargo_toml.contains(
                "serde_json = { version = \"=1.0.149\", features = [\"preserve_order\"] }"
            )
        );
        assert!(cargo_toml.contains("rustc-hash = \"=2.1.2\""));
        assert!(cargo_toml.contains("regex = \"=1.12.3\""));
        assert!(cargo_toml.contains("lazy_static = \"=1.5.0\""));
        assert!(
            cargo_toml.contains("jsonschema = { version = \"=0.46.2\", default-features = false }")
        );
        assert!(cargo_toml.contains("rayon = \"=1.12.0\""));
        assert!(cargo_toml.contains("chrono = { version = \"=0.4.44\", features = [\"serde\"] }"));
        assert!(cargo_toml.contains("base64 = \"=0.22.1\""));
        assert!(cargo_toml.contains("sha2 = \"=0.11.0\""));
        assert!(cargo_toml.contains("tct = { path = \"/repo/tct\" }"));
        assert!(cargo_toml.contains("[profile.release]\nlto = true\ncodegen-units = 1"));
        assert!(cargo_toml.contains("[[bin]]\nname = \"generated-bin\"\npath = \"src/main.rs\""));
        assert!(cargo_toml.contains("[features]\ndefault = []\nprofiling = []"));
    }

    #[test]
    fn generated_wheel_template_uses_pinned_dependency_policy() {
        let cargo_toml = GeneratedCargoToml::new("generated-wheel")
            .version("0.2.0")
            .with_cdylib("generated_wheel")
            .with_tct_path("/repo/tct")
            .with_pyo3_extension()
            .with_jsonschema()
            .with_release_lto()
            .render();

        assert!(cargo_toml.contains("name = \"generated-wheel\""));
        assert!(cargo_toml.contains("version = \"0.2.0\""));
        assert!(cargo_toml.contains("license = \"Apache-2.0\""));
        assert!(cargo_toml.contains("repository = \"https://github.com/jonas-schmitt/tct\""));
        assert!(
            cargo_toml
                .contains("pyo3 = { version = \"=0.29.3\", features = [\"extension-module\"] }")
        );
        assert!(
            cargo_toml.contains("jsonschema = { version = \"=0.46.2\", default-features = false }")
        );
        assert!(
            cargo_toml.contains("[lib]\nname = \"generated_wheel\"\ncrate-type = [\"cdylib\"]")
        );
        assert!(cargo_toml.contains("[profile.release]\nlto = true\ncodegen-units = 1"));
    }

    #[test]
    fn generated_build_dependencies_share_workspace_pins() {
        let cargo_toml = GeneratedCargoToml::new("generated-build")
            .with_build_tct_path("/repo/tct")
            .render();

        assert!(cargo_toml.contains("[build-dependencies]"));
        assert!(cargo_toml.contains("serde_json = \"=1.0.149\""));
        assert!(cargo_toml.contains("tct = { path = \"/repo/tct\" }"));
    }

    #[test]
    fn generated_template_escapes_toml_string_values() {
        let cargo_toml = GeneratedCargoToml::new("quoted")
            .with_tct_path("/tmp/tct \"quoted\"")
            .render();

        assert!(cargo_toml.contains("tct = { path = \"/tmp/tct \\\"quoted\\\"\" }"));
    }
}
