use crate::{
    enc, transport,
    wire::{self, id, packet},
};
use core::marker::PhantomData;
use std::convert::TryInto;

#[derive(Debug, Clone)]
pub enum State {
    EOF,
    InvalidInvoke,
    UnknownPacketKind(packet::Kind),
    RemoteError((u64, String)),
    RemoteClosed,
    SendClosed,
    TerminatedBothClosed,
    TerminatedSentClose,
    TerminatedSentError,
}

fn parse_remote_error(buf: Vec<u8>) -> State {
    if buf.len() < 8 {
        return State::RemoteError((0, String::from("invalid error message")));
    }
    let (prefix, message) = buf.split_at(8);
    let code = u64::from_be_bytes(prefix.try_into().unwrap());
    State::RemoteError((code, String::from_utf8_lossy(message).to_string()))
}

//

#[derive(Debug)]
pub enum Error {
    StateError(State),
    TransportError(transport::Error),
    IOError(std::io::Error),
    EncodingError(enc::Error),
    AnyError(Box<dyn std::error::Error + Send>),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl From<State> for Error {
    fn from(err: State) -> Error {
        Error::StateError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IOError(err)
    }
}

impl From<transport::Error> for Error {
    fn from(err: transport::Error) -> Error {
        Error::TransportError(err)
    }
}

impl From<enc::Error> for Error {
    fn from(err: enc::Error) -> Error {
        Error::EncodingError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

//

trait SetOnce<T> {
    fn set_once(&mut self, t: T);
}

impl<T> SetOnce<T> for Option<T> {
    fn set_once(&mut self, t: T) {
        if let None = self {
            *self = Some(t)
        }
    }
}

trait AsError<E> {
    fn as_error(&self) -> std::result::Result<(), E>;
}

impl<E> AsError<E> for Option<E>
where
    E: Clone,
{
    fn as_error(&self) -> std::result::Result<(), E> {
        match self {
            Some(err) => Err(err.clone()),
            None => Ok(()),
        }
    }
}

// generic stream

pub struct GenericStream<'a> {
    id: id::ID,
    tr: &'a mut dyn transport::Stream,
    buf: &'a mut Vec<u8>,

    send: Option<State>,
    recv: Option<State>,
    term: Option<State>,
}

impl<'a> GenericStream<'a> {
    pub fn new(sid: u64, tr: &'a mut dyn transport::Stream, buf: &'a mut Vec<u8>) -> Self {
        GenericStream {
            id: id::ID::new(sid, 0),
            tr,
            buf,

            send: None,
            recv: None,
            term: None,
        }
    }

    pub fn transport<'tr>(&'tr mut self) -> &'tr mut dyn transport::Stream {
        self.tr
    }

    pub fn id(&self) -> u64 {
        self.id.stream
    }

    //

    async fn write_buf(&mut self, kind: packet::Kind) -> Result<()> {
        self.id.message += 1;

        let pkt = packet::Packet::<&[u8]> {
            data: self.buf,
            id: self.id,
            kind,
        };

        for fr in wire::split::split(&pkt, 64 * 1024) {
            self.tr.write_frame(fr).await?;
        }

        Ok(())
    }

    async fn recv_buf(&mut self) -> Result<()> {
        loop {
            self.recv.as_error()?;
            self.term.as_error()?;

            let id;
            let kind;

            match self.tr.read_packet_into(&mut self.buf).await {
                Ok((id_, kind_)) => {
                    id = id_;
                    kind = kind_
                }

                Err(transport::Error::RemoteClosed) => {
                    self.recv.set_once(State::EOF);
                    self.term.set_once(State::RemoteClosed);
                    continue;
                }

                Err(e) => {
                    return Err(Error::TransportError(e));
                }
            };

            if id.stream != self.id.stream {
                continue;
            }

            match kind {
                packet::Kind::Message => {
                    return Ok(());
                }

                packet::Kind::Invoke => {
                    self.term.set_once(State::InvalidInvoke);
                }

                packet::Kind::Error => {
                    self.send.set_once(State::EOF);
                    let state = parse_remote_error(self.buf.clone());
                    self.term.set_once(state);
                }

                packet::Kind::Close => {
                    self.recv.set_once(State::EOF);
                    self.term.set_once(State::RemoteClosed);
                }

                packet::Kind::CloseSend => {
                    self.recv.set_once(State::EOF);
                    self.terminate_if_both_closed();
                }

                other => {
                    self.term.set_once(State::UnknownPacketKind(other));
                }
            }
        }
    }

