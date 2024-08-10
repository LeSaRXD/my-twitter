use hmac_sha512::Hash;
use rand::Rng;

pub fn encode_password(password: &str, max_iterations: u8) -> [u8; 64] {
	let mut hasher = Hash::new();
	hasher.update(password.as_bytes());
	let mut content = hasher.finalize();

	let mut r = rand::thread_rng();
	for _ in 0..r.gen_range(0..max_iterations) {
		hasher = Hash::new();
		hasher.update(content);
		content = hasher.finalize();
	}

	content
}

pub fn validate_password(password: &str, hash: &[u8], max_iterations: u8) -> bool {
	let mut hasher = Hash::new();
	hasher.update(password.as_bytes());
	let mut content = hasher.finalize();
	for _ in 0..max_iterations {
		if content == hash {
			return true;
		}
		hasher = Hash::new();
		hasher.update(content);
		content = hasher.finalize();
	}
	false
}
