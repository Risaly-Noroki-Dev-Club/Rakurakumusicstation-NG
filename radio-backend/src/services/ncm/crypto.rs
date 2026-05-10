/// AES-128-ECB 加密/解密（网易云 Eapi 使用）

use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;
use aes::cipher::generic_array::GenericArray;

const EAPI_KEY: &str = "e82ckenh8dichen8";

fn generate_key(key: &[u8]) -> [u8; 16] {
    let mut gen_key = [0u8; 16];
    let len = key.len().min(16);
    gen_key[..len].copy_from_slice(&key[..len]);
    let mut i = 16;
    while i < key.len() {
        for j in 0..16 {
            if i < key.len() {
                gen_key[j] ^= key[i];
                i += 1;
            }
        }
    }
    gen_key
}

fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let pad_len = block_size - (data.len() % block_size);
    let mut result = data.to_vec();
    result.extend(std::iter::repeat(pad_len as u8).take(pad_len));
    result
}

fn pkcs7_unpad(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return data.to_vec();
    }
    let pad_len = data[data.len() - 1] as usize;
    if pad_len > data.len() || pad_len == 0 {
        return data.to_vec();
    }
    // verify padding
    for &b in &data[data.len() - pad_len..] {
        if b as usize != pad_len {
            return data.to_vec();
        }
    }
    data[..data.len() - pad_len].to_vec()
}

fn encrypt_ecb(data: &[u8], key_str: &str) -> Vec<u8> {
    let key = generate_key(key_str.as_bytes());
    let cipher = Aes128::new(GenericArray::from_slice(&key));
    let padded = pkcs7_pad(data, 16);
    let mut result = Vec::with_capacity(padded.len());

    for chunk in padded.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        result.extend_from_slice(&block);
    }
    result
}

fn decrypt_ecb(encrypted: &[u8], key_str: &str) -> Vec<u8> {
    let key = generate_key(key_str.as_bytes());
    let cipher = Aes128::new(GenericArray::from_slice(&key));
    let mut result = Vec::with_capacity(encrypted.len());

    for chunk in encrypted.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        result.extend_from_slice(&block);
    }
    pkcs7_unpad(&result)
}

pub fn eapi_encrypt(data: &str) -> Vec<u8> {
    encrypt_ecb(data.as_bytes(), EAPI_KEY)
}

pub fn eapi_decrypt(encrypted: &[u8]) -> Vec<u8> {
    decrypt_ecb(encrypted, EAPI_KEY)
}
