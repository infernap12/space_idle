use aes::Aes256;
use base64::Engine;
use cfb_mode::cipher::AsyncStreamCipher;
use cfb_mode::{
	Decryptor,
	cipher::{KeyIvInit, StreamCipher},
};
use flate2::read::GzDecoder;
use std::{
	env,
	fs::File,
	io::{BufRead, BufReader, Read},
};

type Aes256CfbDec = Decryptor<Aes256>;

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() != 3 {
		eprintln!("Usage: {} <encrypted_file> <wordlist>", args[0]);
		return;
	}

	let encrypted = parse_godot_file(&args[1]);
	let wordlist = File::open(&args[2]).expect("Failed to open wordlist");

	for line in BufReader::new(wordlist).lines() {
		let candidate = line.expect("Failed to read line");
		let key = password_to_key(&candidate);

		let mut buffer = encrypted.ciphertext.clone();
		let mut iv_copy = encrypted.iv;

		let mut decryptor = Aes256CfbDec::new(&key.into(), &iv_copy.into());
		decryptor.decrypt(&mut buffer);

		buffer.truncate(encrypted.original_length as usize);

		let hash = md5::compute(&buffer);
		if hash.0 == encrypted.expected_md5 {
			println!("Found password: {}", candidate);

			// Base64 decode
			let decoded = base64::engine::general_purpose::STANDARD
				.decode(&buffer)
				.expect("Failed to base64 decode");

			// Gzip decompress
			let mut decoder = GzDecoder::new(decoded.as_slice());
			let mut decompressed = Vec::new();
			decoder
				.read_to_end(&mut decompressed)
				.expect("Failed to gzip decompress");

			let out_path = format!("{}.decrypted", args[1]);
			std::fs::write(&out_path, &decompressed).expect("Failed to write output");
			println!("Wrote to: {}", out_path);
			return;
		}
	}

	println!("Password not found in wordlist");
}

fn parse_godot_file(path: &str) -> EncryptedFile {
	let mut file = File::open(path).expect("Failed to open encrypted file");

	// Skip magic header (4 bytes)
	let mut magic = [0u8; 4];
	file.read_exact(&mut magic).unwrap();

	// MD5 of plaintext (16 bytes)
	let mut expected_md5 = [0u8; 16];
	file.read_exact(&mut expected_md5).unwrap();

	// Original length (8 bytes, little-endian)
	let mut len_bytes = [0u8; 8];
	file.read_exact(&mut len_bytes).unwrap();
	let original_length = u64::from_le_bytes(len_bytes);

	// IV (16 bytes)
	let mut iv = [0u8; 16];
	file.read_exact(&mut iv).unwrap();

	// Rest is ciphertext
	let mut ciphertext = Vec::new();
	file.read_to_end(&mut ciphertext).unwrap();

	EncryptedFile {
		expected_md5,
		original_length,
		iv,
		ciphertext,
	}
}

fn password_to_key(password: &str) -> [u8; 32] {
	let digest = md5::compute(password.as_bytes());
	let hex = format!("{:x}", digest);
	let mut key = [0u8; 32];
	key.copy_from_slice(hex.as_bytes());
	key
}

struct EncryptedFile {
	expected_md5: [u8; 16],
	original_length: u64,
	iv: [u8; 16],
	ciphertext: Vec<u8>,
}
