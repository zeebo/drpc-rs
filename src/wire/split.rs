use crate::wire::frame;
use crate::wire::packet;

pub struct Split<'a, D> {
    pkt: &'a packet::Packet<D>,
    data: &'a [u8],
    n: usize,
    done: bool,
}

impl<'a, D> Iterator for Split<'a, D> {
    type Item = frame::Frame<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let mut fr = frame::Frame {
            data: &self.data,
            id: self.pkt.id,
            kind: self.pkt.kind.into(),
            done: true,
            control: false,
        };

        if self.data.len() > self.n && self.n > 0 {
            let (start, end) = self.data.split_at(self.n);
            fr.data = start;
            self.data = end;
            fr.done = false;
        }

        self.done = fr.done;
        Some(fr)
    }
}

pub fn split<'a, Data: std::borrow::Borrow<[u8]>>(
    pkt: &'a packet::Packet<Data>,
    n: usize,
) -> Split<'a, Data> {
    Split {
        pkt,
        data: pkt.data.borrow(),
        n,
        done: false,
    }
}

#[cfg(test)]
mod tests {
    use crate::wire::frame;
    use crate::wire::id;
    use crate::wire::packet;

    static ID: id::ID = id::ID {
        stream: 5,
        message: 10,
    };

    #[test]
    fn test_split_small() {
        let pkt = packet::Packet {
            data: vec![1, 2, 3],
            id: ID,
            kind: packet::Kind::Message,
        };

        assert_eq!(
            super::split(&pkt, 1).collect::<Vec<_>>(),
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
        let pkt = packet::Packet {
            data: vec![1, 2, 3],
            id: ID,
            kind: packet::Kind::Message,
        };

        assert_eq!(
            super::split(&pkt, 0).collect::<Vec<_>>(),
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
