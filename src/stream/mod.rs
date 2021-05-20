use crate::traits;
use crate::wire;

#[derive(Debug, Clone)]
pub enum Error {
    EOF,
    InvalidInvoke,
    UnknownPacketKind(wire::packet::Kind),
    RemoteError(Vec<u8>),
    RemoteClosed,
    SendClosed,
    TerminatedBothClosed,
    TerminatedSentClose,
    TerminatedSentError,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

pub struct Stream<'a> {
    id: wire::id::ID,
    tr: wire::transport::Transport<'a>,

    send: Option<Error>,
    recv: Option<Error>,
    term: Option<Error>,
}

fn set_once<T>(opt: &mut Option<T>, val: T) {
    match *opt {
        Some(_) => return,
        None => *opt = Some(val),
    }
}

fn check_optional(opt: &Option<Error>) -> traits::Result<()> {
    if let Some(err) = &opt {
        Err(Box::new(err.clone()))
    } else {
        Ok(())
    }
}

impl<'a> Stream<'a> {
    pub fn new(sid: u64, tr: wire::transport::Transport<'a>) -> Self {
        Stream {
            id: wire::id::ID {
                stream: sid,
                message: 0,
            },
            tr: tr,

            send: None,
            recv: None,
            term: None,
        }
    }

    pub fn transport(self: &mut Self) -> &mut wire::transport::Transport<'a> {
        &mut self.tr
    }

    pub fn reset(self: &mut Self, sid: u64) {
        self.id.stream = sid;
        self.id.message = 0;
        self.send = None;
        self.recv = None;
        self.term = None;
    }

    fn new_packet(
        self: &mut Self,
        kind: wire::packet::Kind,
        data: Vec<u8>,
    ) -> wire::packet::Packet {
        self.id.message += 1;
        wire::packet::Packet {
            data: data,
            id: self.id,
            kind: kind,
        }
    }

    //

    pub fn raw_write(
        self: &mut Self,
        kind: wire::packet::Kind,
        buf: Vec<u8>,
    ) -> traits::Result<()> {
        let pkt = self.new_packet(kind, buf);
        for fr in wire::split::split(&pkt, 1024) {
            self.tr.write_frame(fr)?
        }
        Ok(())
    }

    pub fn raw_flush(self: &mut Self) -> traits::Result<()> {
        self.tr.flush()
    }

    //

    fn send_set_err(self: &mut Self, err: Error) {
        set_once(&mut self.send, err)
    }
    fn recv_set_err(self: &mut Self, err: Error) {
        set_once(&mut self.recv, err)
    }
    fn term_set_err(self: &mut Self, err: Error) {
        set_once(&mut self.term, err)
    }

    //

    fn send_closed(self: &Self) -> traits::Result<()> {
        check_optional(&self.send)
    }
    fn recv_closed(self: &Self) -> traits::Result<()> {
        check_optional(&self.recv)
    }
    fn terminated(self: &Self) -> traits::Result<()> {
        check_optional(&self.term)
    }

    fn terminate_if_both_closed(self: &mut Self) {
        if self.send.is_some() && self.recv.is_some() {
            self.term_set_err(Error::TerminatedBothClosed)
        }
    }
}

impl<'a, In, Out> traits::Stream<In, Out> for Stream<'a> {
    fn close_send(self: &mut Self) -> traits::Result<()> {
        if self.send.is_some() || self.term.is_some() {
            return Ok(());
        }

        self.send_set_err(Error::SendClosed);
        self.terminate_if_both_closed();
        self.raw_write(wire::packet::Kind::CloseSend, Vec::new())?;
        self.raw_flush()
    }

    fn close(self: &mut Self) -> traits::Result<()> {
        if self.term.is_some() {
            return Ok(());
        }

        self.term_set_err(Error::TerminatedSentClose);
        self.raw_write(wire::packet::Kind::Close, Vec::new())?;
        self.raw_flush()
    }

    fn error(self: &mut Self, msg: &str) -> traits::Result<()> {
        if self.term.is_some() {
            return Ok(());
        }

        let mut buf = Vec::with_capacity(8 + msg.len());
        buf.extend_from_slice(&[0; 8]);
        buf.extend_from_slice(msg.as_bytes());

        self.send_set_err(Error::EOF);
        self.term_set_err(Error::TerminatedSentError);
        self.raw_write(wire::packet::Kind::Error, buf)?;
        self.raw_flush()
    }

    //

    fn send(
        self: &mut Self,
        enc: &dyn traits::Encoding<In, Out>,
        input: &In,
    ) -> traits::Result<()> {
        self.send_closed()?;
        self.terminated()?;

        let buf = enc.marshal(input)?;
        self.raw_write(wire::packet::Kind::Message, buf)?;
        self.raw_flush()?;
        Ok(())
    }

    fn recv(self: &mut Self, enc: &dyn traits::Encoding<In, Out>) -> traits::Result<Out> {
        loop {
            self.recv_closed()?;
            self.terminated()?;

            let pkt = self.tr.read()?;

            if pkt.id.stream != self.id.stream {
                continue;
            }

            match pkt.kind {
                wire::packet::Kind::Message => {
                    return enc.unmarshal(&pkt.data);
                }
                wire::packet::Kind::Invoke => {
                    self.term_set_err(Error::InvalidInvoke);
                }
                wire::packet::Kind::Error => {
                    self.send_set_err(Error::EOF);
                    self.term_set_err(Error::RemoteError(pkt.data.clone()));
                }
                wire::packet::Kind::Close => {
                    self.recv_set_err(Error::EOF);
                    self.term_set_err(Error::RemoteClosed);
                }
                wire::packet::Kind::CloseSend => {
                    self.recv_set_err(Error::EOF);
                    self.terminate_if_both_closed();
                }
                other => {
                    self.term_set_err(Error::UnknownPacketKind(other));
                }
            }
        }
    }
}
