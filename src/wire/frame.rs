use crate::wire::id;
use crate::wire::varint;

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct Frame<'a> {
    pub data: &'a [u8],
    pub id: id::ID,
    pub kind: u8,
    pub done: bool,
    pub control: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    NotEnoughData,
    ParseError,
}

impl From<varint::Error> for Error {
    fn from(err: varint::Error) -> Error {
        match err {
            varint::Error::NotEnoughData => Error::NotEnoughData,
            varint::Error::VarintTooLong => Error::ParseError,
        }
    }
}

pub fn parse_frame(buf: &[u8]) -> Result<(Frame, usize), Error> {
    let mut buf = buf;
    let mut fr: Frame = Default::default();
    let read: usize = 1;

    if let [val, rem @ ..] = buf {
        buf = rem;

        fr.done = (val & 0b00000001) > 0;
        fr.control = (val & 0b10000000) > 0;
        fr.kind = (val & 0b01111110) >> 1
    } else {
        return Err(Error::NotEnoughData);
    }

    let (val, n) = varint::read(buf)?;
    let (read, buf) = (read + n, &buf[n..buf.len()]);
    fr.id.stream = val;

    let (val, n) = varint::read(buf)?;
    let (read, buf) = (read + n, &buf[n..buf.len()]);
    fr.id.message = val;

    let (val, n) = varint::read(buf)?;
    let (read, buf) = (read + n, &buf[n..buf.len()]);

    if val > buf.len() as u64 {
        return Err(Error::NotEnoughData);
    }

    fr.data = &buf[..val as usize];

    Ok((fr, read + val as usize))
}

pub fn append_frame<'a>(buf: &mut Vec<u8>, fr: &Frame<'a>) {
    let mut control = fr.kind << 1;
    if fr.done {
        control |= 0b00000001
    }
    if fr.control {
        control |= 0b10000000
    }

    buf.push(control);
    varint::append(buf, fr.id.stream);
    varint::append(buf, fr.id.message);
    varint::append(buf, fr.data.len() as u64);
    buf.extend_from_slice(fr.data);
}

#[cfg(test)]
mod tests {
    static FR: super::Frame = super::Frame {
        data: &[1, 2, 3],
        id: super::id::ID {
            stream: 5,
            message: 10,
        },
        kind: 5,
        done: true,
        control: false,
    };

    #[test]
    fn append_read() {
        let mut buf = vec![];
        super::append_frame(&mut buf, &FR);
        buf.push(99);

        assert_eq!(super::parse_frame(&buf), Ok((FR, 7)));
    }

    #[test]
    fn read_not_enough_data() {
        let mut buf = vec![];
        super::append_frame(&mut buf, &FR);
        buf.truncate(buf.len() - 1);

        assert_eq!(super::parse_frame(&buf), Err(super::Error::NotEnoughData))
    }

    #[test]
    fn read_parse_error() {
        assert_eq!(
            super::parse_frame(&[0, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128]),
            Err(super::Error::ParseError),
        )
    }
}
