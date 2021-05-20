#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    NotEnoughData,
    VarintTooLong,
}

pub fn read(mut buf: &[u8]) -> Result<(u64, usize), Error> {
    let mut out: u64 = 0;
    let mut n: usize = 0;

    for shift in (0..64).step_by(7) {
        n += 1;

        if let [val, rem @ ..] = buf {
            out |= ((*val as u64) & 127) << shift;
            buf = rem;

            if *val < 128 {
                return Ok((out, n));
            }
        } else {
            return Err(Error::NotEnoughData);
        }
    }

    Err(Error::VarintTooLong)
}

pub fn append(buf: &mut Vec<u8>, mut x: u64) -> () {
    while x >= 128 {
        buf.push((x & 127 | 128) as u8);
        x >>= 7;
    }
    buf.push(x as u8)
}

#[cfg(test)]
mod tests {
    #[test]
    fn read_simple() {
        assert_eq!(super::read(&[1, 2, 3]), Ok((1, 1)));
    }

    #[test]
    fn read_multibyte() {
        assert_eq!(super::read(&[128, 130, 3, 5]), Ok((49408, 3)));
    }

    #[test]
    fn read_not_enough_data() {
        assert_eq!(super::read(&[128, 130]), Err(super::Error::NotEnoughData));
    }

    #[test]
    fn read_varint_too_long() {
        assert_eq!(
            super::read(&[128, 128, 128, 128, 128, 128, 128, 128, 128, 128]),
            Err(super::Error::VarintTooLong),
        );
    }

    #[test]
    fn append_simple() {
        let mut buf = vec![];
        super::append(&mut buf, 1);
        assert_eq!(&buf, &[1])
    }

    #[test]
    fn append_multibyte() {
        let mut buf = vec![];
        super::append(&mut buf, 49408);
        assert_eq!(&buf, &[128, 130, 3])
    }
}
