use std::iter::repeat;
use libc::c_int;

use crypto::symm_internal::evpc;
use ffi;

#[derive(Copy, Clone)]
pub enum Mode {
    Encrypt,
    Decrypt,
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone)]
pub enum Type {
    AES_128_ECB,
    AES_128_CBC,
    /// Requires the `aes_xts` feature
    #[cfg(feature = "aes_xts")]
    AES_128_XTS,
    #[cfg(feature = "aes_ctr")]
    AES_128_CTR,
    // AES_128_GCM,
    AES_128_CFB1,
    AES_128_CFB128,
    AES_128_CFB8,

    AES_256_ECB,
    AES_256_CBC,
    /// Requires the `aes_xts` feature
    #[cfg(feature = "aes_xts")]
    AES_256_XTS,
    #[cfg(feature = "aes_ctr")]
    AES_256_CTR,
    // AES_256_GCM,
    AES_256_CFB1,
    AES_256_CFB128,
    AES_256_CFB8,

    DES_CBC,
    DES_ECB,

    RC4_128,
}


/// Represents a symmetric cipher context.
pub struct Crypter {
    evp: *const ffi::EVP_CIPHER,
    ctx: *mut ffi::EVP_CIPHER_CTX,
    keylen: u32,
    blocksize: u32,
}

impl Crypter {
    pub fn new(t: Type) -> Crypter {
        ffi::init();

        let ctx = unsafe { ffi::EVP_CIPHER_CTX_new() };
        let (evp, keylen, blocksz) = evpc(t);
        Crypter {
            evp: evp,
            ctx: ctx,
            keylen: keylen,
            blocksize: blocksz,
        }
    }

    /**
     * Enables or disables padding. If padding is disabled, total amount of
     * data encrypted must be a multiple of block size.
     */
    pub fn pad(&self, padding: bool) {
        if self.blocksize > 0 {
            unsafe {
                let v = if padding {
                    1 as c_int
                } else {
                    0
                };
                ffi::EVP_CIPHER_CTX_set_padding(self.ctx, v);
            }
        }
    }

    /**
     * Initializes this crypter.
     */
    pub fn init(&self, mode: Mode, key: &[u8], iv: &[u8]) {
        unsafe {
            let mode = match mode {
                Mode::Encrypt => 1 as c_int,
                Mode::Decrypt => 0 as c_int,
            };
            assert_eq!(key.len(), self.keylen as usize);

            ffi::EVP_CipherInit(self.ctx, self.evp, key.as_ptr(), iv.as_ptr(), mode);
        }
    }

    /**
     * Update this crypter with more data to encrypt or decrypt. Returns
     * encrypted or decrypted bytes.
     */
    pub fn update(&self, data: &[u8]) -> Vec<u8> {
        unsafe {
            let sum = data.len() + (self.blocksize as usize);
            let mut res = repeat(0u8).take(sum).collect::<Vec<_>>();
            let mut reslen = sum as c_int;

            ffi::EVP_CipherUpdate(self.ctx,
                                  res.as_mut_ptr(),
                                  &mut reslen,
                                  data.as_ptr(),
                                  data.len() as c_int);

            res.truncate(reslen as usize);
            res
        }
    }

    /**
     * Finish crypting. Returns the remaining partial block of output, if any.
     */
    pub fn finalize(&self) -> Vec<u8> {
        unsafe {
            let mut res = repeat(0u8).take(self.blocksize as usize).collect::<Vec<_>>();
            let mut reslen = self.blocksize as c_int;

            ffi::EVP_CipherFinal(self.ctx, res.as_mut_ptr(), &mut reslen);

            res.truncate(reslen as usize);
            res
        }
    }
}

impl Drop for Crypter {
    fn drop(&mut self) {
        unsafe {
            ffi::EVP_CIPHER_CTX_free(self.ctx);
        }
    }
}

/**
 * Encrypts data, using the specified crypter type in encrypt mode with the
 * specified key and iv; returns the resulting (encrypted) data.
 */
pub fn encrypt(t: Type, key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    let c = Crypter::new(t);
    c.init(Mode::Encrypt, key, iv);
    let mut r = c.update(data);
    let rest = c.finalize();
    r.extend(rest.into_iter());
    r
}

/**
 * Decrypts data, using the specified crypter type in decrypt mode with the
 * specified key and iv; returns the resulting (decrypted) data.
 */
pub fn decrypt(t: Type, key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    let c = Crypter::new(t);
    c.init(Mode::Decrypt, key, iv);
    let mut r = c.update(data);
    let rest = c.finalize();
    r.extend(rest.into_iter());
    r
}

#[cfg(test)]
mod tests {
    use serialize::hex::FromHex;

