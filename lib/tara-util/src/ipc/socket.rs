//! The communication protocol and implementation for communication across
//! [`interprocess::local_socket::tokio::LocalSocketStream`]. The protocol and
//! implementation is simple:
//!
//! ## Writing
//!
//! 1. Serialize the data with [`bincode::serialize`]
//! 2. Write the size of the serialized data as an [`u32`] to the socket (panics if the
//! size > 4GiB, a.k.a. `u32::MAX`).
//! 3. Write the serialized data to the socket.
//!
//! ## Reading
//!
//! 1. Read the size of the incoming data as a [`u32`].
//! 2. Read exactly the number of bytes denoted by the size to get the data.
//! 3. Deserialize with [`bincode::deserialize`].

use async_trait::async_trait;
use byteorder_async::LittleEndian;
use futures_lite::{io::AsyncReadExt, AsyncWriteExt};

use crate::error::IpcErr;

#[async_trait]
pub trait SocketExt {
    async fn read_serde<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, IpcErr>;
    async fn write_serde<T: serde::Serialize + Send>(&mut self, data: T) -> Result<(), IpcErr>;
}

#[async_trait]
impl<R: AsyncReadExt + AsyncWriteExt + Unpin + Send> SocketExt for R {
    /// Read a serializable object from the socket. 4Gib maximum due to `u32::MAX`.
    async fn read_serde<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, IpcErr> {
        use byteorder_async::ReaderToByteOrder;
        let size = self.byte_order().read_u32::<LittleEndian>().await?;

        let mut bytes = vec![0; size as usize];
        self.read_exact(&mut bytes).await?;

        Ok(bincode::deserialize(&bytes)?)
    }

    /// Write a serializable object to the socket. 4Gib maximum due to `u32::MAX`. If the
    /// size is greater than `u32::MAX` then this function will panic.
    async fn write_serde<T: serde::Serialize + Send>(&mut self, data: T) -> Result<(), IpcErr> {
        use byteorder_async::WriterToByteOrder;
        let bytes = bincode::serialize(&data)?;

        self.byte_order()
            .write_u32::<LittleEndian>(u32::try_from(bytes.len()).unwrap())
            .await?;
        self.write_all(&bytes).await?;

        Ok(())
    }
}
