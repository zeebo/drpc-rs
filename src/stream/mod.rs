use packet::Kind::Message;

use crate::wire::{frame, id, packet, split};
use crate::{enc, wire};
use std::io::Read;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub enum Error {
    EOF,
    InvalidInvoke,
    UnknownPacketKind(packet::Kind),
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

//

fn set_once<T>(opt: &mut Option<T>, val: T) {
    match *opt {
        Some(_) => return,
        None => *opt = Some(val),
    }
}

fn check_optional(opt: &Option<Error>) -> crate::Result<()> {
    match opt {
        Some(err) => Err(Box::new(err.clone())),
        None => Ok(()),
    }
}

// transport

pub trait Transport {
    fn read_packet_into(&mut self, buf: &mut Vec<u8>) -> crate::Result<(id::ID, packet::Kind)>;
    fn write_frame(&mut self, fr: frame::Frame) -> crate::Result<()>;
    fn flush(&mut self) -> crate::Result<()>;
}

// stream

pub struct Stream<'a, Enc, In, Out> {
    id: id::ID,
    tr: &'a mut dyn Transport,
    buf: &'a mut Vec<u8>,

    send: Option<Error>,
    recv: Option<Error>,
    term: Option<Error>,

    _enc: PhantomData<*const Enc>,
    _in: PhantomData<*const In>,
    _out: PhantomData<*const Out>,
}

impl<'a, Enc, In, Out> Stream<'a, Enc, In, Out> {
    pub fn new(sid: u64, tr: &'a mut dyn Transport, buf: &'a mut Vec<u8>) -> Self {
        Stream {
            id: id::ID::new(sid, 0),
            tr,
            buf,

            send: None,
            recv: None,
            term: None,

            _enc: PhantomData,
            _in: PhantomData,
            _out: PhantomData,
        }
    }

    //

    fn write_buf(&mut self, kind: packet::Kind) -> crate::Result<()> {
        self.id.message += 1;

        let pkt = packet::Packet::<&[u8]> {
            data: self.buf,
            id: self.id,
            kind,
        };

        for fr in wire::split::split(&pkt, 1024) {
            self.tr.write_frame(fr)?;
        }

        Ok(())
    }

    fn recv_buf(&mut self) -> crate::Result<()> {
        loop {
            let (id, kind) = self.tr.read_packet_into(&mut self.buf)?;
            if id.stream != self.id.stream {
                continue;
            }

            match kind {
                Message => return Ok(()),
                packet::Kind::Invoke => self.term_set_err(Error::InvalidInvoke),
                packet::Kind::Error => {
                    self.send_set_err(Error::EOF);
                    self.term_set_err(Error::RemoteError(self.buf.clone()));
                }
                packet::Kind::Close => {
                    self.recv_set_err(Error::EOF);
                    self.term_set_err(Error::RemoteClosed);
                }
                packet::Kind::CloseSend => {
                    self.recv_set_err(Error::EOF);
                    self.terminate_if_both_closed();
                }
                other => self.term_set_err(Error::UnknownPacketKind(other)),
            }
        }
    }

    //

    fn send_set_err(&mut self, err: Error) {
        set_once(&mut self.send, err)
    }
    fn recv_set_err(&mut self, err: Error) {
        set_once(&mut self.recv, err)
    }
    fn term_set_err(&mut self, err: Error) {
        set_once(&mut self.term, err)
    }

    //

    fn send_closed(self: &Self) -> crate::Result<()> {
        check_optional(&self.send)
    }
    fn recv_closed(self: &Self) -> crate::Result<()> {
        check_optional(&self.recv)
    }
    fn terminated(self: &Self) -> crate::Result<()> {
        check_optional(&self.term)
    }

    fn terminate_if_both_closed(&mut self) {
        if self.send.is_some() && self.recv.is_some() {
            self.term_set_err(Error::TerminatedBothClosed)
        }
    }

    //

    pub fn invoke(&mut self, rpc: &str) -> crate::Result<()> {
        self.buf.clear();
        self.buf.extend_from_slice(rpc.as_bytes());
        self.write_buf(packet::Kind::Invoke)
    }

    pub fn close_send(&mut self) -> crate::Result<()> {
        if self.send.is_some() || self.term.is_some() {
            return Ok(());
        }

        self.send_set_err(Error::SendClosed);
        self.terminate_if_both_closed();

        self.buf.clear();
        self.write_buf(packet::Kind::CloseSend)?;
        self.tr.flush()
    }

    pub fn close(&mut self) -> crate::Result<()> {
        if self.term.is_some() {
            return Ok(());
        }

        self.term_set_err(Error::TerminatedSentClose);

        self.buf.clear();
        self.write_buf(packet::Kind::Close)?;
        self.tr.flush()
    }

    pub fn error(&mut self, msg: &str) -> crate::Result<()> {
        if self.term.is_some() {
            return Ok(());
        }

        self.send_set_err(Error::EOF);
        self.term_set_err(Error::TerminatedSentError);

        self.buf.clear();
        self.buf.reserve(8 + msg.len());
        self.buf.extend_from_slice(&[0; 8]);
        self.buf.extend_from_slice(msg.as_bytes());
        self.write_buf(packet::Kind::Error)?;
        self.tr.flush()
    }
}

impl<'a, Enc: enc::Marshal<In>, In, Out> Stream<'a, Enc, In, Out> {
    pub fn send(&mut self, input: &In) -> crate::Result<()> {
        self.send_closed()?;
        self.terminated()?;

        Enc::marshal(input, &mut *self.buf)?;
        self.write_buf(Message)
    }
}

impl<'a, Enc: enc::Unmarshal<Out>, In, Out> Stream<'a, Enc, In, Out> {
    pub fn recv_into(&mut self, out: &mut Out) -> crate::Result<()> {
        self.recv_closed()?;
        self.terminated()?;

        self.tr.flush()?;
        self.recv_buf()?;
        Enc::unmarshal(&self.buf, out)
    }
}

impl<'a, Enc: enc::Unmarshal<Out>, In, Out: Default> Stream<'a, Enc, In, Out> {
    pub fn recv(&mut self) -> crate::Result<Out> {
        self.recv_closed()?;
        self.terminated()?;

        let mut out = Default::default();
        self.recv_into(&mut out)?;
        Ok(out)
    }
}
