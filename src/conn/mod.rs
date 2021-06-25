use crate::enc;
use crate::stream;
use crate::wire::packet;

mod buffered;

pub trait Transport: std::io::Read + std::io::Write {}

impl<T> Transport for T where T: std::io::Read + std::io::Write {}

pub struct Conn<'a> {
    sid: u64,
    tr: buffered::Transport<'a>,
    buf: Vec<u8>,
}

impl<'a> Conn<'a> {
    pub fn new(tr: &'a mut dyn Transport) -> Conn<'a> {
        Conn {
            sid: 0,
            tr: buffered::Transport::new(tr),
            buf: Vec::new(),
        }
    }

    fn new_stream<'s, Enc, In, Out>(&'s mut self) -> stream::Stream<'s, Enc, In, Out> {
        self.sid += 1;
        stream::Stream::new(self.sid, &mut self.tr, &mut self.buf)
    }

    pub fn invoke<Enc, In, Out>(&mut self, rpc: &str, input: &In) -> crate::Result<Out>
    where
        Enc: enc::Marshal<In> + enc::Unmarshal<Out>,
        Out: Default,
    {
        let mut out = Default::default();
        self.invoke_into::<Enc, _, _>(rpc, input, &mut out)?;
        Ok(out)
    }

    pub fn invoke_into<Enc, In, Out>(
        &mut self,
        rpc: &str,
        input: &In,
        out: &mut Out,
    ) -> crate::Result<()>
    where
        Enc: enc::Marshal<In> + enc::Unmarshal<Out>,
    {
        let mut st = self.new_stream::<Enc, In, Out>();
        st.invoke(rpc)?;
        st.send(input)?;
        st.close_send()?;
        st.recv_into(out)
    }

    pub fn stream<'s, Enc, In, Out>(
        self: &'s mut Self,
        rpc: &str,
    ) -> crate::Result<stream::Stream<'s, Enc, In, Out>>
    where
        Enc: enc::Marshal<In> + enc::Unmarshal<Out>,
    {
        let mut st = self.new_stream();
        st.invoke(rpc)?;
        Ok(st)
    }
}
