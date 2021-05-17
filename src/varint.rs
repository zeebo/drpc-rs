#[derive(Debug, PartialEq, Eq)]
pub enum ReadResult<'a> {
    Ok(u64, &'a [u8]),
    NotEnoughData,
    VarintTooLong,
}

pub fn read(mut buf: &[u8]) -> ReadResult {
    let mut out: u64 = 0;

    for shift in (0..64).step_by(7) {
        if let [val, rem @ ..] = buf {
            out |= ((*val as u64) & 127) << shift;
            buf = rem;

            if *val < 128 {
                return ReadResult::Ok(out, buf);
            }
        } else {
            return ReadResult::NotEnoughData;
        }
    }

    ReadResult::VarintTooLong
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
        assert_eq!(super::read(&[1, 2, 3]), super::ReadResult::Ok(1, &[2, 3]));
    }

    #[test]
    fn read_multibyte() {
        assert_eq!(
            super::read(&[128, 130, 3, 5]),
            super::ReadResult::Ok(49408, &[5])
        );
    }

    #[test]
    fn read_not_enough_data() {
        assert_eq!(super::read(&[128, 130]), super::ReadResult::NotEnoughData);
    }

    #[test]
    fn read_varint_too_long() {
        assert_eq!(
            super::read(&[128, 128, 128, 128, 128, 128, 128, 128, 128, 128]),
            super::ReadResult::VarintTooLong
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
