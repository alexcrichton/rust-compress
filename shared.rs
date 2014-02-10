/*!

Common types and functions shared between algorithms.

*/

use std::io;

/// A writer that knows when to stop
pub trait FiniteWriter: Writer {
	/// mark the end of the stream
	fn write_terminator(&mut self) -> io::IoResult<()> {
		self.flush()
	}
}

impl Writer for ~FiniteWriter {
	fn write(&mut self, buf: &[u8]) -> io::IoResult<()> {
        self.write(buf)
    }
}

impl FiniteWriter for ~FiniteWriter {
	fn write_terminator(&mut self) -> io::IoResult<()> {
		self.write_terminator()
	}
}

impl FiniteWriter for io::MemWriter {}
impl FiniteWriter for io::stdio::StdWriter {}
impl<W: Writer> FiniteWriter for io::BufferedWriter<W> {}
