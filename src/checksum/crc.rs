//! Cyclic Redundancy Check (CRC): ISO 3309, ITU-T V.42
//! This implementation is based off of
//! http://tools.ietf.org/html/rfc1952#section-8

use std::io;

/// A table required for efficient CRC computation. This is immutable and can
/// be reused.
pub struct Table32 {
    table: [u32, ..256]
}

impl Table32 {
    /// Initialise a CRC table.
    pub fn new() -> Table32 {
        let mut table : [u32, ..256] = [0u32, ..256];
        for n in range(0u, 256) {
            let mut c = n as u32;
            for _ in range(0u32, 8) {
                c = if c & 1 != 0 {
                    0xedb88320u32 ^ (c >> 1)
                }else{
                    c >> 1
                }
            }
            table[n] = c;
        }
        Table32{ table: table }
    }
}

/// State used during checksum calculation
pub struct State32<'a> {
    crc: u32,
    table: &'a Table32
}

impl<'a> State32<'a> {
    /// Create a new state for calculating a CRC.
    pub fn new<'a>(table: &'a Table32) -> State32<'a> {
        State32 {
            crc: 0xffffffffu32,
            table: table
        }
    }
    
    /// Get checksum
    pub fn crc32(&self) -> u32 {
        self.crc ^ 0xffffffffu32
    }
    
    /// Reset CRC (i.e. new CRC starts with next read)
    pub fn reset(&mut self) {
        self.crc = 0xffffffffu32;
    }
    
    /// Feed the CRC-calculator with more data.
    pub fn feed(&mut self, buf: &[u8]) {
        let mut c = self.crc;
        for n in range(0, buf.len()) {
            c = self.table.table[((c ^ buf[n] as u32) & 0xff) as uint] ^ (c >> 8);
        }
        self.crc = c;
    }
}

/// A reader which calculates a checksum as it goes
pub struct Reader32<'a, 'b, R> {
    inner: &'b mut R,
    state: State32<'a>
}

impl<'a, 'b, R: Reader> Reader32<'a, 'b, R> {
    /// Create a new state
    pub fn new<'a, 'b>(reader: &'b mut R, table: &'a Table32) -> Reader32<'a, 'b, R> {
        Reader32 {
            inner: reader,
            state: State32::new(table)
        }
    }
    
    /// Get checksum
    pub fn crc32(&self) -> u32 {
        self.state.crc32()
    }
    
    /// Reset CRC (i.e. new CRC starts with next read)
    pub fn reset(&mut self) {
        self.state.reset()
    }
    
    /// Destroy self, returning the used reader.
    pub fn unwrap(self) -> &'b mut R {
        self.inner
    }
}

impl <'a, 'b, R: Reader> Reader for Reader32<'a, 'b, R>{
    fn read(&mut self, buf: &mut [u8]) -> io::IoResult<uint> {
        let len = try!(self.inner.read(buf));
        self.state.feed(buf.slice_to(len));
        Ok(len)
    }
}
