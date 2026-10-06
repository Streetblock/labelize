use std::fmt;
use std::str::FromStr;

/// Interpretation policy, independent of image resolution and output format.
///
/// Existing render entry points use [`Self::Labelary`]. The experimental Zebra
/// profile currently changes only explicit ZPL QR character modes; it is not a
/// complete printer or firmware emulator. It must only be used for ZPL labels:
/// `LabelInfo` does not retain its source language, and EPL uses different QR
/// mode semantics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompatibilityProfile {
    /// Preserve the renderer's Labelary-oriented behavior.
    #[default]
    Labelary,
    /// Honor explicit ZPL QR Numeric, Alphanumeric and Byte modes.
    ZebraExperimental,
}

impl CompatibilityProfile {
    /// Stable name used by the CLI and HTTP API.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Labelary => "labelary",
            Self::ZebraExperimental => "zebra-experimental",
        }
    }
}

impl fmt::Display for CompatibilityProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CompatibilityProfile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "labelary" => Ok(Self::Labelary),
            "zebra-experimental" => Ok(Self::ZebraExperimental),
            _ => Err(format!(
                "unknown compatibility profile '{value}'; expected labelary or zebra-experimental"
            )),
        }
    }
}
