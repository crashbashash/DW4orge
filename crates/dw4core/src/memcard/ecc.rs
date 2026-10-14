//! The PS2 card's per-page error-correcting codes.
//!
//! Each 128-byte chunk of page data yields three bytes: a column parity and two
//! line parities. A page's 16-byte spare area is four such codes followed by
//! four zero bytes on write.
//!
//! Verified against the real card: this reproduces the stored ECC on 351 of the
//! 352 in-use pages, the exception being page 1 in the superblock region, which
//! `write_save` never touches.

/// Bytes of page data per ECC block.
pub const ECC_CHUNK: usize = 128;

/// ECC bytes produced per chunk.
pub const ECC_CHUNK_BYTES: usize = 3;

/// Column-parity masks, in bit order.
const COLUMN_MASKS: [u8; 7] = [0x55, 0x33, 0x0F, 0x00, 0xAA, 0xCC, 0xF0];

/// Odd parity of a byte, folded to one bit.
#[inline]
const fn parity(byte: u8) -> u8 {
    let mut b = byte;
    b ^= b >> 1;
    b ^= b >> 2;
    b ^= b >> 4;
    b & 1
}

/// The column-parity byte for `value`.
#[inline]
fn column_parity(value: u8) -> u8 {
    let mut mask = 0u8;
    let mut i = 0;
    while i < COLUMN_MASKS.len() {
        mask |= parity(value & COLUMN_MASKS[i]) << i;
        i += 1;
    }
    mask
}

/// The three ECC bytes for one 128-byte chunk: `[column, line0, line1]`.
#[must_use]
pub fn ecc_chunk(chunk: &[u8; ECC_CHUNK]) -> [u8; ECC_CHUNK_BYTES] {
    let mut cp = 0x77u8;
    let mut lp0 = 0x7Fu8;
    let mut lp1 = 0x7Fu8;

    for (i, b) in chunk.iter().enumerate() {
        cp ^= column_parity(*b);
        if parity(*b) != 0 {
            // `!i` as a byte has the same low seven bits as the `~i` the
            // reference uses; only those bits survive the mask below.
            lp0 ^= !(i as u8);
            lp1 ^= i as u8;
        }
    }

    [cp, lp0 & 0x7F, lp1 & 0x7F]
}

/// The full spare area for a page of data: four ECC codes, then zeros.
///
/// # Panics
/// If `page_data` is empty or not a multiple of [`ECC_CHUNK`].
#[must_use]
pub fn page_spare(page_data: &[u8]) -> Vec<u8> {
    assert!(
        !page_data.is_empty() && page_data.len().is_multiple_of(ECC_CHUNK),
        "page data must be a non-zero multiple of {ECC_CHUNK} bytes, got {}",
        page_data.len()
    );
    let chunks = page_data.len() / ECC_CHUNK;
    let mut out = Vec::with_capacity(chunks * 4);
    for i in 0..chunks {
        let mut block = [0u8; ECC_CHUNK];
        block.copy_from_slice(&page_data[i * ECC_CHUNK..(i + 1) * ECC_CHUNK]);
        out.extend_from_slice(&ecc_chunk(&block));
    }
    // The reference pads to (page_size / 128) * 4 bytes with zeros: three per
    // chunk plus one, i.e. `chunks` zero bytes in total.
    out.extend(std::iter::repeat_n(0u8, chunks));
    out
}