    // Test vectors from FIPS-197:
    // http://csrc.nist.gov/publications/fips/fips197/fips-197.pdf
    #[test]
    fn test_aes_256_ecb() {
        let k0 = [0x00u8, 0x01u8, 0x02u8, 0x03u8, 0x04u8, 0x05u8, 0x06u8, 0x07u8, 0x08u8, 0x09u8,
                  0x0au8, 0x0bu8, 0x0cu8, 0x0du8, 0x0eu8, 0x0fu8, 0x10u8, 0x11u8, 0x12u8, 0x13u8,
                  0x14u8, 0x15u8, 0x16u8, 0x17u8, 0x18u8, 0x19u8, 0x1au8, 0x1bu8, 0x1cu8, 0x1du8,
                  0x1eu8, 0x1fu8];
        let p0 = [0x00u8, 0x11u8, 0x22u8, 0x33u8, 0x44u8, 0x55u8, 0x66u8, 0x77u8, 0x88u8, 0x99u8,
                  0xaau8, 0xbbu8, 0xccu8, 0xddu8, 0xeeu8, 0xffu8];
        let c0 = [0x8eu8, 0xa2u8, 0xb7u8, 0xcau8, 0x51u8, 0x67u8, 0x45u8, 0xbfu8, 0xeau8, 0xfcu8,
                  0x49u8, 0x90u8, 0x4bu8, 0x49u8, 0x60u8, 0x89u8];
        let c = super::Crypter::new(super::Type::AES_256_ECB);
        c.init(super::Mode::Encrypt, &k0, &[]);
        c.pad(false);
        let mut r0 = c.update(&p0);
        r0.extend(c.finalize().into_iter());
        assert!(r0 == c0);
        c.init(super::Mode::Decrypt, &k0, &[]);
        c.pad(false);
        let mut p1 = c.update(&r0);
        p1.extend(c.finalize().into_iter());
        assert!(p1 == p0);
    }

    #[test]
    fn test_aes_256_cbc_decrypt() {
        let cr = super::Crypter::new(super::Type::AES_256_CBC);
        let iv = [4_u8, 223_u8, 153_u8, 219_u8, 28_u8, 142_u8, 234_u8, 68_u8, 227_u8, 69_u8,
                  98_u8, 107_u8, 208_u8, 14_u8, 236_u8, 60_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8,
                  0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8];
        let data = [143_u8, 210_u8, 75_u8, 63_u8, 214_u8, 179_u8, 155_u8, 241_u8, 242_u8, 31_u8,
                    154_u8, 56_u8, 198_u8, 145_u8, 192_u8, 64_u8, 2_u8, 245_u8, 167_u8, 220_u8,
                    55_u8, 119_u8, 233_u8, 136_u8, 139_u8, 27_u8, 71_u8, 242_u8, 119_u8, 175_u8,
                    65_u8, 207_u8];
        let ciphered_data = [0x4a_u8, 0x2e_u8, 0xe5_u8, 0x6_u8, 0xbf_u8, 0xcf_u8, 0xf2_u8,
                             0xd7_u8, 0xea_u8, 0x2d_u8, 0xb1_u8, 0x85_u8, 0x6c_u8, 0x93_u8,
                             0x65_u8, 0x6f_u8];
        cr.init(super::Mode::Decrypt, &data, &iv);
        cr.pad(false);
        let unciphered_data_1 = cr.update(&ciphered_data);
        let unciphered_data_2 = cr.finalize();

        let expected_unciphered_data = b"I love turtles.\x01";

        assert!(unciphered_data_2.len() == 0);

        assert_eq!(&unciphered_data_1, expected_unciphered_data);
    }

    fn cipher_test(ciphertype: super::Type, pt: &str, ct: &str, key: &str, iv: &str) {
        use serialize::hex::ToHex;

        let cipher = super::Crypter::new(ciphertype);
        cipher.init(super::Mode::Encrypt,
                    &key.from_hex().unwrap(),
                    &iv.from_hex().unwrap());

        let expected = ct.from_hex().unwrap();
        let mut computed = cipher.update(&pt.from_hex().unwrap());
        computed.extend(cipher.finalize().into_iter());

        if computed != expected {
            println!("Computed: {}", computed.to_hex());
            println!("Expected: {}", expected.to_hex());
            if computed.len() != expected.len() {
                println!("Lengths differ: {} in computed vs {} expected",
                         computed.len(),
                         expected.len());
            }
            panic!("test failure");
        }
    }

    #[test]
    fn test_rc4() {

        let pt = "0000000000000000000000000000000000000000000000000000000000000000000000000000";
        let ct = "A68686B04D686AA107BD8D4CAB191A3EEC0A6294BC78B60F65C25CB47BD7BB3A48EFC4D26BE4";
        let key = "97CD440324DA5FD1F7955C1C13B6B466";
        let iv = "";

        cipher_test(super::Type::RC4_128, pt, ct, key, iv);
    }

