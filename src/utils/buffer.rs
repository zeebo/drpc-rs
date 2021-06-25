use std::io;

struct BufferState {
    buf: Vec<u8>,
}

impl BufferState {
    fn new() -> BufferState {
        BufferState { buf: Vec::new() }
    }

    fn from(buf: Vec<u8>) -> BufferState {
        BufferState { buf }
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

impl io::Read for Buffer {
    fn read(self: &mut Self, buf: &mut [u8]) -> io::Result<usize> {
        let mut state = self.state.lock().unwrap();
        while state.empty() {
            state = self.cond.wait(state).unwrap();
        }

        let n = std::cmp::min(buf.len(), state.buf.len());
        buf[0..n].copy_from_slice(&state.buf[0..n]);
        state.buf.drain(0..n);

        self.cond.notify_all();
        Ok(n)
    }
}

impl io::Write for Buffer {
    fn write(self: &mut Self, buf: &[u8]) -> io::Result<usize> {
        let mut state = self.state.lock().unwrap();
        while state.full() {
            state = self.cond.wait(state).unwrap();
        }

        state.buf.extend_from_slice(buf);

        self.cond.notify_all();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

mod test {
    use std::io::{Read, Write};

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
