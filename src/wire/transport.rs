use crate::stream;
use crate::wire::{frame, id, packet};

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

// wrapper

pub struct Transport<'a> {
    tr: &'a mut dyn crate::wire::Transport,
    wbuf: Vec<u8>,
    rbuf: Vec<u8>,
    err: Result<()>,
}

impl<'a> Transport<'a> {
    pub fn new(tr: &'a mut dyn crate::wire::Transport) -> Transport<'a> {
        Transport {
            tr,
            wbuf: Vec::new(),
            rbuf: Vec::new(),
            err: Ok(()),
        }
    }

    fn set_errored<T>(&mut self) -> Result<T> {
        self.err = Err(Error::IOError);
        Err(Error::IOError)
    }

    fn raw_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.err?;
        match self.tr.read(buf) {
            Ok(v) => Ok(v),
            Err(_) => self.set_errored(),
        }
    }

    fn raw_flush(&mut self) -> Result<()> {
        self.err?;
        match self.tr.write(&self.wbuf).and_then(|_| self.tr.flush()) {
            Ok(v) => Ok(v),
            Err(_) => self.set_errored(),
        }
    }
}

impl<'a> stream::Transport for Transport<'a> {
    fn read_packet_into(&mut self, buf: &mut Vec<u8>) -> Result<(id::ID, packet::Kind)> {
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
                    let n = self.raw_read(&mut tmp)?;
                    if n == 0 {
                        return Err(Error::RemoteClosed);
                    }
                    self.rbuf.extend_from_slice(&tmp[0..n]);
                }

                Err(frame::Error::ParseError) => return Err(Error::ParseError),
            }
        }
    }

    fn write_frame(&mut self, fr: frame::Frame) -> Result<()> {
        self.err?;

        frame::append_frame(&mut self.wbuf, &fr);
        if self.wbuf.len() >= 64 * 1024 {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.err?;

        if self.wbuf.len() > 0 {
            let res = self.raw_flush();
            self.wbuf.clear();
            res?
        }

        Ok(())
    }
}
