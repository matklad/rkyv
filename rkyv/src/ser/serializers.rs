//! Serializers that can be used standalone and provide basic capabilities.

use crate::{
    ser::{SeekSerializer, Serializer},
    Fallible,
};
use core::ptr;
#[cfg(feature = "std")]
use std::io;

/// Wraps a byte buffer and equips it with [`Serializer`].
///
/// Common uses include archiving in `#![no_std]` environments and archiving
/// small objects without allocating.
///
/// ## Examples
/// ```
/// use rkyv::{
///     archived_value,
///     ser::{Serializer, serializers::BufferSerializer},
///     Aligned,
///     Archive,
///     Archived,
///     Serialize,
/// };
///
/// #[derive(Archive, Serialize)]
/// enum Event {
///     Spawn,
///     Speak(String),
///     Die,
/// }
///
/// let mut serializer = BufferSerializer::new(Aligned([0u8; 256]));
/// let pos = serializer.serialize_value(&Event::Speak("Help me!".to_string()))
///     .expect("failed to archive event");
/// let buf = serializer.into_inner();
/// let archived = unsafe { archived_value::<Event>(buf.as_ref(), pos) };
/// if let Archived::<Event>::Speak(message) = archived {
///     assert_eq!(message.as_str(), "Help me!");
/// } else {
///     panic!("archived event was of the wrong type");
/// }
/// ```
pub struct BufferSerializer<T> {
    inner: T,
    pos: usize,
}

impl<T> BufferSerializer<T> {
    /// Creates a new archive buffer from a byte buffer.
    pub fn new(inner: T) -> Self {
        Self::with_pos(inner, 0)
    }

    /// Creates a new archive buffer from a byte buffer. The buffer will start
    /// writing at the given position, but the buffer must contain all bytes
    /// (otherwise the alignments of types may not be correct).
    pub fn with_pos(inner: T, pos: usize) -> Self {
        Self { inner, pos }
    }

    /// Consumes the buffer and returns the internal buffer used to create it.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

/// The error type returned by an [`BufferSerializer`].
#[derive(Debug)]
pub enum BufferSerializerError {
    /// Writing has overflowed the internal buffer.
    Overflow {
        pos: usize,
        bytes_needed: usize,
        archive_len: usize,
    },
    /// The serializer sought past the end of the internal buffer.
    SoughtPastEnd {
        seek_position: usize,
        archive_len: usize,
    },
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Fallible for BufferSerializer<T> {
    type Error = BufferSerializerError;
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Serializer for BufferSerializer<T> {
    fn pos(&self) -> usize {
        self.pos
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let end_pos = self.pos + bytes.len();
        let archive_len = self.inner.as_ref().len();
        if end_pos > archive_len {
            Err(BufferSerializerError::Overflow {
                pos: self.pos,
                bytes_needed: bytes.len(),
                archive_len,
            })
        } else {
            unsafe {
                ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    self.inner.as_mut().as_mut_ptr().add(self.pos),
                    bytes.len(),
                );
            }
            self.pos = end_pos;
            Ok(())
        }
    }

    fn pad(&mut self, padding: usize) -> Result<(), Self::Error> {
        let end_pos = self.pos + padding;
        let archive_len = self.inner.as_ref().len();
        if end_pos > archive_len {
            Err(BufferSerializerError::Overflow {
                pos: self.pos,
                bytes_needed: padding,
                archive_len,
            })
        } else {
            self.pos = end_pos;
            Ok(())
        }
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> SeekSerializer for BufferSerializer<T> {
    fn seek(&mut self, pos: usize) -> Result<(), Self::Error> {
        let len = self.inner.as_ref().len();
        if pos > len {
            Err(BufferSerializerError::SoughtPastEnd {
                seek_position: pos,
                archive_len: len,
            })
        } else {
            self.pos = pos;
            Ok(())
        }
    }
}

/// Wraps a type that implements [`io::Write`](std::io::Write) and equips it
/// with [`Serializer`].
///
/// ## Examples
/// ```
/// use rkyv::ser::{serializers::WriteSerializer, Serializer};
///
/// let mut serializer = WriteSerializer::new(Vec::new());
/// assert_eq!(serializer.pos(), 0);
/// serializer.write(&[0u8, 1u8, 2u8, 3u8]);
/// assert_eq!(serializer.pos(), 4);
/// let buf = serializer.into_inner();
/// assert_eq!(buf.len(), 4);
/// assert_eq!(buf, vec![0u8, 1u8, 2u8, 3u8]);
/// ```
#[cfg(feature = "std")]
pub struct WriteSerializer<W: io::Write> {
    inner: W,
    pos: usize,
}

#[cfg(feature = "std")]
impl<W: io::Write> WriteSerializer<W> {
    /// Creates a new serializer from a writer.
    pub fn new(inner: W) -> Self {
        Self::with_pos(inner, 0)
    }

    /// Creates a new serializer from a writer, and assumes that the underlying
    /// writer is currently at the given position.
    pub fn with_pos(inner: W, pos: usize) -> Self {
        Self { inner, pos }
    }

    /// Consumes the serializer and returns the internal writer used to create
    /// it.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

#[cfg(feature = "std")]
impl<W: io::Write> Fallible for WriteSerializer<W> {
    type Error = io::Error;
}

#[cfg(feature = "std")]
impl<W: io::Write> Serializer for WriteSerializer<W> {
    fn pos(&self) -> usize {
        self.pos
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.pos += self.inner.write(bytes)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<W: io::Write + io::Seek> SeekSerializer for WriteSerializer<W> {
    fn seek(&mut self, offset: usize) -> Result<(), Self::Error> {
        self.inner.seek(io::SeekFrom::Start(offset as u64))?;
        self.pos = offset;
        Ok(())
    }
}
