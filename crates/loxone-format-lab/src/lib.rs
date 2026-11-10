//! Reverse-engineering toolkit for Loxone `.Loxone` / `.LoxCC` project files.
//!
//! The lab is intentionally independent of the GUI. It turns a corpus of real
//! projects into evidence: type/attribute/pin frequencies, structural diffs
//! between two saves, schema coverage reports, and starter schema code.

pub mod corpus;
pub mod diff;
pub mod dom;
pub mod gen;
pub mod report;
pub mod roundtrip;
pub mod stats;
