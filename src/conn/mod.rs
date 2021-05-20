use crate::stream;
use crate::traits;
use crate::wire;

use crate::traits::Stream;

pub struct Conn<'a> {
    closed: bool,
    sid: u64,
    stream: stream::Stream<'a>,
}

impl<'a> Conn<'a> {
    pub fn new(tr: &'a mut dyn traits::Transport) -> Conn<'a> {
        Conn {
            closed: false,
            sid: 0,
            stream: stream::Stream::new(0, wire::transport::Transport::new(tr, 1024)),
        }
    }

    fn new_stream(self: &mut Self) {
        self.sid += 1;
        self.stream.reset(self.sid);
    }
}

impl<'a> traits::Conn for Conn<'a> {
    fn close(self: &mut Self) -> traits::Result<()> {
        self.closed = true;
        self.transport().close()
    }

    fn closed(self: &mut Self) -> bool {
        self.closed
    }

    fn transport(self: &mut Self) -> &mut dyn traits::Transport {
        self.stream.transport().transport()
    }

    fn invoke<In, Out>(
        self: &mut Self,
        rpc: &[u8],
        enc: &dyn traits::Encoding<In, Out>,
        input: &In,
    ) -> traits::Result<Out> {
        self.new_stream();
        let buf = enc.marshal(input)?;
        self.stream
            .raw_write(wire::packet::Kind::Invoke, rpc.to_owned())?; // TODO: avoid copy
        self.stream.raw_write(wire::packet::Kind::Message, buf)?;
        traits::Stream::<In, Out>::close_send(&mut self.stream)?;
        self.stream.raw_flush()?;
        self.stream.recv(enc)
    }

    fn stream<In, Out>(
        self: &mut Self,
        rpc: &[u8],
    ) -> traits::Result<&mut dyn traits::Stream<In, Out>> {
        self.new_stream();
        self.stream
            .raw_write(wire::packet::Kind::Invoke, rpc.to_owned())?; // TODO: avoid copy
        self.stream.raw_flush()?;
        Ok(&mut self.stream)
    }
}
