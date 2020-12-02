#![no_std]

#[macro_use]
extern crate bitflags;

mod codec;
mod ids;

/// Encapsulates errors that might occur when issuing or processing eRPCs.
#[derive(Debug, Clone, PartialEq)]
pub enum Err<E> {
    /// Parsing via the nom crate indicated an error
    Parsing(nom::Err<()>),
    /// The CRC was wrong
    CRCMismatch,
    /// There was an issue while transmitting
    TXErr,
    /// The response we were given to parse was for a different (callback,
    /// probably) RPC.
    NotOurs,
    /// There was an RPC-specific error.
    RPCErr(E),
    /// Too much data was present in the response
    ResponseOverrun,
    Unknown,
}

impl<E> From<nom::Err<()>> for Err<E> {
    fn from(nom_err: nom::Err<()>) -> Self {
        Err::Parsing::<E>(nom_err)
    }
}

pub use codec::{FrameHeader, Header};

/// Describes an RPC used by the system.
pub trait RPC {
    type ReturnValue;
    type Error;

    fn header(&self, seq: u32) -> Header;
    fn args(&self, _buff: &mut heapless::Vec<u8, heapless::consts::U64>) {}

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>>;
}

mod system_rpcs;
mod wifi_rpcs;

pub mod rpcs {
    pub use crate::system_rpcs::*;
    pub use crate::wifi_rpcs::*;
}
