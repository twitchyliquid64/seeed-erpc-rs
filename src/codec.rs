use super::ids::*;
use nom::{
    error::ParseError, lib::std::ops::RangeFrom, number::streaming, IResult, InputIter,
    InputLength, Slice,
};

const BASIC_CODEC_VERSION: u8 = 1;

/// header describing an RPC call to a specific service or handler
#[derive(Clone, Debug)]
pub struct Header {
    pub service: Service,
    pub request: u8,
    pub msg_type: MsgType,
    pub sequence: u32, // incrementing number.
}

impl Header {
    /// Encodes the RPC into its wire format
    pub fn as_bytes(&self) -> [u8; 8] {
        let header: u32 = (BASIC_CODEC_VERSION as u32) << 24
            | ((self.service as u32) << 16)
            | ((self.request as u32) << 8)
            | (self.msg_type as u32);
        let header = header.to_le_bytes();

        let seq = self.sequence.to_le_bytes();

        [
            header[0], header[1], header[2], header[3], seq[0], seq[1], seq[2], seq[3],
        ]
    }

    /// Decodes an RPC header from a byte slice or other compatible type
    pub fn parse<I, E: ParseError<I>>(i: I) -> IResult<I, Self, E>
    where
        I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
    {
        let (i, header) = streaming::le_u32(i)?;
        let (i, sequence) = streaming::le_u32(i)?;
        Ok((
            i,
            Self {
                service: (((header >> 16) & 0xff) as u8).into(),
                request: ((header >> 8) & 0xff) as u8,
                msg_type: ((header & 0xff) as u8).into(),
                sequence,
            },
        ))
    }
}

/// Wraps a complete RPC (Header + data) on stream transports, like a UART.
#[derive(Clone, Debug)]
pub struct FrameHeader {
    pub msg_length: u16,
    pub crc16: u16,
}

impl FrameHeader {
    /// Builds a frame header which will wrap the provided msg.
    pub fn new_from_msg(msg: &[u8]) -> Self {
        Self {
            msg_length: msg.len() as u16,
            crc16: crc16(msg),
        }
    }

    /// Encodes the frame header in its wire format.
    pub fn as_bytes(&self) -> [u8; 4] {
        let (l, c) = (self.msg_length.to_le_bytes(), self.crc16.to_le_bytes());
        [l[0], l[1], c[0], c[1]]
    }

    /// Nom parser which decodes a leading FrameHeader from the input. Can be
    /// chained with other nom parsers.
    pub fn parse<I, E: ParseError<I>>(i: I) -> IResult<I, Self, E>
    where
        I: Slice<RangeFrom<usize>> + InputIter<Item = u8> + InputLength,
    {
        let (i, msg_length) = streaming::le_u16(i)?;
        let (i, crc16) = streaming::le_u16(i)?;
        Ok((i, Self { msg_length, crc16 }))
    }

    /// Checks the CRC matches that computed from the provided payload.
    pub fn check_crc<I, E>(&self, data: I) -> Result<(), super::Err<E>>
    where
        I: InputIter<Item = u8>,
    {
        if crc16(data) == self.crc16 {
            Ok(())
        } else {
            Err(super::Err::CRCMismatch)
        }
    }
}

/// computes the CRC value used in the Wio Terminal eRPC codec
pub(crate) fn crc16<I>(data: I) -> u16
where
    I: InputIter<Item = u8>,
{
    let mut crc: u32 = 0xEF4A;

    for b in data.iter_elements() {
        crc ^= (b as u32) << 8;
        for _ in 0..8 {
            let mut temp: u32 = crc << 1;
            if (crc & 0x8000) != 0 {
                temp ^= 0x1021;
            }
            crc = temp;
        }
    }

    crc as u16
}
