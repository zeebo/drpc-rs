use async_trait::async_trait;

use crate::{enc, stream, Transport};

use crate::{StreamRecv, StreamSend};

pub struct Conn<T: crate::Transport> {
    sid: u64,
    tr: T,
    buf: Vec<u8>,
}

impl<T: crate::Transport> Conn<T> {
    pub fn new(tr: T) -> Conn<T> {
        Conn {
            sid: 0,
            tr,
            buf: Vec::new(),
        }
    }

    fn new_stream<'s>(&'s mut self) -> stream::Stream<'s> {
        self.sid += 1;
        stream::Stream::new(self.sid, &mut self.tr, &mut self.buf)
    }

    pub fn transport(&mut self) -> &mut T {
        &mut self.tr
    }

    pub async fn invoke_into<In: enc::Marshal, Out: enc::Unmarshal>(
        &mut self,
        rpc: &[u8],
        input: &In,
        out: &mut Out,
    ) -> stream::Result<()> {
        let mut st = self.new_stream();
        st.invoke(rpc).await?;
        st.send(input).await?;
        st.close_send().await?;
        st.recv_into(out).await?;
        st.close().await?;
        Ok(())
    }

    pub async fn stream<'s>(&'s mut self, rpc: &[u8]) -> stream::Result<stream::Stream<'s>> {
        let mut st = self.new_stream();
        st.invoke(rpc).await?;
        Ok(st)
    }
}

#[async_trait]
impl<T: crate::Transport> crate::Conn for Conn<T> {
    fn transport(&mut self) -> &mut dyn Transport {
        self.transport()
    }

    async fn invoke_into<In: enc::Marshal, Out: enc::Unmarshal>(
        &mut self,
        rpc: &[u8],
        input: &In,
        out: &mut Out,
    ) -> stream::Result<()> {
        self.invoke_into(rpc, input, out).await
    }

    async fn stream<'s, In, Out>(
        &'s mut self,
        rpc: &[u8],
    ) -> stream::Result<Box<dyn crate::Stream<In, Out> + 's>>
    where
        In: enc::Marshal + 's,
        Out: enc::Unmarshal + 's,
    {
        let st = self.stream(rpc).await?;
        Ok(Box::new(st))
    }
}
