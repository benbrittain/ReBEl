use crate::proto::Digest;

pub struct Blob {
    pub inner: Vec<u8>,
    pub digest: Digest,
}

impl Blob {
    pub fn from_proto<T: prost::Message>(data: T) -> Blob {
        let data = data.encode_to_vec();
        Blob::new(&data)
    }

    pub fn new(data: &[u8]) -> Blob {
        let one_shot = ring::digest::digest(&ring::digest::SHA256, data);
        let digest = hex::encode(one_shot);
        Blob {
            inner: data.into(),
            digest: Digest {
                hash: digest,
                size_bytes: data.len() as i64,
            },
        }
    }
}

pub struct Directory {
    pub name: String,
    pub directories: Vec<Directory>,
    pub files: Vec<File>,
}

impl Directory {
    pub fn new(name: String, directories: Vec<Directory>, files: Vec<File>) -> Self {
        Directory {
            name,
            directories,
            files,
        }
    }
}

pub struct File {
    pub name: String,
    pub data: Vec<u8>,
    pub is_executable: bool,
}

impl File {
    pub fn new(name: String, data: Vec<u8>) -> Self {
        File {
            name,
            data,
            is_executable: false,
        }
    }
}
