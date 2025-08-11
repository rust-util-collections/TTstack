//!
//! # Common Utils
//!

#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

/// (de)compression
pub mod zlib {
    use flate2::{
        write::{ZlibDecoder, ZlibEncoder},
        Compression,
    };
    use ruc::*;
    use std::io::Write;

    /// compress
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        let mut en = ZlibEncoder::new(Vec::new(), Compression::default());
        en.write_all(data).c(d!())?;
        en.finish().c(d!())
    }

    /// decompress
    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        let mut d = ZlibDecoder::new(Vec::new());
        d.write_all(data).c(d!()).and_then(|_| d.finish().c(d!()))
    }

    #[cfg(test)]
    mod tests {
        use rand::random;

        #[test]
        fn it_works() {
            (0..(10 + random::<u8>() % 20))
                .map(|i| (0..i).map(|_| random::<u8>()).collect::<Vec<_>>())
                .for_each(|sample| {
                    assert_eq!(sample, super::decode(&super::encode(&sample).unwrap()).unwrap());
                });
        }
    }
}
