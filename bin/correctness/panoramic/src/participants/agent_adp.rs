//! The `agent-adp` participant type: the Core Agent and ADP running together.

use crate::config::ParticipantDeclaration;

/// Configuration, connection and process names accepted by `agent-adp`.
pub(crate) const AGENT_ADP: ParticipantDeclaration = ParticipantDeclaration {
    name: "agent-adp",
    config_files: &[],
    endpoints: &[],
    consumes: &[],
    params: &[],
    processes: &["adp", "agent"],
};
