use std::io::{self, Read};
use flate2::read::GzDecoder;

pub enum DecompressionFormat {
    Gzip,
    UnixCompress, // .Z
    None,
}

pub fn get_format(filename: &str) -> DecompressionFormat {
    if filename.ends_with(".gz") {
        DecompressionFormat::Gzip
    } else if filename.ends_with(".Z") {
        DecompressionFormat::UnixCompress
    } else {
        DecompressionFormat::None
    }
}

pub fn wrap_decoder<'a, R: Read + 'a>(
    format: DecompressionFormat,
    reader: R,
) -> Box<dyn Read + 'a> {
    match format {
        DecompressionFormat::Gzip => Box::new(GzDecoder::new(reader)),
        DecompressionFormat::UnixCompress => Box::new(ZDecoder::new(reader)),
        DecompressionFormat::None => Box::new(reader),
    }
}

/// Optimized .Z (Unix Compress) Decoder implementation
pub struct ZDecoder<R: Read> {
    inner: R,
    eof: bool,
    max_bits: u8,
    block_mode: bool,
    current_bits: u8,
    max_code: u32,
    
    // Optimized table: (prefix_code, char)
    // Root codes 0-255 have prefix_code = u32::MAX
    prefixes: Vec<u32>,
    chars: Vec<u8>,
    
    prefix: u32,
    buffer: u64,
    bits_in_buffer: u8,
    
    output_buffer: Vec<u8>,
    output_pos: usize,
}

impl<R: Read> ZDecoder<R> {
    pub fn new(mut inner: R) -> Self {
        let mut header = [0u8; 3];
        if inner.read_exact(&mut header).is_err() || header[0] != 0x1f || header[1] != 0x9d {
            return Self::empty(inner);
        }

        let max_bits = header[2] & 0x1f;
        let block_mode = (header[2] & 0x80) != 0;

        let table_size = 1 << max_bits;
        let mut prefixes = Vec::with_capacity(table_size);
        let mut chars = Vec::with_capacity(table_size);

        for i in 0..256 {
            prefixes.push(u32::MAX);
            chars.push(i as u8);
        }
        if block_mode {
            prefixes.push(u32::MAX); // Code 256 for CLEAR
            chars.push(0);
        }

        Self {
            inner,
            eof: false,
            max_bits,
            block_mode,
            current_bits: 9,
            max_code: (1 << 9) - 1,
            prefixes,
            chars,
            prefix: u32::MAX,
            buffer: 0,
            bits_in_buffer: 0,
            output_buffer: Vec::with_capacity(1024),
            output_pos: 0,
        }
    }

    fn empty(inner: R) -> Self {
        Self {
            inner,
            eof: true,
            max_bits: 0,
            block_mode: false,
            current_bits: 0,
            max_code: 0,
            prefixes: vec![],
            chars: vec![],
            prefix: u32::MAX,
            buffer: 0,
            bits_in_buffer: 0,
            output_buffer: vec![],
            output_pos: 0,
        }
    }

    fn read_code(&mut self) -> io::Result<Option<u32>> {
        while self.bits_in_buffer < self.current_bits {
            let mut byte = [0u8; 1];
            if self.inner.read_exact(&mut byte).is_err() {
                return Ok(None);
            }
            self.buffer |= (byte[0] as u64) << self.bits_in_buffer;
            self.bits_in_buffer += 8;
        }

        let code = (self.buffer & ((1 << self.current_bits) - 1)) as u32;
        self.buffer >>= self.current_bits;
        self.bits_in_buffer -= self.current_bits;
        Ok(Some(code))
    }

    fn expand_code(prefixes: &[u32], chars: &[u8], code: u32, out: &mut Vec<u8>) {
        let mut curr = code;
        let start_idx = out.len();
        while curr != u32::MAX {
            out.push(chars[curr as usize]);
            curr = prefixes[curr as usize];
        }
        // Reverse the newly added sequence
        out[start_idx..].reverse();
    }
}

impl<R: Read> Read for ZDecoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut written = 0;
        
        while written < buf.len() {
            if self.output_pos < self.output_buffer.len() {
                let n = std::cmp::min(buf.len() - written, self.output_buffer.len() - self.output_pos);
                buf[written..written + n].copy_from_slice(&self.output_buffer[self.output_pos..self.output_pos + n]);
                written += n;
                self.output_pos += n;
                continue;
            }

            if self.eof { break; }

            match self.read_code()? {
                Some(code) => {
                    if self.block_mode && code == 256 {
                        self.prefixes.truncate(257);
                        self.chars.truncate(257);
                        self.current_bits = 9;
                        self.max_code = (1 << 9) - 1;
                        self.prefix = u32::MAX;
                        continue;
                    }

                    self.output_buffer.clear();
                    self.output_pos = 0;

                    if (code as usize) < self.prefixes.len() {
                        Self::expand_code(&self.prefixes, &self.chars, code, &mut self.output_buffer);
                    } else if code == self.prefixes.len() as u32 && self.prefix != u32::MAX {
                        Self::expand_code(&self.prefixes, &self.chars, self.prefix, &mut self.output_buffer);
                        let first_char = self.output_buffer[0];
                        self.output_buffer.push(first_char);
                    } else {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid LZW code"));
                    }

                    if self.prefix != u32::MAX && self.prefixes.len() < (1 << self.max_bits) {
                        let first_char_of_current = self.output_buffer[0];
                        self.prefixes.push(self.prefix);
                        self.chars.push(first_char_of_current);
                        
                        if self.prefixes.len() > self.max_code as usize && self.current_bits < self.max_bits {
                            self.current_bits += 1;
                            self.max_code = (1 << self.current_bits) - 1;
                        }
                    }
                    self.prefix = code;
                }
                None => {
                    self.eof = true;
                    break;
                }
            }
        }
        Ok(written)
    }
}
