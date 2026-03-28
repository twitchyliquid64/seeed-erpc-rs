//! BSD socket RPCs via the RTL8720DN LwIP layer (eRPC service 16).
//!
//! These RPCs wrap the standard POSIX-like socket calls that the
//! RTL8720DN firmware exposes through its eRPC interface.  The C++
//! Seeed Arduino rpcUnified library implements the same calls in
//! `lwip/sockets.c` using `rpc_lwip_*` functions.
//!
//! Only the subset needed for UDP communication is implemented here;
//! TCP-specific calls can be added following the same pattern.

#[allow(dead_code)]
use super::{codec, ids, Err};
use nom::{bytes::streaming::take, number::streaming};

// ── BSD socket constants ────────────────────────────────────────────

/// Address family: IPv4
pub const AF_INET: i32 = 2;
/// Socket type: datagram (UDP)
pub const SOCK_DGRAM: i32 = 2;
/// Socket option level: socket-level options
pub const SOL_SOCKET: i32 = 0xfff;
/// Socket option: allow address reuse
pub const SO_REUSEADDR: i32 = 0x0004;
/// fcntl command: set file status flags
pub const F_SETFL: i32 = 4;
/// File status flag: non-blocking I/O
pub const O_NONBLOCK: i32 = 1;
/// recvfrom flag: return immediately even if no data
pub const MSG_DONTWAIT: i32 = 0x08;

// ── Helper: encode a sockaddr_in into 16 bytes ─────────────────────

/// Encode a `sockaddr_in` suitable for the RTL8720DN LwIP stack.
///
/// Layout (16 bytes, matching `struct sockaddr_in`):
/// ```text
///   [0]      sin_len    = 16
///   [1]      sin_family = AF_INET (2)
///   [2..4]   sin_port   (network byte order / big-endian)
///   [4..8]   sin_addr   (network byte order / big-endian)
///   [8..16]  sin_zero   (padding, all zeros)
/// ```
pub fn encode_sockaddr_in(ip: [u8; 4], port: u16) -> [u8; 16] {
    let mut buf = [0u8; 16];
    buf[0] = 16; // sin_len
    buf[1] = AF_INET as u8; // sin_family
    let port_be = port.to_be_bytes();
    buf[2] = port_be[0];
    buf[3] = port_be[1];
    // sin_addr in network byte order (already in the natural [a,b,c,d] form)
    buf[4] = ip[0];
    buf[5] = ip[1];
    buf[6] = ip[2];
    buf[7] = ip[3];
    // sin_zero[8..16] already zero
    buf
}

/// Decode a `sockaddr_in` from a 16-byte slice.
///
/// Returns `(ip_octets, port)`.
pub fn decode_sockaddr_in(buf: &[u8]) -> ([u8; 4], u16) {
    let port = u16::from_be_bytes([buf[2], buf[3]]);
    let ip = [buf[4], buf[5], buf[6], buf[7]];
    (ip, port)
}

// ── Helper: encode binary_t (length-prefixed blob) ──────────────────

/// Append a `binary_t` value (u32 length prefix + data) to a buffer.
fn push_binary(buff: &mut heapless::Vec<u8, 256>, data: &[u8]) {
    let len = data.len() as u32;
    buff.extend_from_slice(&len.to_le_bytes()).ok();
    buff.extend_from_slice(data).ok();
}

// ── LwipSocket ──────────────────────────────────────────────────────

/// Create a new socket.
///
/// Equivalent to `socket(domain, type, protocol)`.
/// Returns a file descriptor (>= 0) on success, or -1 on error.
pub struct LwipSocket {
    pub domain: i32,
    pub socket_type: i32,
    pub protocol: i32,
}

impl super::RPC for LwipSocket {
    type ReturnValue = i32;
    type Error = ();

    fn args(&self, buff: &mut heapless::Vec<u8, 256>) {
        buff.extend_from_slice(&self.domain.to_le_bytes()).ok();
        buff.extend_from_slice(&self.socket_type.to_le_bytes()).ok();
        buff.extend_from_slice(&self.protocol.to_le_bytes()).ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::LwIP,
            request: ids::LwIPRequest::Socket.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::LwIP
            || hdr.request != ids::LwIPRequest::Socket.into()
        {
            return Err(Err::NotOurs);
        }
        let (_, ret) = streaming::le_i32(data)?;
        Ok(ret)
    }
}

