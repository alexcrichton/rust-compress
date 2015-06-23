/*!

MTF (Move To Front) encoder/decoder
Produces a rank for each input character based on when it was seen last time.
Useful for BWT output encoding, which produces a lot of zeroes and low ranks.

# Links

http://en.wikipedia.org/wiki/Move-to-front_transform

# Example

```rust
use std::io::{self, Read, Write};
use compress::bwt::mtf;

// Encode a stream of bytes
let bytes = b"abracadabra";
let mut e = mtf::Encoder::new(io::BufWriter::new(Vec::new()));
e.write_all(bytes).unwrap();
let encoded = e.finish().into_inner().unwrap();

// Decode a stream of ranks
let mut d = mtf::Decoder::new(io::BufReader::new(&encoded[..]));
let mut decoded = Vec::new();
let result = d.read_to_end(&mut decoded).unwrap();
```

# Credit

*/

use std::mem;
use std::io::{self, Read, Write};

use super::super::byteorder::{self, WriteBytesExt, ReadBytesExt};

pub type Symbol = u8;
pub type Rank = u8;
pub const TOTAL_SYMBOLS: usize = 0x100;


/// MoveToFront encoder/decoder
pub struct MTF {
    /// rank-ordered list of unique Symbols
    pub symbols: [Symbol; TOTAL_SYMBOLS],
}

impl MTF {
    /// create a new zeroed MTF
    pub fn new() -> MTF {
        MTF { symbols: [0; TOTAL_SYMBOLS] }
    }

    /// set the order of symbols to be alphabetical
    pub fn reset_alphabetical(&mut self) {
        for (i,sym) in self.symbols.iter_mut().enumerate() {
            *sym = i as Symbol;
        }
    }

    /// encode a symbol into its rank
    pub fn encode(&mut self, sym: Symbol) -> Rank {
        let mut next = self.symbols[0];
        if next == sym {
            return 0
        }
        let mut rank: Rank = 1;
        loop {
            mem::swap(&mut self.symbols[rank as usize], &mut next);
            if next == sym {
                break;
            }
            rank += 1;
            assert!((rank as usize) < self.symbols.len());
        }
        self.symbols[0] = sym;
        rank
    }

    /// decode a rank into its symbol
    pub fn decode(&mut self, rank: Rank) -> Symbol {
        let sym = self.symbols[rank as usize];
        debug!("\tDecoding rank {} with symbol {}", rank, sym);
        for i in (0 .. rank as usize).rev() {
            self.symbols[i+1] = self.symbols[i];
        }
        self.symbols[0] = sym;
        sym
    }
}


/// A simple MTF stream encoder
pub struct Encoder<W> {
    w: W,
    mtf: MTF,
}

impl<W> Encoder<W> {
    /// start encoding into the given writer
    pub fn new(w: W) -> Encoder<W> {
        let mut mtf = MTF::new();
        mtf.reset_alphabetical();
        Encoder {
            w: w,
            mtf: mtf,
        }
    }

    /// finish encoding and return the wrapped writer
    pub fn finish(self) -> W {
        self.w
    }
}

impl<W: Write> Write for Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for sym in buf.iter() {
            let rank = self.mtf.encode(*sym);
            try!(self.w.write_u8(rank));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}


/// A simple MTF stream decoder
pub struct Decoder<R> {
    r: R,
    mtf: MTF,
}

impl<R> Decoder<R> {
    /// start decoding the given reader
    pub fn new(r: R) -> Decoder<R> {
        let mut mtf = MTF::new();
        mtf.reset_alphabetical();
        Decoder {
            r: r,
            mtf: mtf,
        }
    }

    /// finish decoder and return the wrapped reader
    pub fn finish(self) -> R {
        self.r
    }
}

impl<R: Read> Read for Decoder<R> {
    fn read(&mut self, dst: &mut [u8]) -> io::Result<usize> {
        let mut bytes_read = 0;
        for sym in dst.iter_mut() {
            let rank = match self.r.read_u8() {
                Ok(r) => r,
                Err(byteorder::Error::UnexpectedEOF) => break,
                Err(byteorder::Error::Io(e)) => return Err(e)
            };
            bytes_read += 1;
            *sym = self.mtf.decode(rank);
        }
        Ok((bytes_read))
    }
}


#[cfg(test)]
mod test {
    use std::io::{self, Read, Write};
    #[cfg(feature="unstable")]
    use test::Bencher;
    use super::{Encoder, Decoder};

    fn roundtrip(bytes: &[u8]) {
        info!("Roundtrip MTF of size {}", bytes.len());
        let buf = Vec::new();
        let mut e = Encoder::new(io::BufWriter::new(buf));
        e.write_all(bytes).unwrap();
        let encoded = e.finish().into_inner().unwrap();
        debug!("Roundtrip MTF input: {:?}, ranks: {:?}", bytes, encoded);
        let mut d = Decoder::new(io::BufReader::new(&encoded[..]));
        let mut decoded = Vec::new();
        d.read_to_end(&mut decoded).unwrap();
        assert_eq!(&decoded[..], bytes);
    }

    #[test]
    fn some_roundtrips() {
        roundtrip(b"teeesst_mtf");
        roundtrip(b"");
        roundtrip(include_bytes!("../data/test.txt"));
    }

    #[cfg(feature="unstable")]
    #[bench]
    fn encode_speed(bh: &mut Bencher) {
        let vec = Vec::new();
        let input = include_bytes!("../data/test.txt");
        let mem = io::BufWriter::with_capacity(input.len(), vec);
        let mut e = Encoder::new(mem);
        bh.iter(|| {
            e.write_all(input).unwrap();
        });
        bh.bytes = input.len() as u64;
    }

    #[cfg(feature="unstable")]
    #[bench]
    fn decode_speed(bh: &mut Bencher) {
        let vec = Vec::new();
        let input = include_bytes!("../data/test.txt");
        let mut e = Encoder::new(io::BufWriter::new(vec));
        e.write_all(input).unwrap();
        let encoded = e.finish().into_inner().unwrap();
        bh.iter(|| {
            let mut d = Decoder::new(io::BufReader::new(&encoded[..]));
            let mut buf = Vec::new();
            d.read_to_end(&mut buf).unwrap();
        });
        bh.bytes = input.len() as u64;
    }
}
