use super::frame;
use super::packet;

pub struct Split<'a> {
    pkt: packet::Packet<'a>,
    n: usize,
    done: bool,
}

impl<'a> Iterator for Split<'a> {
    type Item = frame::Frame<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let mut fr = frame::Frame {
            data: self.pkt.data,
            id: self.pkt.id,
            kind: self.pkt.kind as u8,
            done: true,
            control: false,
        };

        if self.pkt.data.len() > self.n && self.n > 0 {
            let (begin, end) = self.pkt.data.split_at(self.n);
            fr.data = begin;
            self.pkt.data = end;
            fr.done = false;
        }

        self.done = fr.done;
        Some(fr)
    }
}

pub fn split(pkt: packet::Packet, n: usize) -> Split {
    Split {
        pkt: pkt,
        n: n,
        done: false,
    }
}

#[cfg(test)]
mod tests {
    use crate::frame;
    use crate::id;
    use crate::packet;

    static ID: id::ID = id::ID {
        stream: 5,
        message: 10,
    };

    static PKT: packet::Packet = packet::Packet {
        data: &[1, 2, 3],
        id: ID,
        kind: packet::Kind::Message,
    };

    #[test]
    fn test_split_small() {
        assert_eq!(
            super::split(PKT, 1).collect::<Vec<_>>(),
            vec![
                frame::Frame {
                    data: &[1],
                    id: ID,
                    kind: 2,
                    done: false,
                    control: false
                },
                frame::Frame {
                    data: &[2],
                    id: ID,
                    kind: 2,
                    done: false,
                    control: false
                },
                frame::Frame {
                    data: &[3],
                    id: ID,
                    kind: 2,
                    done: true,
                    control: false
                },
            ]
        )
    }

    #[test]
    fn test_split_large() {
        assert_eq!(
            super::split(PKT, 0).collect::<Vec<_>>(),
            vec![frame::Frame {
                data: &[1, 2, 3],
                id: ID,
                kind: 2,
                done: true,
                control: false
            }]
        )
    }
}
