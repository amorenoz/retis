//! Rust<>BPF types definitions for the ovs module.
//! Please keep this file in sync with its BPF counterpart in bpf/.

use anyhow::{bail, Result};
use plain::Plain;

use super::OvsEvent;
use crate::core::events::bpf::{parse_raw_section, BpfRawSection};

/// Types of events that can be generated by the ovs module.
#[derive(Debug, Eq, Hash, PartialEq)]
pub(crate) enum OvsEventType {
    /// Upcall tracepoint.
    Upcall = 0,
    /// Upcall enqueue kretprobe.
    UpcallEnqueue = 1,
    /// Upcall return.
    UpcallReturn = 2,
    /// Upcall received in userspace.
    RecvUpcall = 3,
    /// Flow operation
    Operation = 4,
    /// Execute action tracepoint.
    ActionExec = 5,
    /// Execute action tracking.
    ActionExecTrack = 6,
    /// OUTPUT action specific data.
    OutputAction = 7,
}

impl OvsEventType {
    pub(super) fn from_u8(val: u8) -> Result<OvsEventType> {
        use OvsEventType::*;
        Ok(match val {
            0 => Upcall,
            1 => UpcallEnqueue,
            2 => UpcallReturn,
            3 => RecvUpcall,
            4 => Operation,
            5 => ActionExec,
            6 => ActionExecTrack,
            7 => OutputAction,
            x => bail!("Can't construct a OvsEventType from {}", x),
        })
    }
}

/// OVS Upcall data.
#[derive(Default)]
#[repr(C, packed)]
struct UpcallEvent {
    /// Upcall command. Holds OVS_PACKET_CMD:
    ///   OVS_PACKET_CMD_UNSPEC   = 0
    ///   OVS_PACKET_CMD_MISS     = 1
    ///   OVS_PACKET_CMD_ACTION   = 2
    ///   OVS_PACKET_CMD_EXECUTE  = 3
    cmd: u8,
    /// Upcall port.
    port: u32,
    /// Cpu ID
    cpu: u32,
}
unsafe impl Plain for UpcallEvent {}

pub(super) fn unmarshall_upcall(raw_section: &BpfRawSection, event: &mut OvsEvent) -> Result<()> {
    let raw = parse_raw_section::<UpcallEvent>(raw_section)?;

    event.port = Some(raw.port);
    event.cmd = Some(raw.cmd);
    event.cpu = Some(raw.cpu);
    event.event_type = Some("upcall".to_string());

    Ok(())
}

/// OVS action event data.
#[derive(Default)]
#[repr(C, packed)]
struct ActionEvent {
    /// Action to be executed.
    action: u8,
    /// Recirculation id.
    recirc_id: u32,
}

unsafe impl Plain for ActionEvent {}

pub(super) fn unmarshall_exec(raw_section: &BpfRawSection, event: &mut OvsEvent) -> Result<()> {
    let raw = parse_raw_section::<ActionEvent>(raw_section)?;

    // Values from enum ovs_action_attr (uapi/linux/openvswitch.h).
    let action_str = match raw.action {
        0 => "unspecified",
        1 => "output",
        2 => "userspace",
        3 => "set",
        4 => "push_vlan",
        5 => "pop_vlan",
        6 => "sample",
        7 => "recirc",
        8 => "hash",
        9 => "push_mpls",
        10 => "pop_mpls",
        11 => "set_masked",
        12 => "ct",
        13 => "trunc",
        14 => "push_eth",
        15 => "pop_eth",
        16 => "ct_clear",
        17 => "push_nsh",
        18 => "pop_nsh",
        19 => "meter",
        20 => "clone",
        21 => "check_pkt_len",
        22 => "add_mpls",
        23 => "dec_ttl",
        val => bail!("Unsupported action id {val}"),
    };

    event.action = Some(action_str.to_string());
    event.recirculation_id = Some(raw.recirc_id);
    event.event_type = Some("action_execute".to_string());

    Ok(())
}

/// OVS action tracking event data.
#[derive(Default)]
#[repr(C, packed)]
struct ActionTrackEvent {
    /// Queue id.
    queue_id: u32,
}

unsafe impl Plain for ActionTrackEvent {}