// ── LwipBind ────────────────────────────────────────────────────────

/// Bind a socket to a local address.
///
/// Equivalent to `bind(s, name, namelen)`.
/// `name` is a `sockaddr_in` (16 bytes).
pub struct LwipBind {
    pub socket: i32,
    /// Raw `sockaddr_in` bytes (16 bytes). Use [`encode_sockaddr_in`].
    pub name: [u8; 16],
}

impl super::RPC for LwipBind {
    type ReturnValue = i32;
    type Error = ();

    fn args(&self, buff: &mut heapless::Vec<u8, 256>) {
        buff.extend_from_slice(&self.socket.to_le_bytes()).ok();
        push_binary(buff, &self.name);
        buff.extend_from_slice(&(self.name.len() as u32).to_le_bytes())
            .ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::LwIP,
            request: ids::LwIPRequest::Bind.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::LwIP
            || hdr.request != ids::LwIPRequest::Bind.into()
        {
            return Err(Err::NotOurs);
        }
        let (_, ret) = streaming::le_i32(data)?;
        Ok(ret)
    }
}

// ── LwipSetsockopt ──────────────────────────────────────────────────

/// Set a socket option.
///
/// Equivalent to `setsockopt(s, level, optname, optval, optlen)`.
pub struct LwipSetsockopt<'a> {
    pub socket: i32,
    pub level: i32,
    pub optname: i32,
    pub optval: &'a [u8],
}

impl<'a> super::RPC for LwipSetsockopt<'a> {
    type ReturnValue = i32;
    type Error = ();

    fn args(&self, buff: &mut heapless::Vec<u8, 256>) {
        buff.extend_from_slice(&self.socket.to_le_bytes()).ok();
        buff.extend_from_slice(&self.level.to_le_bytes()).ok();
        buff.extend_from_slice(&self.optname.to_le_bytes()).ok();
        push_binary(buff, self.optval);
        buff.extend_from_slice(&(self.optval.len() as u32).to_le_bytes())
            .ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::LwIP,
            request: ids::LwIPRequest::SetSockOpt.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::LwIP
            || hdr.request != ids::LwIPRequest::SetSockOpt.into()
        {
            return Err(Err::NotOurs);
        }
        let (_, ret) = streaming::le_i32(data)?;
        Ok(ret)
    }
}

// ── LwipFcntl ───────────────────────────────────────────────────────

/// Manipulate file descriptor flags.
///
/// Equivalent to `fcntl(s, cmd, val)`.
pub struct LwipFcntl {
    pub socket: i32,
    pub cmd: i32,
    pub val: i32,
}

impl super::RPC for LwipFcntl {
    type ReturnValue = i32;
    type Error = ();

    fn args(&self, buff: &mut heapless::Vec<u8, 256>) {
        buff.extend_from_slice(&self.socket.to_le_bytes()).ok();
        buff.extend_from_slice(&self.cmd.to_le_bytes()).ok();
        buff.extend_from_slice(&self.val.to_le_bytes()).ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::LwIP,
            request: ids::LwIPRequest::Fcntl.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::LwIP
            || hdr.request != ids::LwIPRequest::Fcntl.into()
        {
            return Err(Err::NotOurs);
        }
        let (_, ret) = streaming::le_i32(data)?;
        Ok(ret)
    }
}

// ── LwipClose ───────────────────────────────────────────────────────

/// Close a socket.
///
/// Equivalent to `close(s)`.
pub struct LwipClose {
    pub socket: i32,
}

impl super::RPC for LwipClose {
    type ReturnValue = i32;
    type Error = ();

    fn args(&self, buff: &mut heapless::Vec<u8, 256>) {
        buff.extend_from_slice(&self.socket.to_le_bytes()).ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::LwIP,
            request: ids::LwIPRequest::Close.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::LwIP
            || hdr.request != ids::LwIPRequest::Close.into()
        {
            return Err(Err::NotOurs);
        }
        let (_, ret) = streaming::le_i32(data)?;
        Ok(ret)
    }
}

