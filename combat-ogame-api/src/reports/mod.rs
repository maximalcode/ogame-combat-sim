//! On-demand community-proxy imports. Candidates are deliberately not
//! `CombatRequest`s: review missing fields and modifier semantics before simulation.
mod client;
mod completion;
mod model;
mod parse;
#[cfg(test)]
mod tests;

pub use client::ReportClient;
pub use completion::{
    CompletionEvidence, CompletionInput, CompletionResult, EvidenceLedger, EvidenceRecord,
    EvidenceSource, FieldIssue, FieldIssueKind, PartialLifeformBonus, ParticipantEvidence,
    PinnedUniverse, PinnedUniverseSettings, TechnologyBasis, TechnologyEvidence,
    VerifiedBattleInput, complete_candidate, complete_report,
};
pub use model::{Candidate, Participant, Provenance, ResourcesCandidate, TechnologyCandidate};
pub use parse::parse_report;

use std::fmt;

/// Maximum accepted full JSON envelope, for both offline and network input.
pub const MAX_REPORT_BYTES: usize = 2 * 1024 * 1024;

/// The two report kinds supported by this import path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportKind {
    Combat,
    Espionage,
}

/// An individual report capability. Formatting never reveals its secret suffix.
pub struct ReportId {
    value: String,
    kind: ReportKind,
    community: String,
    universe: u32,
}

impl ReportId {
    /// Accept one exact combat (`cr`) or espionage (`sr`) identifier, not a URL.
    pub fn parse(value: &str) -> Result<Self, ReportError> {
        if value.len() > 80 {
            return Err(ReportError::InvalidId);
        }
        let parts: Vec<_> = value.split('-').collect();
        if parts.len() != 4
            || parts[1].len() != 2
            || !parts[1].bytes().all(|byte| byte.is_ascii_lowercase())
            || parts[2].is_empty()
            || !parts[2].bytes().all(|byte| byte.is_ascii_digit())
            || parts[3].len() != 40
            || !parts[3].bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ReportError::InvalidId);
        }
        let kind = match parts[0] {
            "cr" => ReportKind::Combat,
            "sr" => ReportKind::Espionage,
            _ => return Err(ReportError::UnsupportedKind),
        };
        let universe = parts[2].parse().map_err(|_| ReportError::InvalidId)?;
        if universe == 0 {
            return Err(ReportError::InvalidId);
        }
        Ok(Self {
            value: value.to_owned(),
            kind,
            community: parts[1].to_owned(),
            universe,
        })
    }
}

impl fmt::Debug for ReportId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReportId([redacted])")
    }
}

/// Redacted failures: no underlying HTTP/JSON error or provider text is retained.
#[derive(Debug, PartialEq, Eq)]
pub enum ReportError {
    InvalidId,
    UnsupportedKind,
    Malformed,
    Provider,
    Field(String),
    TooLarge,
    HttpStatus(u16),
    Transport,
    Timeout,
    RateLimited { retry_after_seconds: u64 },
}

impl fmt::Display for ReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId => formatter.write_str("invalid report ID; supply one cr- or sr- identifier, not a URL"),
            Self::UnsupportedKind => formatter.write_str("unsupported report type; only combat and espionage reports are supported"),
            Self::Malformed => formatter.write_str("malformed or unsupported report response; obtain a fresh report or check proxy compatibility"),
            Self::Provider => formatter.write_str("proxy could not retrieve this report; it may be expired or unavailable; check the ID or obtain a fresh report"),
            Self::Field(field) => write!(formatter, "invalid report field {field}; cannot import this value safely"),
            Self::TooLarge => formatter.write_str("report exceeds the 2 MiB size limit"),
            Self::HttpStatus(status) => write!(formatter, "proxy returned HTTP {status}; report may be unavailable or expired; check the ID or try later"),
            Self::Transport => formatter.write_str("proxy connection failed; check connectivity and try later"),
            Self::Timeout => formatter.write_str("proxy request timed out; try again later"),
            Self::RateLimited { retry_after_seconds } => write!(formatter, "report request rate limited; wait at least {retry_after_seconds} seconds; independent processes share the provider quota"),
        }
    }
}

impl std::error::Error for ReportError {}
