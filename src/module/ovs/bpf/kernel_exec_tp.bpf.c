#include <openvswitch.h>
#include <bpf/bpf_core_read.h>

#include <common.h>
#include <ovs_common.h>
#include <netlink.h>

/* Please keep in sync with its Rust counterpart in crate::module::ovs::bpf.rs. */
struct exec_event {
	u8 action;
	u32 recirc_id;
} __attribute__((packed));

/* Please keep in sync with its Rust counterpart in crate::module::ovs::bpf.rs. */
struct exec_track_event {
	u32 queue_id;
} __attribute__((packed));

/* Please keep in sync with its Rust counterpart in crate::module::ovs::bpf.rs. */
struct exec_output {
	u32 port;
} __attribute__((packed));


static __always_inline s32 handle_tracking(struct retis_context *ctx,
					   struct retis_raw_event *event)
{
	struct exec_track_event *track;
	struct packet_buffer *buff;
	struct sk_buff *skb;
	int zero = 0;
	u64 tid = bpf_get_current_pid_tgid();

	if (!bpf_map_lookup_elem(&inflight_exec_cmd, &tid)) {
		/* This call to ovs_do_execute_action does not come from a
		* userspace command. */
		return 0;
	}

	skb = retis_get_sk_buff(ctx);
	if (!skb)
		return -1;

	buff = bpf_map_lookup_elem(&packet_buffers, &zero);
	if (!buff)
		return -1;

	track = get_event_section(event, COLLECTOR_OVS, OVS_DP_ACTION_TRACK,
				  sizeof(*track));
	if (!track)
		return -1;

	track->queue_id = hash_skb(buff, skb);
	return 1;
}

/* Hook for ovs_do_execute_action tracepoint. */
DEFINE_HOOK_RAW(
	struct nlattr *attr;
	struct sw_flow_key *key;
	struct exec_event *exec;

	key = (struct sw_flow_key *) ctx->regs.reg[2];
	if (!key)
		return 0;

	attr = (struct nlattr *) ctx->regs.reg[3];
	if (!attr)
		return 0;

	exec = get_event_section(event, COLLECTOR_OVS, OVS_DP_ACTION,
				 sizeof(*exec));
	if (!exec)
		return 0;

	exec->action = nla_type(attr);
	exec->recirc_id = BPF_CORE_READ(key, recirc_id);
	/* Discard events for unmatching packets not coming from
	 * userspace. */
	if (!handle_tracking(ctx, event) &&
	    !(ctx->filters_ret & RETIS_F_PACKET_PASS))
		return 0;

	// Add action-specific data for some actions.
	switch (exec->action) {
	case OVS_ACTION_ATTR_OUTPUT:
		{
		struct exec_output *output =
			get_event_section(event, COLLECTOR_OVS,
					  OVS_DP_ACTION_OUTPUT,
					  sizeof(*output));
		if (!output)
			return 0;

		bpf_probe_read_kernel(&output->port, sizeof(output->port),
				      nla_data(attr));
		break;
		}
	}

	return 0;
)

char __license[] SEC("license") = "GPL";