// ── LwipRecvfrom ────────────────────────────────────────────────────

/// Received UDP datagram with sender information.
#[derive(Debug)]
pub struct RecvResult {
    /// Payload bytes received.
    pub data: heapless::Vec<u8, 256>,
    /// Sender IP address octets `[a, b, c, d]`.
    pub remote_ip: [u8; 4],
    /// Sender UDP port.
    pub remote_port: u16,
}

/// Receive a datagram from a socket, returning the sender's address.
///
/// Equivalent to `recvfrom(s, mem, len, flags, from, fromlen)`.
///
/// The RTL8720DN firmware additionally takes a `timeout` parameter
/// (in milliseconds) appended after `fromlen`.
///
/// Returns `None` (via `Err::RPCErr(None)`) when no data is available
/// (EWOULDBLOCK with non-blocking socket).
pub struct LwipRecvfrom {
    pub socket: i32,
    pub len: u32,
    pub flags: i32,
    pub timeout_ms: u32,
}

impl super::RPC for LwipRecvfrom {
    type ReturnValue = RecvResult;
    type Error = i32;

    fn args(&self, buff: &mut heapless::Vec<u8, 256>) {
        buff.extend_from_slice(&self.socket.to_le_bytes()).ok();
        buff.extend_from_slice(&self.len.to_le_bytes()).ok();
        buff.extend_from_slice(&self.flags.to_le_bytes()).ok();
        // fromlen — size of sockaddr_in buffer
        let fromlen: u32 = 16;
        buff.extend_from_slice(&fromlen.to_le_bytes()).ok();
        buff.extend_from_slice(&self.timeout_ms.to_le_bytes()).ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::LwIP,
            request: ids::LwIPRequest::Recvfrom.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::LwIP
            || hdr.request != ids::LwIPRequest::Recvfrom.into()
        {
            return Err(Err::NotOurs);
        }

        // Response layout from RTL8720DN:
        //   binary_t mem     → u32 length + data bytes
        //   binary_t from    → u32 length + sockaddr_in bytes
        //   u32      fromlen
        //   i32      ret     (bytes received, or -1 on error)

        // Parse binary_t mem
        let (data, mem_len) = streaming::le_u32(data)?;
        let (data, mem_data) = take(mem_len as usize)(data)?;

        // Parse binary_t from
        let (data, from_len) = streaming::le_u32(data)?;
        let (data, from_data) = take(from_len as usize)(data)?;

        // Parse fromlen (u32) and ret (i32)
        let (data, _fromlen) = streaming::le_u32(data)?;
        let (_, ret) = streaming::le_i32(data)?;

        if ret < 0 {
            return Err(Err::RPCErr(ret));
        }

        let (remote_ip, remote_port) = if from_len >= 8 {
            decode_sockaddr_in(from_data)
        } else {
            ([0u8; 4], 0u16)
        };

        let mut payload = heapless::Vec::new();
        let copy_len = core::cmp::min(ret as usize, mem_len as usize);
        payload.extend_from_slice(&mem_data[..copy_len]).ok();

        Ok(RecvResult {
            data: payload,
            remote_ip,
            remote_port,
        })
    }
}

// ── LwipSendto ──────────────────────────────────────────────────────

/// Send a datagram to a specific destination.
///
/// Equivalent to `sendto(s, dataptr, size, flags, to, tolen)`.
pub struct LwipSendto<'a> {
    pub socket: i32,
    pub data: &'a [u8],
    pub flags: i32,
    /// Destination address as raw `sockaddr_in` (16 bytes).
    /// Use [`encode_sockaddr_in`].
    pub to: [u8; 16],
}

impl<'a> super::RPC for LwipSendto<'a> {
    type ReturnValue = i32;
    type Error = ();

    fn args(&self, buff: &mut heapless::Vec<u8, 256>) {
        buff.extend_from_slice(&self.socket.to_le_bytes()).ok();
        push_binary(buff, self.data);
        buff.extend_from_slice(&self.flags.to_le_bytes()).ok();
        push_binary(buff, &self.to);
        buff.extend_from_slice(&(self.to.len() as u32).to_le_bytes())
            .ok();
    }

