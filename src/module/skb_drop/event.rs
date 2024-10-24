use std::{collections::HashMap, fmt};

use anyhow::Result;
use btf_rs::Type;
use log::warn;
use plain::Plain;

use crate::{
    core::{
        events::{
            bpf::{parse_single_raw_section, BpfRawSection},
            *,
        },
        inspect::inspector,
    },
    event_section, event_section_factory,
    module::ModuleId,
};

// Skb drop event section. Same as the event from BPF, please keep in sync with
// its BPF counterpart.
#[event_section]
pub(crate) struct SkbDropEvent {
    /// Reason why a packet was freed/dropped. Only reported from specific
    /// functions. See `enum skb_drop_reason` in the kernel.
    pub(crate) drop_reason: String,
}

impl EventFmt for SkbDropEvent {
    fn event_fmt(&self, f: &mut fmt::Formatter, _: DisplayFormat) -> fmt::Result {
        write!(f, "drop ({})", self.drop_reason)
    }
}

#[derive(Default)]
#[event_section_factory(SkbDropEvent)]
pub(crate) struct SkbDropEventFactory {
    /// Map of u32 to skb free reasons. It is filled lazyly to avoid executing
    /// `parse_drop_reasons` on a machine not collecting events. A `Some` value
    /// with an empty map means we couldn't retrieve the drop reasons from the
    /// running kernel.
    reasons: Option<HashMap<u32, String>>,
}

impl SkbDropEventFactory {
    pub(crate) fn parse_drop_reasons(&mut self) -> Result<()> {
        let mut reasons = HashMap::new();

        if let (btf, Type::Enum(r#enum)) = inspector()?
            .kernel
            .btf
            .resolve_type_by_name("skb_drop_reason")?
        {
            for member in r#enum.members.iter() {
                if member.val() < 0 {
                    continue;
                }
                reasons.insert(
                    u32::try_from(member.val())?,
                    btf.resolve_name(member)?
                        .trim_start_matches("SKB_")
                        .trim_start_matches("DROP_REASON_")
                        .to_string(),
                );
            }
        } else {
            warn!("Can't retrieve skb drop reason definitions from the kernel. Events will contain raw data (enum int value).");
        }

        self.reasons = Some(reasons);
        Ok(())
    }
}

impl RawEventSectionFactory for SkbDropEventFactory {
    fn from_raw(&mut self, raw_sections: Vec<BpfRawSection>) -> Result<Box<dyn EventSection>> {
        let raw = parse_single_raw_section::<BpfSkbDropEvent>(ModuleId::SkbDrop, raw_sections)?;
        let drop_reason = raw.drop_reason;

        // Parse skb drop reasons if not already done.
        if self.reasons.is_none() {
            self.parse_drop_reasons()?;
        }

        // Unwrap as we just made sure it was Some(..).
        let drop_reason = match self.reasons.as_ref().unwrap().get(&drop_reason) {
            Some(r) => r.clone(),
            None => format!("{}", drop_reason),
        };

        Ok(Box::new(SkbDropEvent { drop_reason }))
    }
}

#[derive(Default)]
#[repr(C, packed)]
struct BpfSkbDropEvent {
    drop_reason: u32,
}

unsafe impl Plain for BpfSkbDropEvent {}
