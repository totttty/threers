//! RFC 1951 DEFLATE decoder. Pure Rust, no dependencies. Used by EXR ZIP/ZIPS
//! and any future zlib-flavoured payloads. Supports stored blocks, fixed
//! Huffman, and dynamic Huffman.
//!
//! Pass in zlib-wrapped data (1.95 RFC) or raw DEFLATE. `inflate_raw` consumes
//! raw DEFLATE; `inflate_zlib` peels the 2-byte zlib header + Adler-32 trailer.

#[derive(Debug)]
pub enum DeflateError {
    Truncated,
    BadCode,
    BadDistance,
    BadHeader,
}

pub fn inflate_zlib(input: &[u8]) -> Result<Vec<u8>, DeflateError> {
    if input.len() < 6 {
        return Err(DeflateError::Truncated);
    }
    // Zlib header: CMF (low nibble == 8 for deflate) + FLG.
    if (input[0] & 0x0f) != 8 {
        return Err(DeflateError::BadHeader);
    }
    let payload = &input[2..input.len() - 4];
    inflate_raw(payload)
}

pub fn inflate_raw(input: &[u8]) -> Result<Vec<u8>, DeflateError> {
    let mut r = BitReader::new(input);
    let mut out = Vec::new();
    loop {
        let bfinal = r.read_bits(1)?;
        let btype = r.read_bits(2)?;
        match btype {
            0 => inflate_stored(&mut r, &mut out)?,
            1 => inflate_fixed(&mut r, &mut out)?,
            2 => inflate_dynamic(&mut r, &mut out)?,
            _ => return Err(DeflateError::BadHeader),
        }
        if bfinal == 1 {
            break;
        }
    }
    Ok(out)
}

fn inflate_stored(r: &mut BitReader, out: &mut Vec<u8>) -> Result<(), DeflateError> {
    r.align_to_byte();
    let len = r.read_le_u16()? as usize;
    let _nlen = r.read_le_u16()?;
    for _ in 0..len {
        out.push(r.read_byte()?);
    }
    Ok(())
}

fn inflate_fixed(r: &mut BitReader, out: &mut Vec<u8>) -> Result<(), DeflateError> {
    let (lit, dist) = fixed_huffman_tables();
    inflate_block(r, out, &lit, &dist)
}

fn inflate_dynamic(r: &mut BitReader, out: &mut Vec<u8>) -> Result<(), DeflateError> {
    let hlit = r.read_bits(5)? as usize + 257;
    let hdist = r.read_bits(5)? as usize + 1;
    let hclen = r.read_bits(4)? as usize + 4;
    let code_length_order = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut clens = [0u8; 19];
    for i in 0..hclen {
        clens[code_length_order[i]] = r.read_bits(3)? as u8;
    }
    let cl_tree = build_huffman(&clens)?;

    let total = hlit + hdist;
    let mut all_lens: Vec<u8> = Vec::with_capacity(total);
    while all_lens.len() < total {
        let sym = decode_symbol(r, &cl_tree)?;
        match sym {
            0..=15 => all_lens.push(sym as u8),
            16 => {
                let n = r.read_bits(2)? as usize + 3;
                let last = *all_lens.last().ok_or(DeflateError::BadCode)?;
                for _ in 0..n {
                    all_lens.push(last);
                }
            }
            17 => {
                let n = r.read_bits(3)? as usize + 3;
                for _ in 0..n {
                    all_lens.push(0);
                }
            }
            18 => {
                let n = r.read_bits(7)? as usize + 11;
                for _ in 0..n {
                    all_lens.push(0);
                }
            }
            _ => return Err(DeflateError::BadCode),
        }
    }
    let lit = build_huffman(&all_lens[..hlit])?;
    let dist = build_huffman(&all_lens[hlit..hlit + hdist])?;
    inflate_block(r, out, &lit, &dist)
}

fn inflate_block(
    r: &mut BitReader,
    out: &mut Vec<u8>,
    lit: &HuffmanTree,
    dist: &HuffmanTree,
) -> Result<(), DeflateError> {
    let length_extra = [
        (3, 0),
        (4, 0),
        (5, 0),
        (6, 0),
        (7, 0),
        (8, 0),
        (9, 0),
        (10, 0),
        (11, 1),
        (13, 1),
        (15, 1),
        (17, 1),
        (19, 2),
        (23, 2),
        (27, 2),
        (31, 2),
        (35, 3),
        (43, 3),
        (51, 3),
        (59, 3),
        (67, 4),
        (83, 4),
        (99, 4),
        (115, 4),
        (131, 5),
        (163, 5),
        (195, 5),
        (227, 5),
        (258, 0),
    ];
    let dist_extra = [
        (1, 0),
        (2, 0),
        (3, 0),
        (4, 0),
        (5, 1),
        (7, 1),
        (9, 2),
        (13, 2),
        (17, 3),
        (25, 3),
        (33, 4),
        (49, 4),
        (65, 5),
        (97, 5),
        (129, 6),
        (193, 6),
        (257, 7),
        (385, 7),
        (513, 8),
        (769, 8),
        (1025, 9),
        (1537, 9),
        (2049, 10),
        (3073, 10),
        (4097, 11),
        (6145, 11),
        (8193, 12),
        (12289, 12),
        (16385, 13),
        (24577, 13),
    ];
    loop {
        let sym = decode_symbol(r, lit)?;
        if sym < 256 {
            out.push(sym as u8);
        } else if sym == 256 {
            return Ok(());
        } else {
            let lsym = (sym - 257) as usize;
            if lsym >= length_extra.len() {
                return Err(DeflateError::BadCode);
            }
            let (base, extra) = length_extra[lsym];
            let length = base as usize + r.read_bits(extra as u32)? as usize;
            let dsym = decode_symbol(r, dist)? as usize;
            if dsym >= dist_extra.len() {
                return Err(DeflateError::BadDistance);
            }
            let (dbase, dextra) = dist_extra[dsym];
            let distance = dbase as usize + r.read_bits(dextra as u32)? as usize;
            if distance > out.len() {
                return Err(DeflateError::BadDistance);
            }
            let start = out.len() - distance;
            for k in 0..length {
                let b = out[start + k % distance];
                out.push(b);
            }
        }
    }
}

