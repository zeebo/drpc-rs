use super::frame;
use super::id;
use super::packet;
use super::transport;

type Transport<'a, E> = Box<dyn transport::Transport<Error = E> + 'a>;

#[derive(Debug)]
enum Error<E: std::fmt::Debug> {
    ReadError(E),
    ParseError,
    IDMonotonicityError,
    PacketKindChangeError,
    DataOverflowError,
}

struct Reader<'a, E> {
    tr: Transport<'a, E>,
    id: id::ID,
}

impl<'a, E: std::fmt::Debug> Reader<'a, E> {
    fn new(tr: Transport<'a, E>) -> Reader<'a, E> {
        Reader {
            tr: tr,
            id: id::ID::default(),
        }
    }

    fn read_packet(self: &mut Self) -> Result<packet::Packet, Error<E>> {
        let mut tmp = vec![0; 4096];
        let mut buf = Vec::new();
        let mut parsed = 0;

        let mut pkt = packet::Packet::default();

        loop {
            if parsed > 0 {
                buf.drain(0..parsed);
                parsed = 0;
            }

            if buf.capacity() > (4 << 20 + 1 + 9 + 9 + 9) {
                return Err(Error::DataOverflowError);
            }

            match frame::parse_frame(&buf) {
                frame::ParseFrameResult::Ok(fr, n) => {
                    parsed = n;

                    if fr.control {
                        continue;
                    } else if fr.id < self.id {
                        return Err(Error::IDMonotonicityError);
                    } else if self.id < fr.id {
                        self.id = fr.id;
                        pkt.data.clear();
                        pkt.id = fr.id;
                        pkt.kind = fr.kind.into();
                    } else if pkt.kind != fr.kind.into() {
                        return Err(Error::PacketKindChangeError);
                    }

                    pkt.data.extend_from_slice(fr.data);

                    if pkt.data.len() > (4 << 20) {
                        return Err(Error::DataOverflowError);
                    } else if fr.done {
                        return Ok(pkt);
                    }
                }
                frame::ParseFrameResult::NotEnoughData => {
                    let (n, err) = self.tr.read(&mut tmp);
                    if let Some(err) = err {
                        return Err(Error::ReadError(err));
                    }
                    buf.extend_from_slice(&tmp[0..n]);
                }
                frame::ParseFrameResult::ParseError => return Err(Error::ParseError),
            }
        }
    }
}

mod test {
    use crate::id;
    use crate::packet;

    #[test]
    fn test_reader_packets() {
        let mut buf = vec![4, 1, 1, 5, 1, 2, 3, 4, 5, 5, 1, 1, 5, 6, 7, 8, 9, 10];
        let mut r = super::Reader::new(Box::new(&mut buf));

        assert_eq!(
            r.read_packet().unwrap(),
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
}
