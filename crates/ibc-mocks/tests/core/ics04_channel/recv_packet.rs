use ibc::core::events::{IbcEvent, MessageEvent};
use ibc::core::ics02_client::ClientExecutionContext;
use ibc::core::ics03_connection::connection::{
    ConnectionEnd, Counterparty as ConnectionCounterparty, State as ConnectionState,
};
use ibc::core::ics03_connection::version::get_compatible_versions;
use ibc::core::ics04_channel::channel::{ChannelEnd, Counterparty, Order, State};
use ibc::core::ics04_channel::msgs::recv_packet::test_util::get_dummy_raw_msg_recv_packet;
use ibc::core::ics04_channel::msgs::recv_packet::MsgRecvPacket;
use ibc::core::ics04_channel::msgs::PacketMsg;
use ibc::core::ics04_channel::packet::Packet;
use ibc::core::ics04_channel::Version;
use ibc::core::ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId};
use ibc::core::timestamp::{Timestamp, ZERO_DURATION};
use ibc::core::{execute, validate, ExecutionContext, MsgEnvelope};
use ibc::prelude::*;
use ibc::test_utils::get_dummy_account_id;
use ibc::Height;
use ibc_mocks::core::definition::MockContext;
use ibc_mocks::relayer::context::RelayerContext;
use ibc_mocks::router::definition::MockRouter;
use rstest::*;
use test_log::test;

pub struct Fixture {
    pub context: MockContext,
    pub router: MockRouter,
    pub client_height: Height,
    pub host_height: Height,
    pub msg: MsgRecvPacket,
    pub conn_end_on_b: ConnectionEnd,
    pub chan_end_on_b: ChannelEnd,
}

#[fixture]
fn fixture() -> Fixture {
    let context = MockContext::default();

    let router = MockRouter::new_with_transfer();

    let host_height = context.query_latest_height().unwrap().increment();

    let client_height = host_height.increment();

    let msg = MsgRecvPacket::try_from(get_dummy_raw_msg_recv_packet(
        client_height.revision_height(),
    ))
    .unwrap();

    let packet = msg.packet.clone();

    let chan_end_on_b = ChannelEnd::new(
        State::Open,
        Order::default(),
        Counterparty::new(packet.port_id_on_a, Some(packet.chan_id_on_a)),
        vec![ConnectionId::default()],
        Version::new("ics20-1".to_string()),
    )
    .unwrap();

    let conn_end_on_b = ConnectionEnd::new(
        ConnectionState::Open,
        ClientId::default(),
        ConnectionCounterparty::new(
            ClientId::default(),
            Some(ConnectionId::default()),
            Default::default(),
        ),
        get_compatible_versions(),
        ZERO_DURATION,
    )
    .unwrap();

    Fixture {
        context,
        router,
        client_height,
        host_height,
        msg,
        conn_end_on_b,
        chan_end_on_b,
    }
}

#[rstest]
fn recv_packet_fail_no_channel(fixture: Fixture) {
    let Fixture {
        context,
        router,
        msg,
        ..
    } = fixture;

    let msg_envelope = MsgEnvelope::from(PacketMsg::from(msg));

    let res = validate(&context, &router, msg_envelope);

    assert!(
        res.is_err(),
        "Validation fails because no channel exists in the context"
    )
}

#[rstest]
fn recv_packet_validate_happy_path(fixture: Fixture) {
    let Fixture {
        context,
        router,
        msg,
        conn_end_on_b,
        chan_end_on_b,
        client_height,
        host_height,
        ..
    } = fixture;

    let packet = &msg.packet;
    let mut context = context
        .with_client(&ClientId::default(), client_height)
        .with_connection(ConnectionId::default(), conn_end_on_b)
        .with_channel(
            packet.port_id_on_b.clone(),
            packet.chan_id_on_b.clone(),
            chan_end_on_b,
        )
        .with_send_sequence(
            packet.port_id_on_b.clone(),
            packet.chan_id_on_b.clone(),
            1.into(),
        )
        .with_height(host_height)
        // This `with_recv_sequence` is required for ordered channels
        .with_recv_sequence(
            packet.port_id_on_b.clone(),
            packet.chan_id_on_b.clone(),
            packet.seq_on_a,
        );

    context
        .get_client_execution_context()
        .store_update_time(
            ClientId::default(),
            client_height,
            Timestamp::from_nanoseconds(1000).unwrap(),
        )
        .unwrap();
    context
        .get_client_execution_context()
        .store_update_height(
            ClientId::default(),
            client_height,
            Height::new(0, 5).unwrap(),
        )
        .unwrap();

    let msg_envelope = MsgEnvelope::from(PacketMsg::from(msg));

    let res = validate(&context, &router, msg_envelope);

    assert!(
        res.is_ok(),
        "Happy path: validation should succeed. err: {res:?}"
    )
}

#[rstest]
fn recv_packet_timeout_expired(fixture: Fixture) {
    let Fixture {
        context,
        router,
        msg,
        conn_end_on_b,
        chan_end_on_b,
        client_height,
        host_height,
        ..
    } = fixture;

    let packet_old = Packet {
        seq_on_a: 1.into(),
        port_id_on_a: PortId::transfer(),
        chan_id_on_a: ChannelId::default(),
        port_id_on_b: PortId::transfer(),
        chan_id_on_b: ChannelId::default(),
        data: Vec::new(),
        timeout_height_on_b: client_height.into(),
        timeout_timestamp_on_b: Timestamp::from_nanoseconds(1).unwrap(),
    };

    let msg_packet_old = MsgRecvPacket::new(
        packet_old,
        msg.proof_commitment_on_a.clone(),
        msg.proof_height_on_a,
        get_dummy_account_id(),
    );

    let msg_envelope = MsgEnvelope::from(PacketMsg::from(msg_packet_old));

    let context = context
        .with_client(&ClientId::default(), client_height)
        .with_connection(ConnectionId::default(), conn_end_on_b)
        .with_channel(PortId::transfer(), ChannelId::default(), chan_end_on_b)
        .with_send_sequence(PortId::transfer(), ChannelId::default(), 1.into())
        .with_height(host_height);

    let res = validate(&context, &router, msg_envelope);

    assert!(
        res.is_err(),
        "recv_packet validation should fail when the packet has timed out"
    )
}

#[rstest]
fn recv_packet_execute_happy_path(fixture: Fixture) {
    let Fixture {
        context,
        mut router,
        msg,
        conn_end_on_b,
        chan_end_on_b,
        client_height,
        ..
    } = fixture;
    let mut ctx = context
        .with_client(&ClientId::default(), client_height)
        .with_connection(ConnectionId::default(), conn_end_on_b)
        .with_channel(PortId::transfer(), ChannelId::default(), chan_end_on_b);

    let msg_env = MsgEnvelope::from(PacketMsg::from(msg));

    let res = execute(&mut ctx, &mut router, msg_env);

    assert!(res.is_ok());

    assert_eq!(ctx.events.len(), 4);
    assert!(matches!(
        &ctx.events[0],
        &IbcEvent::Message(MessageEvent::Channel)
    ));
    assert!(matches!(&ctx.events[1], &IbcEvent::ReceivePacket(_)));
    assert!(matches!(
        &ctx.events[2],
        &IbcEvent::Message(MessageEvent::Channel)
    ));
    assert!(matches!(&ctx.events[3], &IbcEvent::WriteAcknowledgement(_)));
}