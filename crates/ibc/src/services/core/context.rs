use crate::prelude::*;

use crate::{
    core::{
        ics02_client::{client_state::Status, client_type::ClientType},
        ics03_connection::connection::IdentifiedConnectionEnd,
        ics04_channel::{channel::IdentifiedChannelEnd, packet::Sequence},
        ics24_host::{
            identifier::{ClientId, ConnectionId},
            path::{AckPath, ChannelEndPath, CommitmentPath, Path},
        },
        ContextError, ValidationContext,
    },
    Height,
};

/// Context to be implemented by the host to provide proofs in gRPC query responses
///
/// Trait used for the [`gRPC query services`](crate::services).
pub trait ProvableContext {
    /// Returns the proof for the given path at the given height.
    /// As this is in the context of IBC, the path is expected to be an [`IbcPath`](Path).
    fn get_proof(&self, height: Height, path: &Path) -> Option<Vec<u8>>;
}

/// Context to be implemented by the host that provides gRPC query services.
///
/// Trait used for the [`gRPC query services`](crate::services).
pub trait QueryContext: ProvableContext + ValidationContext {
    // Client queries

    /// Returns the list of all clients.
    fn client_states(
        &self,
    ) -> Result<Vec<(ClientId, <Self as ValidationContext>::AnyClientState)>, ContextError>;

    /// Returns the list of all consensus states for the given client.
    fn consensus_states(
        &self,
        client_id: &ClientId,
    ) -> Result<Vec<(Height, <Self as ValidationContext>::AnyConsensusState)>, ContextError>;

    /// Returns the list of all heights at which consensus states for the given client are.
    fn consensus_state_heights(&self, client_id: &ClientId) -> Result<Vec<Height>, ContextError>;

    /// Returns the status of the given client.
    fn client_status(&self, client_id: &ClientId) -> Result<Status, ContextError>;

    /// Returns the list of supported client types.
    fn allowed_clients(&self) -> Vec<ClientType>;

    // Connection queries

    /// Returns the list of all connection ends.
    fn connection_ends(&self) -> Result<Vec<IdentifiedConnectionEnd>, ContextError>;

    /// Returns the list of all connection ids of the given client.
    fn client_connection_ends(
        &self,
        client_id: &ClientId,
    ) -> Result<Vec<ConnectionId>, ContextError>;

    // Channel queries

    /// Returns the list of all channel ends.
    fn channel_ends(&self) -> Result<Vec<IdentifiedChannelEnd>, ContextError>;

    /// Returns the list of all channel ends of the given connection.
    fn connection_channel_ends(
        &self,
        connection_id: &ConnectionId,
    ) -> Result<Vec<IdentifiedChannelEnd>, ContextError>;

    // Packet queries

    /// Returns the list of all packet commitments for the given channel end.
    fn packet_commitments(
        &self,
        channel_end_path: &ChannelEndPath,
    ) -> Result<Vec<CommitmentPath>, ContextError>;

    /// Filters the list of packet sequences for the given channel end that are acknowledged.
    /// Returns all the packet acknowledgements if `sequences` is empty.
    fn packet_acknowledgements(
        &self,
        channel_end_path: &ChannelEndPath,
        sequences: impl ExactSizeIterator<Item = Sequence>,
    ) -> Result<Vec<AckPath>, ContextError>;

    /// Filters the packet sequences for the given channel end that are not received.
    fn unreceived_packets(
        &self,
        channel_end_path: &ChannelEndPath,
        sequences: impl ExactSizeIterator<Item = Sequence>,
    ) -> Result<Vec<Sequence>, ContextError>;

    /// Filters the list of packet sequences for the given channel end whose acknowledgement is not received.
    /// Returns all the unreceived acknowledgements if `sequences` is empty.
    fn unreceived_acks(
        &self,
        channel_end_path: &ChannelEndPath,
        sequences: impl ExactSizeIterator<Item = Sequence>,
    ) -> Result<Vec<Sequence>, ContextError>;
}