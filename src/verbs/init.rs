//! Purpose: implement `podras init`.
//!
//! Responsibilities:
//! - scaffold `.podras/curation.toml` from a commented template when absent;
//! - inject or refresh the AGENTS.md PODRAS managed block idempotently;
//! - report each action, or `already initialized` when nothing changed.
//!
//! Rationale: init is the only onboarding mutation; idempotence is the property
//! that lets humans and agents run it unconditionally.

use std::fs;
use std::path::Path;

use genesis::managed_block::{BlockDef, BlockInjector, BlockRegistry, InjectResult};

use crate::curation;
use crate::error::Result;

/// Repository-relative AGENTS.md path.
const AGENTS_REL_PATH: &str = "AGENTS.md";

/// Name of the managed block init owns.
const BLOCK_NAME: &str = "PODRAS";

/// Commented curation template; parses as an all-defaults curation file.
const CURATION_TEMPLATE: &str = r#"# .podras/curation.toml — human-owned intent for this repository.
#
# `podras brief` reads this file; `podras focus` and `podras park` are the
# supported ways to change it. Every key is optional, so the commented shape
# below is the default. Dates are `YYYY-MM-DD` (RFC 3339 is also accepted).

# One-line project purpose, used in the brief heading.
# purpose_lite = "..."

# Focus cap. Defaults to 1 when absent.
# max_focus = 1

# Whether whisper notes may appear in the committed render. Defaults to false.
# publish_whisper = false

# Human health verdict, owned by a person, never inferred from signals.
# [health]
# status = "green"          # green | amber | red
# owner = "..."
# confirmed_at = "YYYY-MM-DD"
# reason = "..."

# Focused items. The brief reports how many sources disagree; the human decides.
# [[focus]]
# id = "..."
# reason = "..."
# focused_since = "YYYY-MM-DD"
# confirmed_at = "YYYY-MM-DD"

# Parked items stay parked until the resurface trigger fires.
# [[parked]]
# id = "..."
# reason = "..."
# resurface = "YYYY-MM-DD"  # or "event:<name>"
# parked_at = "YYYY-MM-DD"
# confirmed_at = "YYYY-MM-DD"
"#;

/// Managed block text: the briefing contract every agent must follow.
const BLOCK_CONTENT: &str = "\n## podras

- Run `podras brief --json` first; it is the machine-readable status contract.
- Check freshness before inference: stale or missing sources are warnings, not facts.
- Beads owns issue lifecycle; focus is intent, not permission.
- Never silently change focus or unpark an item; ask the human and use the verbs.
- Never hand-edit committed renders; regenerate them with `podras brief --write`.
- Roadmap `later`/`shaping`/`deferred` items are not requirements; no-gos are hard constraints.
- Never edit `.podras/roadmap.toml`; admission, deferral and kill are human decisions.
";

/// Initialize podras state for the repository.
pub fn run(repo: &Path) -> Result<()> {
    let curation_action = scaffold_curation(repo)?;
    let agents_action = inject_agents_block(repo)?;

    match (curation_action, agents_action) {
        (false, None) => println!("already initialized"),
        (created, action) => {
            if created {
                println!("created .podras/curation.toml");
            }
            if let Some(action) = action {
                println!("{}", describe(action));
            }
        }
    }
    Ok(())
}

/// Write the curation template when the file is absent.
///
/// Returns `true` when the file was created. Existing bytes are never touched:
/// curation is human-owned, and init is not a formatter.
///
/// The template is staged in a sibling temp file and hard-linked into place.
/// `link` refuses to replace an existing directory entry, so a concurrent
/// writer that lands first keeps its bytes and the failure surfaces as
/// `AlreadyExists`. Linking also means a crash can only leave the staging file
/// behind; `curation.toml` never exists partially written, so a later run
/// cannot mistake a torn template for human-authored curation.
fn scaffold_curation(repo: &Path) -> Result<bool> {
    let path = repo.join(curation::CURATION_REL_PATH);
    let dir = repo.join(".podras");
    fs::create_dir_all(&dir)?;
    let staged = dir.join(format!(
        ".curation.toml.scaffold.{}.tmp",
        std::process::id()
    ));
    fs::write(&staged, CURATION_TEMPLATE).inspect_err(|_| {
        let _ = fs::remove_file(&staged);
    })?;
    let linked = fs::hard_link(&staged, &path);
    let _ = fs::remove_file(&staged);
    match linked {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Inject or refresh the PODRAS block, returning an action only if bytes moved.
///
/// The injector reports `Updated` even for identical content, so byte equality
/// decides whether the run was a no-op; this keeps repeat runs quiet and
/// byte-identical.
fn inject_agents_block(repo: &Path) -> Result<Option<InjectResult>> {
    let path = repo.join(AGENTS_REL_PATH);
    let before = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let result = injector().inject(&path, BLOCK_NAME, BLOCK_CONTENT)?;
    let after = fs::read(&path)?;
    if before.as_deref() == Some(after.as_slice()) {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}

/// Build the injector with the PODRAS block and provenance footer.
fn injector() -> BlockInjector {
    let mut registry = BlockRegistry::new();
    registry.register(BlockDef::new(BLOCK_NAME));
    BlockInjector::new(registry).with_provenance("podras")
}

/// Human-readable action line for an injection result.
fn describe(result: InjectResult) -> &'static str {
    match result {
        InjectResult::Created => "created AGENTS.md with the PODRAS block",
        InjectResult::Prepended => "injected the PODRAS block into AGENTS.md",
        InjectResult::Updated => "refreshed the PODRAS block in AGENTS.md",
    }
}
