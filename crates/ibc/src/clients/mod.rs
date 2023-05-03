//! Light client implementations to be used in [Core](core).
//!
//! [core]: https://github.com/cosmos/ibc-rs/tree/main/crates/ibc/src/core

use core::any::Any;

pub mod ics07_tendermint;

/// Allows type to be converted to `&dyn Any`
pub trait AsAny: Any {
    fn as_any(&self) -> &dyn Any;
}

impl<M: Any> AsAny for M {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
