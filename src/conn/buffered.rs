use crate::stream;
use crate::wire::{frame, id, packet};
use std::io::{Read,Write};

// error

#[derive(Debug, Copy, Clone)]
pub enum Error {
    ParseError,
    IDMonotonicityError,
    PacketKindChangeError,
    DataOverflowError,
    OperationFailedError,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

// wrapper

pub struct Transport<'a> {
    tr: &'a mut dyn crate::conn::Transport,
    wbuf: Vec<u8>,
    rbuf: Vec<u8>,
    err: Option<Error>,
}

impl<'a> Transport<'a> {
    pub fn new(tr: &'a mut dyn crate::conn::Transport) -> Transport<'a> {
        Transport {
            tr,
            wbuf: Vec::new(),
            rbuf: Vec::new(),
            err: None,
        }
    }

    fn errored(&self) -> Result<(), Error> {
        match self.err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn set_errored<T>(&mut self, e: std::io::Error) -> crate::Result<T> {
        self.err = Some(Error::OperationFailedError);
        Err(Box::new(e))
    }

    fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        self.errored()?;
        match self.tr.read(buf) {
            Ok(v) => Ok(v),
            Err(e) => self.set_errored(e),
        }
    }

    fn flush(&mut self) -> crate::Result<()> {
        self.errored()?;
        match self.tr.write(&self.wbuf).and_then(|_| self.tr.flush()) {
            Ok(v) => Ok(v),
            Err(e) => self.set_errored(e),
        }
    }
}

impl<'a> stream::Transport for Transport<'a> {
    fn read_packet_into(&mut self, buf: &mut Vec<u8>) -> crate::Result<(id::ID, packet::Kind)> {
        self.errored()?;

        let mut tmp = [0; 4096];
        let mut parsed = 0;

        self.rbuf.clear();
        buf.clear();
        let mut id = id::ID::default();
        let mut kind = packet::Kind::default();

        loop {
            if parsed > 0 {
                self.rbuf.drain(0..parsed);
                parsed = 0;
            }

            if self.rbuf.len() > (4 << 20 + 1 + 9 + 9 + 9) {
                return Err(Box::new(Error::DataOverflowError));
            }

            match frame::parse_frame(&self.rbuf) {
                Ok((fr, read)) => {
                    parsed = read;

                    if fr.control {
                        continue;
                    } else if fr.id < id {
                        return Err(Box::new(Error::IDMonotonicityError));
                    } else if id < fr.id {
                        buf.clear();
                        id = fr.id;
                        kind = fr.kind.into();
                    } else if kind != fr.kind.into() {
                        return Err(Box::new(Error::PacketKindChangeError));
                    }

                    buf.extend_from_slice(fr.data);

                    if buf.len() > (4 << 20) {
                        return Err(Box::new(Error::DataOverflowError));
                    } else if fr.done {
                        return Ok((id, kind));
                    }
                }

                Err(frame::Error::NotEnoughData) => {
                    // TODO: can we do this read directly into spare vector capacity?
                    let n = self.read(&mut tmp)?;
                    self.rbuf.extend_from_slice(&tmp[0..n]);
                }

                Err(frame::Error::ParseError) => return Err(Box::new(Error::ParseError)),
            }
        }
    }

    fn write_frame(&mut self, fr: frame::Frame) -> crate::Result<()> {
        self.errored()?;

        frame::append_frame(&mut self.wbuf, &fr);
        if self.wbuf.len() >= 8 * 1024 {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> crate::Result<()> {
        self.errored()?;

        if self.wbuf.len() > 0 {
            let res = self.flush();
            self.wbuf.clear();
            res?
        }

        Ok(())
    }
}
