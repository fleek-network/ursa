use crate::narwhal::NarwhalService;

/// The consensus layer, which wraps a narwhal service and moves the epoch forward.
pub struct Consensus {
    narwhal: NarwhalService,
}
