//! helper lib to to work with strings

const BUFFER_LENGTH: usize = 12;
const BUFFER_LAST_IDX: usize = BUFFER_LENGTH - 1;
type Buffer = [u8; 12];

pub fn num_length(n: u32) -> usize {
    if n > 9 {
        num_length(n / 10) + 1
    } else {
        1
    }
}

#[allow(dead_code)]
pub fn num_length_hex(n: u32) -> usize {
    if n > 15 {
        num_length(n >> 4) + 1
    } else {
        1
    }
}

pub fn u32_to_str(num: u32) -> (usize, Buffer) {
    let lng = num_length(num);
    let mut value = num;

    let mut buf = [b' '; BUFFER_LENGTH];
    for i in 0..lng {
        let dig = value % 10;
        value /= 10;
        buf[lng - i] = b'0' + dig as u8;
    }
    (lng, buf)
}

pub fn f32_to_str(mut value: f32, dig: u8) -> (usize, Buffer) {
    // can't fix this with rust types :-(
    assert!(dig < 5);

    let mut mul = 1_f32;
    for _ in 0..dig {
        mul *= 10_f32;
    }

    value *= mul;

    let l: i16 = if value > i16::MAX as f32 {
        i16::MAX
    } else if value < i16::MIN as f32 {
        i16::MIN
    } else {
        unsafe { value.to_int_unchecked() }
    };

    let (len, mut buffer) = i16_to_str(l);
    if dig == 0 {
        (len, buffer)
    } else {
        // if there are missing leading '0', write them into the buffer
        let dig = dig as usize;
        if len <= dig {
            let missing_leading_null = dig + 1 - len;
            let fill_start = BUFFER_LAST_IDX - len;
            for i in 0..missing_leading_null {
                buffer[fill_start - i] = b'0';
            }
        }
        // create a gap and insert the '.'
        let dot_pos = BUFFER_LAST_IDX - dig;
        for i in 1..=dot_pos {
            buffer[i - 1] = buffer[i];
        }
        buffer[dot_pos] = b'.';

        // return the new buffer and the length
        ((len + 1).min(BUFFER_LENGTH), buffer)
    }
}
pub fn i16_to_str(value: i16) -> (usize, Buffer) {
    let sign_length = if value < 0 { 1 } else { 0 };
    let mut value = value.unsigned_abs();
    let lng = num_length(value as u32).min(BUFFER_LAST_IDX);

    let mut buf = [b' '; BUFFER_LENGTH];

    // add sign
    if sign_length != 0 {
        // lng <= BUFFER_LENGTH - 1
        buf[BUFFER_LAST_IDX - lng - 1] = b'-';
    }

    for i in 0..lng {
        let dig = value % 10;
        value /= 10;

        buf[BUFFER_LAST_IDX - i] = b'0' + dig as u8;
    }
    (lng + sign_length, buf)
}