pub(super) fn unmarshall_exec_track(
    raw_section: &BpfRawSection,
    event: &mut OvsEvent,
) -> Result<()> {
    let raw = parse_raw_section::<ActionTrackEvent>(raw_section)?;
    event.queue_id = Some(raw.queue_id);
    Ok(())
}

/// OVS output action data.
#[derive(Default)]
#[repr(C, packed)]
struct OutputAction {
    /// Output port.
    port: u32,
}
unsafe impl Plain for OutputAction {}

pub(super) fn unmarshall_output(raw_section: &BpfRawSection, event: &mut OvsEvent) -> Result<()> {
    let raw = parse_raw_section::<OutputAction>(raw_section)?;
    event.port = Some(raw.port);
    Ok(())
}

/// OVS Recv Upcall data.
#[derive(Default)]
#[repr(C, packed)]
struct RecvUpcall {
    r#type: u32,
    pkt_size: u32,
    key_size: u64,
    queue_id: u32,
    batch_ts: u64,
    batch_idx: u8,
}
unsafe impl Plain for RecvUpcall {}

pub(super) fn unmarshall_recv(raw_section: &BpfRawSection, event: &mut OvsEvent) -> Result<()> {
    let raw = parse_raw_section::<RecvUpcall>(raw_section)?;

    event.upcall_type = Some(raw.r#type);
    event.pkt_size = Some(raw.pkt_size);
    event.key_size = Some(raw.key_size);
    event.queue_id = Some(raw.queue_id);
    event.batch_ts = Some(raw.batch_ts);
    event.batch_idx = Some(raw.batch_idx);
    event.event_type = Some("recv_upcall".to_string());

    Ok(())
}

/// OVS Operation data.
#[derive(Default)]
#[repr(C, packed)]
struct OvsOperation {
    op_type: u8,
    queue_id: u32,
    batch_ts: u64,
    batch_idx: u8,
}
unsafe impl Plain for OvsOperation {}

pub(super) fn unmarshall_operation(
    raw_section: &BpfRawSection,
    event: &mut OvsEvent,
) -> Result<()> {
    let raw = parse_raw_section::<OvsOperation>(raw_section)?;

    event.op_type = Some(match raw.op_type {
        0 => "exec".to_string(),
        1 => "put".to_string(),
        x => bail!("Unknown operation type {x}"),
    });

    event.queue_id = Some(raw.queue_id);
    event.batch_ts = Some(raw.batch_ts);
    event.batch_idx = Some(raw.batch_idx);
    event.event_type = Some("flow_operation".to_string());

    Ok(())
}

/// Upcall enqueue data.
#[derive(Default)]
#[repr(C, packed)]
struct UpcallEnqueue {
    ret: i32,
    cmd: u8,
    port: u32,
    upcall_ts: u64,
    upcall_cpu: u32,
    queue_id: u32,
}
unsafe impl Plain for UpcallEnqueue {}

pub(super) fn unmarshall_upcall_enqueue(
    raw_section: &BpfRawSection,
    event: &mut OvsEvent,
) -> Result<()> {
    let raw = parse_raw_section::<UpcallEnqueue>(raw_section)?;

    event.r#return = Some(raw.ret);
    event.upcall_port = Some(raw.port);
    event.cmd = Some(raw.cmd);
    event.upcall_ts = Some(raw.upcall_ts);
    event.upcall_cpu = Some(raw.upcall_cpu);
    event.queue_id = Some(raw.queue_id);
    event.event_type = Some("upcall_enqueue".to_string());

    Ok(())
}

/// OVS Upcall return
#[derive(Default)]
#[repr(C, packed)]
struct UpcallReturn {
    upcall_ts: u64,
    upcall_cpu: u32,
    ret_code: i32,
}
unsafe impl Plain for UpcallReturn {}

pub(super) fn unmarshall_upcall_return(
    raw_section: &BpfRawSection,
    event: &mut OvsEvent,
) -> Result<()> {
    let raw = parse_raw_section::<UpcallReturn>(raw_section)?;

    event.upcall_ts = Some(raw.upcall_ts);
    event.upcall_cpu = Some(raw.upcall_cpu);
    event.return_code = Some(raw.ret_code);
    event.event_type = Some("upcall_return".to_string());

    Ok(())
}