    #[test]
    #[cfg(feature = "aes_xts")]
    fn test_aes256_xts() {
        // Test case 174 from
        // http://csrc.nist.gov/groups/STM/cavp/documents/aes/XTSTestVectors.zip
        let pt = "77f4ef63d734ebd028508da66c22cdebdd52ecd6ee2ab0a50bc8ad0cfd692ca5fcd4e6dedc45df7f\
                  6503f462611dc542";
        let ct = "ce7d905a7776ac72f240d22aafed5e4eb7566cdc7211220e970da634ce015f131a5ecb8d400bc9e8\
                  4f0b81d8725dbbc7";
        let key = "b6bfef891f83b5ff073f2231267be51eb084b791fa19a154399c0684c8b2dfcb37de77d28bbda3b\
                   4180026ad640b74243b3133e7b9fae629403f6733423dae28";
        let iv = "db200efb7eaaa737dbdf40babb68953f";

        cipher_test(super::Type::AES_256_XTS, pt, ct, key, iv);
    }

    #[test]
    #[cfg(feature = "aes_ctr")]
    fn test_aes128_ctr() {

        let pt = "6BC1BEE22E409F96E93D7E117393172AAE2D8A571E03AC9C9EB76FAC45AF8E5130C81C46A35CE411\
                  E5FBC1191A0A52EFF69F2445DF4F9B17AD2B417BE66C3710";
        let ct = "874D6191B620E3261BEF6864990DB6CE9806F66B7970FDFF8617187BB9FFFDFF5AE4DF3EDBD5D35E\
                  5B4F09020DB03EAB1E031DDA2FBE03D1792170A0F3009CEE";
        let key = "2B7E151628AED2A6ABF7158809CF4F3C";
        let iv = "F0F1F2F3F4F5F6F7F8F9FAFBFCFDFEFF";

        cipher_test(super::Type::AES_128_CTR, pt, ct, key, iv);
    }

    // #[test]
    // fn test_aes128_gcm() {
    // Test case 3 in GCM spec
    // let pt = ~"d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255";
    // let ct = ~"42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091473f59854d5c2af327cd64a62cf35abd2ba6fab4";
    // let key = ~"feffe9928665731c6d6a8f9467308308";
    // let iv = ~"cafebabefacedbaddecaf888";
    //
    // cipher_test(super::AES_128_GCM, pt, ct, key, iv);
    // }

    #[test]
    fn test_aes128_cfb1() {
        // Lifted from http://csrc.nist.gov/publications/nistpubs/800-38a/sp800-38a.pdf

        let pt = "6bc1";
        let ct = "68b3";
        let key = "2b7e151628aed2a6abf7158809cf4f3c";
        let iv = "000102030405060708090a0b0c0d0e0f";

        cipher_test(super::Type::AES_128_CFB1, pt, ct, key, iv);
    }

    #[test]
    fn test_aes128_cfb128() {

        let pt = "6bc1bee22e409f96e93d7e117393172a";
        let ct = "3b3fd92eb72dad20333449f8e83cfb4a";
        let key = "2b7e151628aed2a6abf7158809cf4f3c";
        let iv = "000102030405060708090a0b0c0d0e0f";

        cipher_test(super::Type::AES_128_CFB128, pt, ct, key, iv);
    }

    #[test]
    fn test_aes128_cfb8() {

        let pt = "6bc1bee22e409f96e93d7e117393172aae2d";
        let ct = "3b79424c9c0dd436bace9e0ed4586a4f32b9";
        let key = "2b7e151628aed2a6abf7158809cf4f3c";
        let iv = "000102030405060708090a0b0c0d0e0f";

        cipher_test(super::Type::AES_128_CFB8, pt, ct, key, iv);
    }

    #[test]
    fn test_aes256_cfb1() {

        let pt = "6bc1";
        let ct = "9029";
        let key = "603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4";
        let iv = "000102030405060708090a0b0c0d0e0f";

        cipher_test(super::Type::AES_256_CFB1, pt, ct, key, iv);
    }

    #[test]
    fn test_aes256_cfb128() {

        let pt = "6bc1bee22e409f96e93d7e117393172a";
        let ct = "dc7e84bfda79164b7ecd8486985d3860";
        let key = "603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4";
        let iv = "000102030405060708090a0b0c0d0e0f";

        cipher_test(super::Type::AES_256_CFB128, pt, ct, key, iv);
    }

    #[test]
    fn test_aes256_cfb8() {

        let pt = "6bc1bee22e409f96e93d7e117393172aae2d";
        let ct = "dc1f1a8520a64db55fcc8ac554844e889700";
        let key = "603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4";
        let iv = "000102030405060708090a0b0c0d0e0f";

        cipher_test(super::Type::AES_256_CFB8, pt, ct, key, iv);
    }

    #[test]
    fn test_des_cbc() {

        let pt = "54686973206973206120746573742e";
        let ct = "6f2867cfefda048a4046ef7e556c7132";
        let key = "7cb66337f3d3c0fe";
        let iv = "0001020304050607";

        cipher_test(super::Type::DES_CBC, pt, ct, key, iv);
    }

    #[test]
    fn test_des_ecb() {

        let pt = "54686973206973206120746573742e";
        let ct = "0050ab8aecec758843fe157b4dde938c";
        let key = "7cb66337f3d3c0fe";
        let iv = "0001020304050607";

        cipher_test(super::Type::DES_ECB, pt, ct, key, iv);
    }
}