    //

    fn terminate_if_both_closed(&mut self) {
        if self.send.is_some() && self.recv.is_some() {
            self.term.set_once(State::TerminatedBothClosed)
        }
    }

    //

    pub async fn invoke(&mut self, rpc: &[u8]) -> Result<()> {
        self.buf.clear();
        self.buf.extend_from_slice(rpc);
        self.write_buf(packet::Kind::Invoke).await
    }

    pub async fn close_send(&mut self) -> Result<()> {
        if self.send.is_some() || self.term.is_some() {
            return Ok(());
        }

        self.send.set_once(State::SendClosed);
        self.terminate_if_both_closed();

        self.buf.clear();
        self.write_buf(packet::Kind::CloseSend).await?;
        self.tr.flush().await?;
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        if self.term.is_some() {
            return Ok(());
        }

        self.term.set_once(State::TerminatedSentClose);

        self.buf.clear();
        self.write_buf(packet::Kind::Close).await?;
        self.tr.flush().await?;
        Ok(())
    }

    pub async fn error(&mut self, msg: &str, code: u64) -> Result<()> {
        if self.term.is_some() {
            return Ok(());
        }

        self.send.set_once(State::EOF);
        self.term.set_once(State::TerminatedSentError);

        self.buf.clear();
        self.buf.reserve(8 + msg.len());
        self.buf.extend_from_slice(&code.to_be_bytes());
        self.buf.extend_from_slice(msg.as_bytes());
        self.write_buf(packet::Kind::Error).await?;
        self.tr.flush().await?;
        Ok(())
    }

    pub async fn send<Enc: enc::Marshal<In>, In>(&mut self, input: &In) -> Result<()> {
        self.send.as_error()?;
        self.term.as_error()?;

        Enc::marshal(input, &mut *self.buf)?;
        self.write_buf(packet::Kind::Message).await
    }

    pub async fn recv_into<Enc: enc::Unmarshal<Out>, Out>(&mut self, out: &mut Out) -> Result<()> {
        self.recv.as_error()?;
        self.term.as_error()?;

        self.tr.flush().await?;
        self.recv_buf().await?;
        Enc::unmarshal(&self.buf, out)?;
        Ok(())
    }

    pub async fn recv<Enc: enc::Unmarshal<Out>, Out: Default>(&mut self) -> Result<Out> {
        let mut out = Default::default();
        self.recv_into::<Enc, Out>(&mut out).await?;
        Ok(out)
    }

    pub fn fix<Enc, In, Out>(self) -> Stream<'a, Enc, In, Out> {
        Stream {
            g: self,
            _enc: PhantomData,
            _in: PhantomData,
            _out: PhantomData,
        }
    }
}

pub struct Stream<'a, Enc, In, Out> {
    g: GenericStream<'a>,
    _enc: PhantomData<Enc>,
    _in: PhantomData<In>,
    _out: PhantomData<Out>,
}

impl<'a, Enc, In, Out> Stream<'a, Enc, In, Out> {
    pub fn transport<'tr>(&'tr mut self) -> &'tr mut dyn transport::Stream {
        self.g.transport()
    }

    pub fn id(&self) -> u64 {
        self.g.id()
    }

    pub async fn invoke(&mut self, rpc: &[u8]) -> Result<()> {
        self.g.invoke(rpc).await
    }

    pub async fn close_send(&mut self) -> Result<()> {
        self.g.close_send().await
    }

    pub async fn close(&mut self) -> Result<()> {
        self.g.close().await
    }

    pub async fn error(&mut self, msg: &str, code: u64) -> Result<()> {
        self.g.error(msg, code).await
    }
}

impl<'a, Enc: enc::Marshal<In> + Send, In, Out> Stream<'a, Enc, In, Out> {
    pub async fn send(&mut self, input: &In) -> Result<()> {
        self.g.send::<Enc, In>(input).await
    }
}

impl<'a, Enc: enc::Unmarshal<Out>, In, Out> Stream<'a, Enc, In, Out> {
    pub async fn recv_into(&mut self, out: &mut Out) -> Result<()> {
        self.g.recv_into::<Enc, Out>(out).await
    }
}

impl<'a, Enc: enc::Unmarshal<Out>, In, Out: Default> Stream<'a, Enc, In, Out> {
    pub async fn recv(&mut self) -> Result<Out> {
        self.g.recv::<Enc, Out>().await
    }
}
