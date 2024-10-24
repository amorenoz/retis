#ifndef __CORE_PROBE_KERNEL_BPF_HELPERS__
#define __CORE_PROBE_KERNEL_BPF_HELPERS__

#include <bpf/bpf_tracing.h>

enum bpf_func_id___x { BPF_FUNC_get_func_ip___5_15_0 = 42 };

/* The following helper retrieves the function IP in kprobes.
 *
 * The proper way to get the function IP from a kprobe is by using
 * bpf_get_func_ip, which was introduced in Linux v5.15. If running on an older
 * kernel, we can get the current IP and compute the previous IP. But when
 * CONFIG_X86_KERNEL_IBT=y, indirect call landing sites and former ones will
 * have an extra endbr or nop4 instruction making the function IP +4 further up;
 * in such cases the only way to retrieve the function IP is also by using
 * bpf_get_func_ip.
 *
 * However, support for bpf_get_func_ip, CONFIG_X86_KERNEL_IBT option and its
 * handling in bpf_get_func_ip were done in different commits, merged into
 * different kernel versions, with no Fixes: tag. So we might end up in a
 * situation where CONFIG_X86_KERNEL_IBT=y and bpf_get_func_ip does not support
 * it. Our strategy is to always use bpf_get_func_ip if available and still use
 * the manual computation otherwise to allow some stable/downstream kernels to
 * work. We can't do much more and it might happen that some kernels with
 * CONFIG_X86_KERNEL_IBT=y and bpf_get_func_ip won't work. Hopefully that should
 * be rare, and even less common over time.
 */
static __always_inline u64 kprobe_get_func_ip(struct pt_regs *ctx) {
	if (bpf_core_enum_value_exists(enum bpf_func_id___x, BPF_FUNC_get_func_ip___5_15_0))
		return bpf_get_func_ip(ctx);
	else
		return PT_REGS_IP(ctx) - 1;
}

#endif /* __CORE_PROBE_KERNEL_BPF_HELPERS__ */
