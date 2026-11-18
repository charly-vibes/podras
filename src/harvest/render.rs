//! Purpose: render the harvest candidate contract.
//!
//! Responsibilities:
//! - render the human narrative grouped by kind, with explicit empty groups;
//! - render the candidate payload inside the genesis success envelope.
//!
//! Rationale: the two renderings are the only output of harvest and share one
//! payload, so they sit together. Neither reads the repository.

use genesis::envelope::{Envelope, EnvelopeKind};

use super::{CandidateKind, Harvest};
use crate::error::Result;

/// Render the human narrative grouped by kind, with explicit empty groups.
pub fn render(harvest: &Harvest) -> String {
    let mut out = format!("Harvest — {} candidate(s)\n", harvest.candidates.len());
    for kind in CandidateKind::ALL {
        out.push_str(&format!("\n{}\n", kind.as_str()));
        if !render_group(&mut out, harvest, kind) {
            out.push_str("  none\n");
        }
    }
    out
}

/// Render one kind's candidates; false when the group is empty.
fn render_group(out: &mut String, harvest: &Harvest, kind: CandidateKind) -> bool {
    let mut found = false;
    for candidate in harvest
        .candidates
        .iter()
        .filter(|candidate| candidate.kind == kind)
    {
        found = true;
        out.push_str(&format!("  - {}\n", candidate.summary));
        for evidence in &candidate.evidence {
            out.push_str(&format!(
                "    {}: {}\n",
                evidence.kind.as_str(),
                evidence.reference
            ));
        }
    }
    found
}

/// Render the candidate contract inside the genesis success envelope.
pub fn to_json(harvest: &Harvest) -> Result<String> {
    let envelope = Envelope::success(
        env!("CARGO_PKG_VERSION"),
        EnvelopeKind::Ok,
        harvest.clone(),
        Vec::new(),
        Vec::new(),
    );
    let mut rendered = serde_json::to_string_pretty(&envelope)?;
    rendered.push('\n');
    Ok(rendered)
}
