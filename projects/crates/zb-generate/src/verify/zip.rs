//! ZIP verification adapter surface (Living `06`).

use serde::{Deserialize, Serialize};
use zb_types::{ArtifactSet, ZbError, ZbResult};

/// Facts the Acorn ZIP adapter must eventually expose for full verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcornZipCapability {
    /// Can index ZIP / ZIP64 and discover member ranges.
    pub index_zip: bool,
    /// Can expose central-directory CRC32 / flags / version facts.
    pub member_crc_facts: bool,
    /// Bounded decode reports fully_consumed + sizes.
    pub bounded_decode_complete: bool,
    /// Decode returns expected / actual CRC32.
    pub decode_crc_pair: bool,
    /// ParseBudget / cancel is honored on scan and decode.
    pub parse_budget: bool,
    /// ZIP64 / data-descriptor / trailing-data status is queryable.
    pub format_status_query: bool,
}

impl AcornZipCapability {
    /// Current known gap relative to Living `06` §3 (no Acorn wire yet).
    pub fn current_gap() -> Self {
        Self {
            index_zip: false,
            member_crc_facts: false,
            bounded_decode_complete: false,
            decode_crc_pair: false,
            parse_budget: false,
            format_status_query: false,
        }
    }

    /// Whether the adapter is sufficient for the v0 ZIP profile verification gate.
    pub fn supports_v0_full_verify(&self) -> bool {
        self.index_zip
            && self.member_crc_facts
            && self.bounded_decode_complete
            && self.decode_crc_pair
            && self.parse_budget
    }

    /// Missing capability ids in stable order.
    pub fn missing(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if !self.index_zip {
            out.push("index_zip");
        }
        if !self.member_crc_facts {
            out.push("member_crc_facts");
        }
        if !self.bounded_decode_complete {
            out.push("bounded_decode_complete");
        }
        if !self.decode_crc_pair {
            out.push("decode_crc_pair");
        }
        if !self.parse_budget {
            out.push("parse_budget");
        }
        if !self.format_status_query {
            out.push("format_status_query");
        }
        out
    }
}

/// Structured verification report (independent of generate report).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Whether full verification completed successfully.
    pub ok: bool,
    /// Adapter capability snapshot used for this attempt.
    pub acorn: AcornZipCapability,
    /// Missing capabilities when verification could not run.
    pub missing_capabilities: Vec<String>,
    /// Stable diagnostic notes.
    pub notes: Vec<String>,
}

/// Verify emitted ZIP bytes through an independent parser path.
///
/// Until Acorn exposes CRC / complete-decode facts, this returns
/// [`verification_capability_missing`](ZbError::verification_capability_missing)
/// instead of treating index success as proof.
pub fn verify_zip_bytes(_bytes: &[u8]) -> ZbResult<VerificationReport> {
    let acorn = AcornZipCapability::current_gap();
    if !acorn.supports_v0_full_verify() {
        let missing: Vec<String> = acorn.missing().into_iter().map(str::to_string).collect();
        return Err(ZbError::verification_capability_missing(format!(
            "Acorn ZIP adapter missing required facts: {}",
            missing.join(", ")
        )));
    }
    Ok(VerificationReport {
        ok: true,
        acorn,
        missing_capabilities: Vec::new(),
        notes: vec!["full Acorn verification path not wired in this build".to_string()],
    })
}

/// Verify an [`ArtifactSet`] root artifact.
pub fn verify_artifact_set(set: &ArtifactSet) -> ZbResult<VerificationReport> {
    let root = set
        .artifacts
        .iter()
        .find(|a| a.id == set.root)
        .or_else(|| set.artifacts.first());
    match root {
        Some(artifact) => verify_zip_bytes(&artifact.bytes),
        None => Err(ZbError::new(
            zb_types::ErrorCode::InvalidTopology,
            "artifact set has no root bytes to verify",
        )),
    }
}
