//! Sonic display selection at 8007c5c0..8007c648, after ordinary/LOD model selection.
//! This does not infer status or counter from a host's movement state.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    /// Keep the already selected model and draw the character's transition motion.
    Transition,
    /// Keep the already selected model and resolve this animation's motion.
    Current(u16),
    /// Resolve both model and motion from entry 30, without changing the animation clock.
    SpinModel,
}

/// Common action dispatcher at 80084410..80084428, before action-specific updates.
/// Action entry/reset writes remain the caller's responsibility.
pub fn advance_counter(status: u16, counter: u16) -> u16 {
    if status & 0x100 != 0 { counter.wrapping_add(1) } else { counter }
}

pub fn select(mode: i16, animation: u16, status: u16, character: u8, counter: u16) -> Selection {
    if mode == 2 {
        Selection::Transition
    } else if status & 0x100 != 0 && character != 12 && counter & 0x11 != 0 {
        Selection::SpinModel
    } else {
        Selection::Current(animation)
    }
}
