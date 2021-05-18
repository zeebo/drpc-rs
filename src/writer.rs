use super::frame;
use super::packet;
use super::transport;

type Transport<'a, E> = Box<dyn transport::Transport<Error = E> + 'a>;

struct Writer<'a, E> {
    tr: Transport<'a, E>,
    buf: Vec<u8>,
    size: usize,
}

impl<'a, E> Writer<'a, E> {
    fn new(tr: Transport<'a, E>, size: usize) -> Writer<'a, E> {
        Writer {
            tr: tr,
            buf: Vec::with_capacity(size),
            size: size,
        }
    }

    fn reset(self: &mut Self) {
        self.buf.clear();
    }

    fn write_packet(self: &mut Self, pkt: packet::Packet) -> Option<E> {
        self.write_frame(frame::Frame {
            data: &pkt.data,
            id: pkt.id,
            kind: pkt.kind.into(),
            done: true,
            control: false,
        })
    }

    fn write_frame(self: &mut Self, fr: frame::Frame) -> Option<E> {
        frame::append_frame(&mut self.buf, &fr);

        if self.buf.len() >= self.size {
            let result = self.tr.write(&self.buf);
            self.buf.clear();
            return result;
        }

        None
    }

    fn flush(self: &mut Self) -> Option<E> {
        if self.buf.len() > 0 {
            let result = self.tr.write(&self.buf);
            self.buf.clear();
            return result;
        }

        None
    }
}

mod test {
    use crate::frame;
    use crate::id;
    use crate::packet;
    use crate::transport;

    #[test]
    fn test_write_packet() {
        let mut buf = Vec::new();

        {
            let mut w = super::Writer::new(Box::new(&mut buf), 5);
            w.write_packet(packet::Packet {
                data: vec![1, 2, 3, 4, 5],
                id: id::ID::default(),
                kind: packet::Kind::Message,
            });
        }

        assert_eq!(buf, &[5, 0, 0, 5, 1, 2, 3, 4, 5])
    }

    #[test]
    fn test_write_frame() {
        let mut buf = Vec::new();

        {
            let mut w = super::Writer::new(Box::new(&mut buf), 5);
            w.write_frame(frame::Frame {
                data: &[1, 2, 3, 4, 5],
                id: id::ID::default(),
                kind: 8,
                done: true,
                control: false,
            });
        }

        assert_eq!(buf, &[17, 0, 0, 5, 1, 2, 3, 4, 5])
    }

    #[test]
    fn test_buffering() {
        let mut buf = Vec::new();

        {
            let mut w = super::Writer::new(Box::new(&mut buf), 50);
            w.write_packet(packet::Packet {
                data: vec![1, 2, 3, 4, 5],
                id: id::ID::default(),
                kind: packet::Kind::Message,
            });
        }

        assert_eq!(buf, &[])
    }

    #[test]
    fn test_flush() {
        let mut buf = Vec::new();

        {
            let mut w = super::Writer::new(Box::new(&mut buf), 50);
            w.write_packet(packet::Packet {
                data: vec![1, 2, 3, 4, 5],
                id: id::ID::default(),
                kind: packet::Kind::Message,
            });
            w.flush();
        }

        assert_eq!(buf, &[5, 0, 0, 5, 1, 2, 3, 4, 5])
    }

    #[test]
    fn test_write_multiple_frame() {
        let mut buf = Vec::new();

        {
            let mut w = super::Writer::new(Box::new(&mut buf), 5);
            w.write_frame(frame::Frame {
                data: &[1, 2, 3, 4, 5],
                id: id::ID {
                    stream: 1,
                    message: 1,
                },
                kind: 2,
                done: false,
                control: false,
            });
            w.write_frame(frame::Frame {
                data: &[6, 7, 8, 9, 10],
                id: id::ID {
                    stream: 1,
                    message: 1,
                },
                kind: 2,
                done: true,
                control: false,
            });
        }

        assert_eq!(
            buf,
            &[4, 1, 1, 5, 1, 2, 3, 4, 5, 5, 1, 1, 5, 6, 7, 8, 9, 10]
        )
    }
}
