fn u8_to_u16(slice: &[u8]) -> u16 {
    if slice.len() < 2 {
        panic!("Not enough bytes in the slice");
    }

    let byte1 = slice[0] as u16;
    let byte2 = slice[1] as u16;

    (byte1 << 8) | byte2
}
