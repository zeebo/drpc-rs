use crate::{
    enc, stream,
    transport::{self, Stream},
};

pub struct Conn<W> {
    sid: u64,
    tr: transport::Transport<W>,
    buf: Vec<u8>,
}

impl<W: transport::Wire> Conn<W> {
    pub fn new(w: W) -> Conn<W> {
        Conn {
            sid: 0,
            tr: transport::Transport::new(w),
            buf: Vec::new(),
        }
    }

    fn new_stream<'s>(&'s mut self) -> stream::GenericStream<'s> {
        self.sid += 1;
        stream::GenericStream::new(self.sid, &mut self.tr, &mut self.buf)
    }

    pub fn wire(&mut self) -> &mut dyn transport::Wire {
        self.tr.wire()
    }

    pub async fn invoke_into<Enc, In, Out>(
        &mut self,
        rpc: &[u8],
        input: &In,
        out: &mut Out,
    ) -> stream::Result<()>
    where
        Enc: enc::Marshal<In> + enc::Unmarshal<Out> + Send,
    {
        let mut st = self.new_stream();
        st.invoke(rpc).await?;
        st.send::<Enc, In>(input).await?;
        st.close_send().await?;
        st.recv_into::<Enc, Out>(out).await?;
        Ok(())
    }

    pub async fn invoke<Enc, In, Out: Default>(
        &mut self,
        rpc: &[u8],
        input: &In,
    ) -> stream::Result<Out>
    where
        Enc: enc::Marshal<In> + enc::Unmarshal<Out> + Send,
    {
        let mut out = Default::default();
        self.invoke_into::<Enc, In, Out>(rpc, input, &mut out)
            .await?;
        Ok(out)
    }

    pub async fn stream<'s, Enc: 's, In: 's, Out: 's>(
        &'s mut self,
        rpc: &[u8],
    ) -> stream::Result<stream::Stream<'s, Enc, In, Out>> {
        let mut st = self.new_stream();
        st.invoke(rpc).await?;
        Ok(st.fix())
    }
}