    fn header(&self, seq: u32) -> codec::Header {
        codec::Header {
            sequence: seq,
            msg_type: ids::MsgType::Invocation,
            service: ids::Service::LwIP,
            request: ids::LwIPRequest::Sendto.into(),
        }
    }

    fn parse(&mut self, data: &[u8]) -> Result<Self::ReturnValue, Err<Self::Error>> {
        let (data, hdr) = codec::Header::parse(data)?;
        if hdr.msg_type != ids::MsgType::Reply
            || hdr.service != ids::Service::LwIP
            || hdr.request != ids::LwIPRequest::Sendto.into()
        {
            return Err(Err::NotOurs);
        }
        let (_, ret) = streaming::le_i32(data)?;
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RPC;

    #[test]
    fn socket_args_encoding() {
        let rpc = LwipSocket {
            domain: AF_INET,
            socket_type: SOCK_DGRAM,
            protocol: 0,
        };
        let mut buf = heapless::Vec::<u8, 256>::new();
        rpc.args(&mut buf);
        // domain=2 (LE), type=2 (LE), protocol=0 (LE) → 12 bytes
        assert_eq!(buf.len(), 12);
        assert_eq!(&buf[0..4], &2i32.to_le_bytes()); // AF_INET
        assert_eq!(&buf[4..8], &2i32.to_le_bytes()); // SOCK_DGRAM
        assert_eq!(&buf[8..12], &0i32.to_le_bytes()); // protocol
    }

    #[test]
    fn bind_args_encoding() {
        let addr = encode_sockaddr_in([192, 168, 1, 100], 4000);
        let rpc = LwipBind {
            socket: 3,
            name: addr,
        };
        let mut buf = heapless::Vec::<u8, 256>::new();
        rpc.args(&mut buf);
        // socket(4) + binary_t(4 len + 16 data) + namelen(4) = 28 bytes
        assert_eq!(buf.len(), 28);
        assert_eq!(&buf[0..4], &3i32.to_le_bytes()); // socket fd
        assert_eq!(&buf[4..8], &16u32.to_le_bytes()); // binary_t length
        assert_eq!(buf[9], AF_INET as u8); // sin_family
        // port 4000 = 0x0FA0 → big-endian [0x0F, 0xA0]
        assert_eq!(buf[10], 0x0F);
        assert_eq!(buf[11], 0xA0);
    }

    #[test]
    fn sockaddr_in_roundtrip() {
        let ip = [10, 0, 0, 1];
        let port = 5000;
        let encoded = encode_sockaddr_in(ip, port);
        let (dec_ip, dec_port) = decode_sockaddr_in(&encoded);
        assert_eq!(dec_ip, ip);
        assert_eq!(dec_port, port);
    }

    #[test]
    fn fcntl_args_encoding() {
        let rpc = LwipFcntl {
            socket: 5,
            cmd: F_SETFL,
            val: O_NONBLOCK,
        };
        let mut buf = heapless::Vec::<u8, 256>::new();
        rpc.args(&mut buf);
        assert_eq!(buf.len(), 12);
        assert_eq!(&buf[0..4], &5i32.to_le_bytes());
        assert_eq!(&buf[4..8], &F_SETFL.to_le_bytes());
        assert_eq!(&buf[8..12], &O_NONBLOCK.to_le_bytes());
    }

    #[test]
    fn sendto_args_encoding() {
        let dest = encode_sockaddr_in([192, 168, 1, 1], 8080);
        let payload = b"hello";
        let rpc = LwipSendto {
            socket: 3,
            data: payload,
            flags: 0,
            to: dest,
        };
        let mut buf = heapless::Vec::<u8, 256>::new();
        rpc.args(&mut buf);
        // socket(4) + binary_t data(4+5) + flags(4) + binary_t to(4+16) + tolen(4) = 41
        assert_eq!(buf.len(), 41);
        assert_eq!(&buf[0..4], &3i32.to_le_bytes()); // socket
        assert_eq!(&buf[4..8], &5u32.to_le_bytes()); // data length
        assert_eq!(&buf[8..13], b"hello"); // data payload
    }
}
