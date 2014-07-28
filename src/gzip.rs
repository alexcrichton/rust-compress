//! Gzip Compression and Decompression
//!
//! This module contains an implementation of a Gzip decompressor, which uses
//! zlib streams.
//!
//! # Example
//!
//! ```rust
//! use compress::gzip;
//! use std::io::File;
//!
//! let stream = File::open(&Path::new("path/to/file.flate"));
//! let decompressed = gzip::Decoder::new(stream).read_to_end();
//! ```
//!
//! # Related links
//!
//! * http://tools.ietf.org/html/rfc1952 - RFC that this implementation is based
//!   on

use std::io;

use crc = checksum::crc;
use flate;

/// Structure used to decode a gzip-encoded stream/file. The wrapped stream can
/// be re-acquired through the unwrap() method.
pub struct Decoder<'a,R> {
    crc_table: &'a crc::Table32,
    r: flate::Decoder<R>
}

/// Reader for a stream member
pub struct Member<'a, 'b, R> {
    /// File name (may be empty). In theory this is ISO 8859-1 (LATIN-1)
    /// encoded; in practice I've seen UTF-8.
    pub file_name: Vec<u8>,
    /// File comment (may be empty). In theory this is ISO 8859-1 (LATIN-1)
    /// encoded; in practice I've seen UTF-8.
    pub file_comment: Vec<u8>,
    //TODO: probably the above should be converted to Strings and be movable
    
    crc: crc::State32<'a>,
    r: &'b mut flate::Decoder<R>,
    len: u32
}

macro_rules! try_no_eof (
    ($ex: expr) => (
        match $ex {
            Ok(result) => result,
            Err(ref e) if e.kind == io::EndOfFile => {
                return Err(io::IoError {
                    kind: io::InvalidInput,
                    desc: "unexpected end of file",
                    detail: None
                });
            },
            Err(e) => {
                return Err(e);
            }
        }
    );
)

// warnings should prevent dangerous/ambiguous code, not prevent extra clarity
#[allow(unnecessary_parens)]
impl<'a, R: Reader> Decoder<'a, R> {
    /* not possible without changing the type of crc_table, I think
    /// Creates a new gzip-stream decoder which will wrap the specified reader.
    /// This decoder also implements the `Reader` trait, and the underlying
    /// reader can be re-acquired through the `unwrap` method.
    pub fn new(reader: R) -> Decoder<R> {
        Decoder {
            crc_table: &crc::Table32::new(),
            r: flate::Decoder::new(reader)
        }
    }
    */
    
    /// Same as new(), except use an existing CRC table
    pub fn new_with_crc<'a>(reader: R, crc_table: &'a crc::Table32) -> Decoder<'a, R> {
        Decoder {
            crc_table: crc_table,
            r: flate::Decoder::new(reader)
        }
    }

    /// Destroys this decoder, returning the underlying reader.
    pub fn unwrap(self) -> R {
        self.r.r
    }
    
    /// Read a member from the gzip stream. If the stream is valid but ends
    /// here, EndOfFile is returned; all other errors should result in a
    /// different error code. Note: self will be frozen until the returned
    /// Member has been destroyed.
    pub fn member<'b>(&'b mut self) -> io::IoResult<Member<'a, 'b, R>> {
        // these values are assigned in the block below, but outlive it
        let fhcrc: bool;
        let crc: u32;
        let file_name: Vec<u8>;
        let file_comment: Vec<u8>;
        {
            // from here, all reads should go through this reader (not self.r):
            let mut crc_reader = crc::Reader32::new( &mut self.r, self.crc_table );
            
            let mut buf = [0u8, ..10];
            // read at least the first byte; EOF here is okay
            let len = try!(crc_reader.read_at_least(1, buf));
            if len < 10 {
                // read, interpreting EOF as an error
                try_no_eof!(crc_reader.read_at_least(10 - len, buf.mut_slice_from(len)));
            };
            
            if buf[0] != 0x1f || buf[1] != 0x8b {
                return Err(io::IoError {
                    kind: io::InvalidInput,
                    desc: "not a gzip stream",
                    detail: None
                })
            }
            
            let cm = buf[2];
            if cm != 0x8 {
                return Err(io::IoError {
                    kind: io::InvalidInput,
                    desc: "unsupport compression method",
                    detail: None,
                })
            }
            
            let flg = buf[3];
            // bit 0 FTEXT indicates ASCII (as opposed to binary); we can ignore this
            // bit 1 FHCRC indicates a CRC at the end of the header
            fhcrc = (flg & 2 != 0);
            // bit 2 FEXTRA indicates extra fields (below)
            let fextra = (flg & 4 != 0);
            // bit 3 FNAME indicates that the original file name is stored
            let fname = (flg & 8 != 0);
            // bit 4 FCOMMENT indicates that a comment is given
            let fcomment = (flg & 16 != 0);
            // bits 5-7 are reserved and must be zero
            if flg & (32 + 64 + 128) != 0 {
                return Err(io::IoError {
                    kind: io::InvalidInput,
                    desc: "reserved bits are set; refusing to read an unknown format",
                    detail: None
                })
            }
            
            //let mtime = read_le_u32 from buf ...
            // ignore XFL (buf[8]) and OS (buf[9])
            
            if fextra {
                let xlen = try_no_eof!(crc_reader.read_le_u16());
                // read and discard the "extra field"
                try_no_eof!(crc_reader.read_exact(xlen as uint));
            }
            
            let mut str_builder: Vec<u8> = Vec::new();
            if fname {
                loop {
                    let byte: u8 = try!(crc_reader.read_byte());
                    if byte == 0u8 {
                        break;
                    }
                    str_builder.push(byte);
                }
            }
            file_name = str_builder;
            
            str_builder = Vec::new();
            if fcomment {
                loop {
                    let byte: u8 = try!(crc_reader.read_byte());
                    if byte == 0u8 {
                        break;
                    }
                    str_builder.push(byte);
                }
            }
            file_comment = str_builder;
            
            crc = crc_reader.crc32();
        }       // destroy crc_reader; use self.r directly again
        
        if fhcrc {
            let crc16 = try_no_eof!(self.r.read_le_u16());
            if (crc & 0xFFFF) != crc16 as u32 {
                return Err(io::IoError {
                    kind: io::InvalidInput,
                    desc: "header checksum invalid",
                    detail: None
                })
            }
        }
        
        Ok(Member{
            file_name: file_name,
            file_comment: file_comment,
            crc: crc::State32::new(self.crc_table),
            len: 0,
            r: &mut self.r
        })
    }

    /// Tests if this stream has reached the EOF point yet.
    pub fn eof(&self) -> bool { self.r.eof() }
}

impl<'a, 'b, R: Reader> Reader for Member<'a, 'b, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::IoResult<uint> {
        match self.r.read(buf) {
            Ok(n) => {
                self.crc.feed(buf.slice_to(n));
                self.len += n as u32;
                Ok(n)
            }
            Err(ref e) if e.kind == io::EndOfFile => {
                let crc32 = try_no_eof!(self.r.r.read_le_u32());
                let isize = try_no_eof!(self.r.r.read_le_u32());
                if crc32 != self.crc.crc32() || isize != self.len {
                    return Err(io::IoError {
                        kind: io::InvalidInput,
                        desc: "invalid checksum on gzip stream",
                        detail: None,
                    })
                }
                return Err(e.clone())   // i.e. stream finished and valid
            }
            Err(e) => Err(e)
        }
    }
}
