use hmac_sha512::Hash;
use rand::Rng;

pub fn encode_password(password: &String, max_iterations: u8) -> String {
	
	let mut hasher = Hash::new();
	hasher.update(password.as_bytes());
	let mut content = hasher.finalize();

	let mut r = rand::thread_rng();
	for _ in 0..r.gen_range(0..max_iterations) {
		hasher = Hash::new();
		hasher.update(content);
		content = hasher.finalize();
	}

	hex::encode(content)

}

pub fn validate_password(password: &String, hash: &String, max_iterations: u8) -> bool {
	
	let mut hasher = Hash::new();
	hasher.update(password.as_bytes());
	let mut content = hasher.finalize();
	for _ in 0..max_iterations {
		if &hex::encode(content) == hash {
			return true;
		}
		hasher = Hash::new();
		hasher.update(content);
		content = hasher.finalize();
	}
	false

}
