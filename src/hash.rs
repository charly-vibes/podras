//! Purpose: derive the canonical semantic payload and source hash of a brief.
//!
//! Responsibilities:
//! - strip volatile and self-referential fields from a brief;
//! - hash the canonical JSON of that payload;
//! - let callers supply a HEAD sha before the envelope is stamped.
//!
//! Rationale: determinism tests compare payloads byte-for-byte, so the hashing
//! rules live apart from the model contract they operate on.

use sha2::{Digest, Sha256};

use crate::model::Brief;

/// Serialize a brief with volatile and self-referential fields removed.
///
/// Two runs over identical inputs differ only in `generated_at`; `source_hash`
/// is a hash of this very payload, so it cannot participate in its own input.
/// The result is the semantic payload compared byte-for-byte by determinism
/// tests and hashed by [`source_hash`].
pub fn semantic_payload(brief: &Brief) -> serde_json::Value {
    let mut payload = serde_json::to_value(brief).expect("Brief is always JSON-serializable");
    if let Some(object) = payload.as_object_mut() {
        object.remove("generated_at");
        object.remove("source_hash");
    }
    payload
}

/// SHA-256 over the canonical JSON of the semantic payload.
///
/// Canonical JSON means sorted object keys (serde_json's default map ordering)
/// and no insignificant whitespace. `generated_at` and `source_hash` are
/// omitted, so re-rendering identical inputs yields identical hashes. When
/// `repo_head` is supplied it participates as `head_sha`, which lets a caller
/// hash a payload whose envelope has not been stamped with HEAD yet.
pub fn source_hash(brief: &Brief, repo_head: Option<&str>) -> String {
    let mut payload = semantic_payload(brief);
    if let (Some(head), Some(object)) = (repo_head, payload.as_object_mut()) {
        object.insert(
            "head_sha".to_string(),
            serde_json::Value::String(head.to_string()),
        );
    }
    let canonical = serde_json::to_string(&payload).expect("semantic payload is serializable");
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}
