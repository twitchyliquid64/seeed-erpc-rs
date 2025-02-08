use super::{codec, ids, Err};
use heapless::String;
use nom::{number::streaming, InputIter};

/// Returns a string indicating the firmware version on the wifi chip.
pub struct GetVersion {}

impl super::RPC for GetVersion {
    type ReturnValue = String<16>;
    type Error = ();

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::System,
            request: ids::SystemRequest::VersionID.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<String<16>, Err<()>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::System
            || hdr.request != ids::SystemRequest::VersionID.into()
        {
            return Err(Err::NotOurs);
        }

        let (data, length) = streaming::le_u32(data)?;
        if length > 16 {
            return Err(Err::ResponseOverrun);
        }

        let mut out: Self::ReturnValue = String::new();
        for b in data.iter_elements() {
            out.push(b as char).map_err(|_| Err::ResponseOverrun)?;
        }
        Ok(out)
    }
}
