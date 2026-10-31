//! Participant-type declarations and the registry the loader checks cases against.

use crate::config::ParticipantDeclaration;

mod agent_adp;

/// Participant types available to case files.
pub(crate) const REGISTERED_TYPES: &[&ParticipantDeclaration] = &[&agent_adp::AGENT_ADP];
