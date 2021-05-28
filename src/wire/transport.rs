use crate::traits;
use crate::traits::*;
use crate::wire::frame;
use crate::wire::id;
use crate::wire::packet;

//

#[derive(Debug)]
pub enum Error {
    ReadError(traits::Error),
    ParseError,
    IDMonotonicityError,
    PacketKindChangeError,
    DataOverflowError,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

//

pub struct Transport<'a, Tr: traits::Transport> {
    tr: &'a mut Tr,

    id: id::ID,
    tmp: Vec<u8>,

    buf: Vec<u8>,
    size: usize,
}

impl<'a, Tr: traits::Transport> Transport<'a, Tr> {
    pub fn new(tr: &'a mut Tr, size: usize) -> Transport<'a, Tr> {
        Transport {
            tr: tr,

            id: id::ID::default(),
            tmp: vec![0; 4096],

            buf: Vec::with_capacity(size),
            size: size,
        }
    }

    pub fn transport(self: &mut Self) -> &mut Tr {
        self.tr
    }

    pub fn read(self: &mut Self) -> Result<packet::Packet<Vec<u8>>> {
        let mut buf = Vec::new();
        let mut parsed = 0;

        let mut pkt = packet::Packet::<Vec<u8>>::default();

        loop {
            if parsed > 0 {
                buf.drain(0..parsed);
                parsed = 0;
            }

            if buf.capacity() > (4 << 20 + 1 + 9 + 9 + 9) {
                return Err(Box::new(Error::DataOverflowError));
            }

            match frame::parse_frame(&buf) {
                Ok((fr, read)) => {
                    parsed = read;

                    if fr.control {
                        continue;
                    } else if fr.id < self.id {
                        return Err(Box::new(Error::IDMonotonicityError));
                    } else if self.id < fr.id {
                        self.id = fr.id;
                        pkt.data.clear();
                        pkt.id = fr.id;
                        pkt.kind = fr.kind.into();
                    } else if pkt.kind != fr.kind.into() {
                        return Err(Box::new(Error::PacketKindChangeError));
                    }

                    pkt.data.extend_from_slice(fr.data);

                    if pkt.data.len() > (4 << 20) {
                        return Err(Box::new(Error::DataOverflowError));
                    } else if fr.done {
                        return Ok(pkt);
                    }
                }

                Err(frame::Error::NotEnoughData) => {
                    let n = self.tr.read(&mut self.tmp)?;
                    buf.extend_from_slice(&self.tmp[0..n]);
                }

                Err(frame::Error::ParseError) => return Err(Box::new(Error::ParseError)),
            }
        }
    }

    pub fn reset(self: &mut Self) {
        self.buf.clear();
    }

    pub fn write_packet<D>(self: &mut Self, pkt: packet::Packet<D>) -> Result<()>
    where
        D: std::borrow::Borrow<[u8]>,
    {
        self.write_frame(frame::Frame {
            data: pkt.data.borrow(),
            id: pkt.id,
            kind: pkt.kind.into(),
            done: true,
            control: false,
        })
    }

    pub fn write_frame(self: &mut Self, fr: frame::Frame) -> Result<()> {
        frame::append_frame(&mut self.buf, &fr);

        if self.buf.len() >= self.size {
            let result = self.tr.write(&self.buf);
            self.buf.clear();
            result?;
            return Ok(());
        }

        Ok(())
    }

    pub fn flush(self: &mut Self) -> Result<()> {
        if self.buf.len() > 0 {
            let result = self.tr.write(&self.buf);
            self.buf.clear();
            result?;
            return Ok(());
        }

        Ok(())
    }
}

mod test {
    use crate::utils::buffer;
    use crate::wire::frame;
    use crate::wire::id;
    use crate::wire::packet;

    #[test]
    fn test_read_packets() {
        let mut buf =
            buffer::Buffer::from(vec![4, 1, 1, 5, 1, 2, 3, 4, 5, 5, 1, 1, 5, 6, 7, 8, 9, 10]);
        let mut r = super::Transport::new(&mut buf, 1024);

        assert_eq!(
            r.read().unwrap(),
            packet::Packet {
                data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                id: id::ID {
                    stream: 1,
                    message: 1,
                },
                kind: packet::Kind::Message,
            }
        );
    }

    #[test]
    fn test_write_packet() {
        let mut buf = buffer::Buffer::new();

        {
            let mut w = super::Transport::new(&mut buf, 5);
            w.write_packet(packet::Packet {
                data: vec![1, 2, 3, 4, 5],
                id: id::ID::default(),
                kind: packet::Kind::Message,
            })
            .unwrap();
        }

        assert_eq!(buf.take_contents(), &[5, 0, 0, 5, 1, 2, 3, 4, 5])
    }

    #[test]
    fn test_write_frame() {
        let mut buf = buffer::Buffer::new();

        {
            let mut w = super::Transport::new(&mut buf, 5);
            w.write_frame(frame::Frame {
                data: &[1, 2, 3, 4, 5],
                id: id::ID::default(),
                kind: 8,
                done: true,
                control: false,
            })
            .unwrap();
        }

        assert_eq!(buf.take_contents(), &[17, 0, 0, 5, 1, 2, 3, 4, 5])
    }

    #[test]
    fn test_buffering() {
        let mut buf = buffer::Buffer::new();

        {
            let mut w = super::Transport::new(&mut buf, 50);
            w.write_packet(packet::Packet {
                data: vec![1, 2, 3, 4, 5],
                id: id::ID::default(),
                kind: packet::Kind::Message,
            })
            .unwrap();
        }

        assert_eq!(buf.take_contents(), &[])
    }

    #[test]
    fn test_flush() {
        let mut buf = buffer::Buffer::new();

        {
            let mut w = super::Transport::new(&mut buf, 50);
            w.write_packet(packet::Packet {
                data: vec![1, 2, 3, 4, 5],
                id: id::ID::default(),
                kind: packet::Kind::Message,
            })
            .unwrap();
            w.flush().unwrap();
        }

        assert_eq!(buf.take_contents(), &[5, 0, 0, 5, 1, 2, 3, 4, 5])
    }

    #[test]
    fn test_write_multiple_frame() {
        let mut buf = buffer::Buffer::new();

        {
            let mut w = super::Transport::new(&mut buf, 5);
            w.write_frame(frame::Frame {
                data: &[1, 2, 3, 4, 5],
                id: id::ID {
                    stream: 1,
                    message: 1,
                },
                kind: 2,
                done: false,
                control: false,
            })
            .unwrap();
            w.write_frame(frame::Frame {
                data: &[6, 7, 8, 9, 10],
                id: id::ID {
                    stream: 1,
                    message: 1,
                },
                kind: 2,
                done: true,
                control: false,
            })
            .unwrap();
        }

        assert_eq!(
            buf.take_contents(),
            &[4, 1, 1, 5, 1, 2, 3, 4, 5, 5, 1, 1, 5, 6, 7, 8, 9, 10]
        )
    }
}
