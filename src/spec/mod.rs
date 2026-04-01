//! Protocol specification parsers.
//!
//! Handles two spec formats:
//! - Standard wire protocol (used by Cipher, Forge, Keep, Sentry, Veil, Courier, Chronicle, ShrouDB core)
//! - Sigil format (different command structure with `syntax`/`parameters` fields)
//! - Moat composite spec (references multiple engine specs)

pub mod moat;
pub mod wire;
