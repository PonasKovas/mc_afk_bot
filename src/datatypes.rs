use crate::{MyAsyncRead, MyAsyncWrite};
use async_trait::async_trait;
use tokio::io;

// A data type that is used in the minecraft protocol
// all info available on https://wiki.vg/index.php?title=Protocol
#[async_trait]
pub trait DataType {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()>;
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self>
    where
        Self: Sized;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarInt(pub i64);

#[derive(Clone, Debug)]
pub struct MString(pub String);

#[derive(Clone, Debug, Copy)]
pub enum Slot {
    Present(VarInt, i8), // this has NBT data too but we're not reading it
    NotPresent,
}

impl VarInt {
    pub fn size(&self) -> u8 {
        let mut bytes = 0;
        let mut temp = self.0;
        loop {
            bytes += 1;
            temp = temp >> 7;
            if temp == 0 {
                break;
            }
        }

        bytes
    }
}

// DataType implementations //
//////////////////////////////

#[async_trait]
impl DataType for VarInt {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        let mut number = (self.0 as u64) as i64;

        loop {
            let mut byte: u8 = number as u8 & 0b01111111;

            number = number >> 7;
            if number != 0 {
                byte = byte | 0b10000000;
            }

            output.write_byte(byte).await?;

            if number == 0 {
                break;
            }
        }

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let mut i = 0;
        let mut result: i64 = 0;
        loop {
            let number = input.read_byte().await?;

            let value = (number & 0b01111111) as i64;
            result = result | (value << (7 * i));

            if (number & 0b10000000) == 0 {
                break;
            }
            i += 1;
        }

        Ok(Self(result))
    }
}

#[async_trait]
impl DataType for MString {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        // string length as VarInt
        VarInt(self.0.len() as i64).serialize(output).await?;
        // the actual string bytes
        output.write(self.0.as_bytes()).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let string_length = VarInt::deserialize(input).await?;

        let mut string = vec![0; string_length.0 as usize];
        input.read(&mut string[..]).await?;
        let string = String::from_utf8_lossy(&string).into_owned();

        Ok(MString(string))
    }
}

#[async_trait]
impl DataType for Slot {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        match self {
            Slot::Present(id, number) => {
                true.serialize(output).await?;

                id.serialize(output).await?;
                number.serialize(output).await?;
            }
            Slot::NotPresent => {
                false.serialize(output).await?;
            }
        }

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        if bool::deserialize(input).await? {
            Ok(Self::Present(
                VarInt::deserialize(input).await?,
                i8::deserialize(input).await?,
            ))
        } else {
            Ok(Self::NotPresent)
        }
    }
}

#[async_trait]
impl<T: DataType + Send> DataType for Vec<T> {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        // vec length as VarInt
        let size = self.len();
        VarInt(size as i64).serialize(output).await?;
        // the actual data
        for item in self {
            item.serialize(output).await?;
        }

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let vec_size = VarInt::deserialize(input).await?;

        let mut data = Vec::with_capacity(vec_size.0 as usize);
        for _ in 0..vec_size.0 {
            data.push(T::deserialize(input).await?);
        }

        Ok(data)
    }
}

#[async_trait]
impl DataType for u16 {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        output.write(&mut self.to_be_bytes()).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let mut bytes = [0u8; 2];

        input.read(&mut bytes).await?;

        Ok(u16::from_be_bytes(bytes))
    }
}

#[async_trait]
impl DataType for i32 {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        output.write(&mut self.to_be_bytes()).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let mut bytes = [0u8; 4];

        input.read(&mut bytes).await?;

        Ok(i32::from_be_bytes(bytes))
    }
}

#[async_trait]
impl DataType for i16 {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        output.write(&mut self.to_be_bytes()).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let mut bytes = [0u8; 2];

        input.read(&mut bytes).await?;

        Ok(i16::from_be_bytes(bytes))
    }
}

#[async_trait]
impl DataType for i8 {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        output.write(&mut self.to_be_bytes()).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let mut bytes = [0u8; 1];

        input.read(&mut bytes).await?;

        Ok(i8::from_be_bytes(bytes))
    }
}

#[async_trait]
impl DataType for i64 {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        output.write(&mut self.to_be_bytes()).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let mut bytes = [0u8; 8];

        input.read(&mut bytes).await?;

        Ok(i64::from_be_bytes(bytes))
    }
}

#[async_trait]
impl DataType for f32 {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        output.write(&mut self.to_be_bytes()).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let mut bytes = [0u8; 4];

        input.read(&mut bytes).await?;

        Ok(f32::from_be_bytes(bytes))
    }
}

#[async_trait]
impl DataType for f64 {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        output.write(&mut self.to_be_bytes()).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let mut bytes = [0u8; 8];

        input.read(&mut bytes).await?;

        Ok(f64::from_be_bytes(bytes))
    }
}

#[async_trait]
impl DataType for u8 {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        output.write_byte(self).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        Ok(input.read_byte().await?)
    }
}

#[async_trait]
impl DataType for bool {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        output.write_byte(self as u8).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        Ok(input.read_byte().await? == 1)
    }
}

#[async_trait]
impl DataType for u128 {
    async fn serialize<O: MyAsyncWrite + 'static + Send>(self, output: &mut O) -> io::Result<()> {
        // nice format, mojang
        output
            .write(&mut ((self >> 64) as u64).to_be_bytes())
            .await?;
        output.write(&mut (self as u64).to_be_bytes()).await?;

        Ok(())
    }
    async fn deserialize<S: MyAsyncRead + 'static + Send>(input: &mut S) -> io::Result<Self> {
        let mut bytes = [0u8; 8];
        input.read(&mut bytes).await?;
        let mut number = (u64::from_be_bytes(bytes) as u128) << 64;

        let mut bytes = [0u8; 8];
        input.read(&mut bytes).await?;
        number |= u64::from_be_bytes(bytes) as u128;

        Ok(number)
    }
}
