#!/usr/bin/python3
"""
Little integration testing script while proper integration tests in Rust aren't implemented.
"""

import magic, os, hashlib

def make_random_file():
	with open('test-file', 'wb') as fout:
		fout.write(os.urandom(2048))

def sanity_check_format(format: str):
	make_random_file()
	md5sum = hashlib.md5(open('test-file', 'rb').read()).hexdigest()
	os.system(f"cargo run -- -i test-file -o test-file.{format}")
	os.remove('test-file')
	os.system(f"cargo run -- -i test-file.{format}")
	if md5sum != hashlib.md5(open('test-file', 'rb').read()).hexdigest():
		print("Something went wrong with tar (de)compression.")
		os._exit(2)
	os.remove('test-file')
	os.remove(f'test-file.{format}')


if __name__ == "__main__":

	# We'll use MIME sniffing through magic numbers to 
	# verify if ouch is actually outputting the file formats
	# that it should

	m = magic.open(magic.MAGIC_MIME)

	try:
		os.mkdir("testbuilds")
	except OSError:
		print ("Could not make testbuilds folder. Exiting.")
		os._exit(2)

	os.chdir("testbuilds")

	m.load()
	files = [
		"src.tar",
		"src.zip",
		"src.tar.gz",
		"src.tar.bz",
		"src.tar.bz2",
		"src.tar.lz",
		"src.tar.lzma",
	]

	expected_mime_types = [
		"application/x-tar",
		"application/zip",
		"application/gzip",
		"application/x-bzip2",
		"application/x-bzip2",
		
		# TODO: Is this right?
		# Perhaps the output should be application/x-lzma
		"application/octet-stream",
		"application/octet-stream"
	]

	for file in files:
		rv = os.system(f"cargo run -- compress ../src/ {file}")
		if rv != 0:
			print(f"Failed while compressing {file}")
			os._exit(2)

	for (file, expected_mime) in zip(files, expected_mime_types):
		if m.file(file) != expected_mime: 
			print(f"Test failed at file {file}.")
			print(f"Got: {m.file(file)}.")
			print(f"Expected: {expected_mime}.")
			os._exit(2)

	for (idx, file) in enumerate(files):
		rv = os.system(f"cargo run -- {file} -o out{idx}/")
		if rv != 0:
			print(f"Failed while decompressing {file}")		
			os._exit(2)

	# os.chdir("..")
	# os.system("rm -rf testbuilds")

	# # We'll now verify if ouch is not altering the data it is compressing
	# # and decompressing

	# sanity_check_format("zip")
	# sanity_check_format("tar")
	# sanity_check_format("tar.gz")
	# sanity_check_format("tar.bz")
	# sanity_check_format("tar.bz2")
	# sanity_check_format("tar.lz")
	# sanity_check_format("tar.lzma")