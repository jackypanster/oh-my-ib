//! FROZEN SPEC — agents-md. Offline: `AGENTS.md` (the canonical agent-conventions doc) exists at the
//! repo root and states the load-bearing facts an agent needs. The coder must NOT edit this file.
//! RED until impl writes + tracks `AGENTS.md` with the markers below.

#[test]
fn agents_md_states_the_conventions() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/AGENTS.md");
    let text = std::fs::read_to_string(path).expect("AGENTS.md must exist at the repo root");
    for marker in [
        "agent-first",              // the operator's authoring principle
        "Authoring (agent-first)",  // the section sentinel
        "CONTRACT.md",              // pointer to the pipeline contract (how the repo is built)
        "OMI_ALLOW_LIVE",           // the hard live-write safety gate
    ] {
        assert!(text.contains(marker), "AGENTS.md must contain marker {marker:?}");
    }
    assert!(
        text.len() > 800,
        "AGENTS.md must be substantive (got {} bytes)",
        text.len()
    );
}