#[derive(Debug)]
struct HuffmanTree {
    /// Lookup table: bits[code_len-1][reversed_code] = symbol. Slow but simple.
    codes: Vec<(u32, u8, u32)>, // (length, _, symbol) tuples sorted by (length, code)
}

fn build_huffman(lengths: &[u8]) -> Result<HuffmanTree, DeflateError> {
    let max_len = *lengths.iter().max().unwrap_or(&0) as usize;
    if max_len == 0 {
        return Ok(HuffmanTree { codes: Vec::new() });
    }
    let mut bl_count = vec![0u32; max_len + 1];
    for &l in lengths {
        if l > 0 {
            bl_count[l as usize] += 1;
        }
    }
    let mut next_code = vec![0u32; max_len + 1];
    let mut code = 0u32;
    for bits in 1..=max_len {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }
    let mut codes = Vec::new();
    for (sym, &l) in lengths.iter().enumerate() {
        if l > 0 {
            let l = l as usize;
            codes.push((l as u32, 0u8, next_code[l]));
            // Replace with (length, _, code, sym) actually; we'll restructure.
            *codes.last_mut().unwrap() = (l as u32, 0, next_code[l]);
            next_code[l] += 1;
            let last = codes.len() - 1;
            codes[last] = (l as u32, 0, ((next_code[l] - 1) << 16) | sym as u32);
        }
    }
    Ok(HuffmanTree { codes })
}

fn decode_symbol(r: &mut BitReader, tree: &HuffmanTree) -> Result<u32, DeflateError> {
    // Build a simple linear scan: read 1 bit at a time, accumulate, find matching code.
    let mut code = 0u32;
    let mut len = 0u32;
    while len < 16 {
        let bit = r.read_bits(1)?;
        code = (code << 1) | bit as u32;
        len += 1;
        // Linear search through the codes table.
        for &(l, _, packed) in &tree.codes {
            if l == len && (packed >> 16) == code {
                return Ok(packed & 0xffff);
            }
        }
    }
    Err(DeflateError::BadCode)
}

fn fixed_huffman_tables() -> (HuffmanTree, HuffmanTree) {
    let mut lens = vec![0u8; 288];
    for i in 0..144 {
        lens[i] = 8;
    }
    for i in 144..256 {
        lens[i] = 9;
    }
    for i in 256..280 {
        lens[i] = 7;
    }
    for i in 280..288 {
        lens[i] = 8;
    }
    let lit = build_huffman(&lens).unwrap();
    let dist = build_huffman(&vec![5u8; 30]).unwrap();
    (lit, dist)
}

// ---- bit reader ----

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_buf: u32,
    bit_count: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bit_buf: 0,
            bit_count: 0,
        }
    }

    fn read_bits(&mut self, n: u32) -> Result<u32, DeflateError> {
        while self.bit_count < n {
            if self.pos >= self.data.len() {
                return Err(DeflateError::Truncated);
            }
            self.bit_buf |= (self.data[self.pos] as u32) << self.bit_count;
            self.pos += 1;
            self.bit_count += 8;
        }
        let v = self.bit_buf & ((1u32 << n) - 1);
        self.bit_buf >>= n;
        self.bit_count -= n;
        Ok(v)
    }

    fn align_to_byte(&mut self) {
        let drop = self.bit_count % 8;
        self.bit_buf >>= drop;
        self.bit_count -= drop;
    }

    fn read_byte(&mut self) -> Result<u8, DeflateError> {
        if self.bit_count >= 8 {
            let b = (self.bit_buf & 0xff) as u8;
            self.bit_buf >>= 8;
            self.bit_count -= 8;
            Ok(b)
        } else if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            Ok(b)
        } else {
            Err(DeflateError::Truncated)
        }
    }

    fn read_le_u16(&mut self) -> Result<u16, DeflateError> {
        let a = self.read_byte()? as u16;
        let b = self.read_byte()? as u16;
        Ok(a | (b << 8))
    }
}
