//! Lowering, evaluation and export for cuecue (see docs/ARCHITECTURE.md).

/// An evaluation error, reduced to what tests compare: its kind and path (docs/SPEC.md §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: Kind,
    /// The value the error concerns, e.g. `services.api.replicas`; `None` for errors about no value.
    pub path: Option<String>,
}

/// Diagnostic kinds of docs/SPEC.md §5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Syntax,
    Reference,
    Conflict,
    Closed,
    Required,
    Incomplete,
    Cycle,
    Builtin,
    Call,
}

impl Kind {
    /// The name used in docs/SPEC.md §5 and in the oracle's normalized output.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Syntax => "syntax",
            Kind::Reference => "reference",
            Kind::Conflict => "conflict",
            Kind::Closed => "closed",
            Kind::Required => "required",
            Kind::Incomplete => "incomplete",
            Kind::Cycle => "cycle",
            Kind::Builtin => "builtin",
            Kind::Call => "call",
        }
    }
}

/// Evaluates `files` (path, source) and exports the result as JSON, formatted like
/// `cue export --out json`. Root-level files form the package; files in subdirectories are
/// available to imports, as in CUE's own test harness.
///
/// Evaluation isn't implemented yet (M2.2), so every call fails without diagnostics.
pub fn export_json(files: &[(&str, &str)]) -> Result<String, Vec<Error>> {
    let _ = files;
    Err(Vec::new())
}
