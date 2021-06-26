use crate::enc;
use crate::stream;
use crate::wire;

pub struct Conn<'a> {
    sid: u64,
    tr: wire::transport::Transport<'a>,
    buf: Vec<u8>,
}

impl<'a> Conn<'a> {
    pub fn new(tr: &'a mut dyn wire::Transport) -> Conn<'a> {
        Conn {
            sid: 0,
            tr: wire::transport::Transport::new(tr),
            buf: Vec::new(),
        }
    }

    fn new_stream<'s, Enc, In, Out>(&'s mut self) -> stream::Stream<'s, Enc, In, Out> {
        self.sid += 1;
        stream::GenericStream::new(self.sid, &mut self.tr, &mut self.buf).fix()
    }

    pub fn invoke<Enc, In, Out>(&mut self, rpc: &[u8], input: &In) -> stream::Result<Out>
    where
        Enc: enc::Marshal<In>,
        Enc: enc::Unmarshal<Out>,
        Out: Default,
    {
        let mut out = Default::default();
        self.invoke_into::<Enc, _, _>(rpc, input, &mut out)?;
        Ok(out)
    }

    pub fn invoke_into<Enc, In, Out>(
        &mut self,
        rpc: &[u8],
        input: &In,
        out: &mut Out,
    ) -> stream::Result<()>
    where
        Enc: enc::Marshal<In>,
        Enc: enc::Unmarshal<Out>,
    {
        let mut st = self.new_stream::<Enc, In, Out>();
        st.invoke(rpc)?;
        st.send(input)?;
        st.close_send()?;
        st.recv_into(out)
    }

    pub fn stream<'s, Enc, In, Out>(
        self: &'s mut Self,
        rpc: &[u8],
    ) -> stream::Result<stream::Stream<'s, Enc, In, Out>>
    where
        Enc: enc::Marshal<In>,
        Enc: enc::Unmarshal<Out>,
    {
        let mut st = self.new_stream();
        st.invoke(rpc)?;
        Ok(st)
    }
}
