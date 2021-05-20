use crate::traits;

#[derive(Debug)]
pub enum Error {
    Closed,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

//

struct BufferState {
    buf: Vec<u8>,
    closed: bool,
}

impl BufferState {
    fn new() -> BufferState {
        BufferState {
            buf: Vec::new(),
            closed: false,
        }
    }

    fn from(buf: Vec<u8>) -> BufferState {
        BufferState {
            buf: buf,
            closed: false,
        }
    }

    fn empty(self: &Self) -> bool {
        self.buf.len() == 0
    }

    fn full(self: &Self) -> bool {
        self.buf.len() > 4096
    }
}

//

pub struct Buffer {
    cond: std::sync::Condvar,
    state: std::sync::Mutex<BufferState>,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer::from(Vec::new())
    }

    pub fn from(buf: Vec<u8>) -> Buffer {
        Buffer {
            cond: std::sync::Condvar::new(),
            state: std::sync::Mutex::new(BufferState::from(buf)),
        }
    }

    pub fn take_contents(self: &Self) -> Vec<u8> {
        let mut state = self.state.lock().unwrap();
        std::mem::take(&mut state.buf)
    }
}

impl traits::Transport for Buffer {
    fn write(self: &mut Self, buf: &[u8]) -> traits::Result<usize> {
        let mut state = self.state.lock().unwrap();
        while !state.closed && state.full() {
            state = self.cond.wait(state).unwrap();
        }

        if state.closed {
            return Err(Box::new(Error::Closed));
        }

        state.buf.extend_from_slice(buf);

        self.cond.notify_all();
        Ok(buf.len())
    }

    fn read(self: &mut Self, buf: &mut [u8]) -> traits::Result<usize> {
        let mut state = self.state.lock().unwrap();
        while !state.closed && state.empty() {
            state = self.cond.wait(state).unwrap();
        }

        if state.closed {
            return Err(Box::new(Error::Closed));
        }

        let n = std::cmp::min(buf.len(), state.buf.len());
        buf[0..n].copy_from_slice(&state.buf[0..n]);
        state.buf.drain(0..n);

        self.cond.notify_all();
        Ok(n)
    }

    fn close(self: &mut Self) -> traits::Result<()> {
        let mut state = self.state.lock().unwrap();

        if state.closed {
            return Ok(());
        }

        state.closed = true;

        self.cond.notify_all();
        Ok(())
    }
}

mod test {
    use crate::traits::Transport;

    #[test]
    fn test_buffer_write_read() {
        let mut buf = super::Buffer::new();
        let mut tmp = [0; 10];

        buf.write(&[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(buf.read(&mut tmp).unwrap(), 5);
        assert_eq!(tmp, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0]);
        assert_eq!(buf.take_contents(), vec![]);
    }
}
