use crate::wire::{frame, id, packet};

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// error

#[derive(Debug, Copy, Clone)]
pub enum Error {
    RemoteClosed,
    ParseError,
    IDMonotonicityError,
    PacketKindChangeError,
    DataOverflowError,
    IOError,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

// transport

pub struct Transport<W> {
    w: W,
    wbuf: Vec<u8>,
    rbuf: Vec<u8>,
    err: Result<()>,
}

impl<W: crate::Wire> Transport<W> {
    pub fn new(w: W) -> Transport<W> {
        Transport {
            w,
            wbuf: Vec::new(),
            rbuf: Vec::new(),
            err: Ok(()),
        }
    }

    fn set_errored<V>(&mut self) -> Result<V> {
        self.err = Err(Error::IOError);
        Err(Error::IOError)
    }

    async fn raw_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.err?;
        match self.w.read(buf).await {
            Ok(v) => Ok(v),
            Err(_) => self.set_errored(),
        }
    }

    async fn raw_flush(&mut self) -> Result<()> {
        self.err?;
        match self.w.write(&self.wbuf).await {
            Err(_) => self.set_errored(),
            Ok(_) => match self.w.flush().await {
                Err(_) => self.set_errored(),
                Ok(v) => Ok(v),
            },
        }
    }
}

#[async_trait]
impl<W: crate::Wire> crate::Transport for Transport<W> {
    fn wire(&mut self) -> &mut dyn crate::Wire {
        &mut self.w
    }

    async fn read_packet_into(&mut self, buf: &mut Vec<u8>) -> Result<(id::ID, packet::Kind)> {
        self.err?;

        let mut tmp = [0; 4096];
        let mut parsed = 0;

        buf.clear();
        let mut id = id::ID::default();
        let mut kind = packet::Kind::default();

        loop {
            if parsed > 0 {
                self.rbuf.drain(0..parsed);
                parsed = 0;
            }

            if self.rbuf.len() > (4 << 20 + 1 + 9 + 9 + 9) {
                return Err(Error::DataOverflowError);
            }

            match frame::parse_frame(&self.rbuf) {
                Ok((fr, read)) => {
                    parsed = read;

                    if fr.control {
                        continue;
                    } else if fr.id < id {
                        return Err(Error::IDMonotonicityError);
                    } else if id < fr.id {
                        buf.clear();
                        id = fr.id;
                        kind = fr.kind.into();
                    } else if kind != fr.kind.into() {
                        return Err(Error::PacketKindChangeError);
                    }

                    buf.extend_from_slice(fr.data);

                    if buf.len() > (4 << 20) {
                        return Err(Error::DataOverflowError);
                    } else if fr.done {
                        self.rbuf.drain(0..parsed);
                        return Ok((id, kind));
                    }
                }

                Err(frame::Error::NotEnoughData) => {
                    // TODO: can we do this read directly into spare vector capacity?
                    let n = self.raw_read(&mut tmp).await?;
                    if n == 0 {
                        return Err(Error::RemoteClosed);
                    }
                    self.rbuf.extend_from_slice(&tmp[0..n]);
                }

                Err(frame::Error::ParseError) => return Err(Error::ParseError),
            }
        }
    }

    async fn write_frame(&mut self, fr: frame::Frame<'_>) -> Result<()> {
        self.err?;

        frame::append_frame(&mut self.wbuf, &fr);
        if self.wbuf.len() >= 64 * 1024 {
            self.flush().await?;
        }

        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        self.err?;

        if self.wbuf.len() > 0 {
            let res = self.raw_flush().await;
            self.wbuf.clear();
            res?
        }

        Ok(())
    }
}
