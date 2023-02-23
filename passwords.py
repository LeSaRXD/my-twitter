from hashlib import sha256
from random import randint

def encode_pw(pw, max_iterations: int = 100):
	hashed_pw = pw.encode()

	for i in range(randint(1, max_iterations)):
		hashed_pw = sha256(hashed_pw).digest()
	
	return (sha256(hashed_pw).hexdigest(), i)

def validate_pw(pw, hashed_pw, max_iterations: int = 100):
	tried_pw = pw.encode()
	for i in range(max_iterations):
		tried_pw = sha256(tried_pw).digest()
		if sha256(tried_pw).hexdigest() == hashed_pw:
			return True
	
	return False
