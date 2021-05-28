use crate::stream;
use crate::traits;
use crate::traits::*;
use crate::wire::packet;
use crate::wire::transport;

pub struct Conn<'a, Tr: Transport> {
    closed: bool,
    sid: u64,
    st: stream::Stream<'a, Tr>,
}

impl<'a, Tr: Transport> Conn<'a, Tr> {
    pub fn new(tr: &mut Tr) -> Conn<Tr> {
        Conn {
            closed: false,
            sid: 0,
            st: stream::Stream::new(0, transport::Transport::new(tr, 1024)),
        }
    }

    fn new_stream(self: &mut Self) {
        self.sid += 1;
        self.st.reset(self.sid);
    }

    pub fn close(self: &mut Self) -> Result<()> {
        self.closed = true;
        self.transport().close()
    }

    pub fn closed(self: &mut Self) -> bool {
        self.closed
    }

    pub fn transport(self: &mut Self) -> &mut Tr {
        self.st.transport().transport()
    }

    pub fn invoke<In, Out, Enc: Encoding<In, Out>>(
        self: &mut Self,
        rpc: &[u8],
        enc: &Enc,
        input: &In,
    ) -> Result<Out> {
        self.new_stream();
        let buf = enc.marshal(input)?;
        self.st.raw_write(packet::Kind::Invoke, rpc)?;
        self.st.raw_write(packet::Kind::Message, buf)?;
        self.st.close_send()?;
        self.st.raw_flush()?;
        self.st.recv(enc)
    }

    pub fn stream(self: &mut Self, rpc: &[u8]) -> Result<&mut stream::Stream<'a, Tr>> {
        self.new_stream();
        self.st.raw_write(packet::Kind::Invoke, rpc)?;
        self.st.raw_flush()?;
        Ok(&mut self.st)
    }
}
