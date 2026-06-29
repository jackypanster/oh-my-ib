//! FROZEN SPEC — slim-claude-md. Offline: CLAUDE.md is a short pointer at the canonical AGENTS.md,
//! not a second copy of the conventions. The coder must NOT edit this file.
//! RED until impl slims CLAUDE.md (it is currently a ~4KB duplicate that never references AGENTS.md).

#[test]
fn claude_md_is_a_pointer_to_agents_md() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/CLAUDE.md");
    let text = std::fs::read_to_string(path).expect("CLAUDE.md must exist at the repo root");
    assert!(
        text.contains("AGENTS.md"),
        "CLAUDE.md must point at the canonical AGENTS.md"
    );
    assert!(
        text.len() > 100 && text.len() < 900,
        "CLAUDE.md must stay a short pointer, not a second copy (got {} bytes)",
        text.len()
    );
}
