#!/usr/bin/python3

"""
Integration testing script for Ouch
"""

import os
from subprocess import DEVNULL, STDOUT, check_call
from hashlib import sha3_384
from tempfile import TemporaryDirectory, NamedTemporaryFile, tempdir
import magic

def run(command: str):
    # Will raise an Exception if running the command fails
    check_call(command.split(), stdout=DEVNULL, stderr=STDOUT)

def compress(to_be_compressed: str, destination: str):
    run(f"cargo run --release -- c {to_be_compressed} {destination}")

def decompress(to_decompress: str):
    run(f"cargo run --release -- d {to_decompress}")

# We'll use MIME sniffing through magic numbers to 
# verify if ouch is actually outputting the file formats
# that it should
def compression_test_suite(tmp_dir: TemporaryDirectory):
    print("Running MIME type-based compression test", end='')

    db = magic.Magic(mime = True)
    files = [
		"src.tar",
		"src.zip",
		"src.tar.gz",
		"src.tar.bz",
		"src.tar.bz2",
		"src.tar.lzma",
	]

    expected_mime_types = [
        "application/x-tar",
        "application/zip",
        "application/gzip",
        "application/x-bzip2",
        "application/x-bzip2",
        "application/x-xz"
    ]

    for (file, expected) in zip(files, expected_mime_types):
        compress("../src", file)
        assert db.from_file(file) == expected
        os.remove(file)
    print("... ok")

def open_and_checksum(path: str):
    return sha3_384(open(path, 'rb').read()).hexdigest()

def decompression_test_suite(tmp_dir: TemporaryDirectory):
    print("Running SHA3-384-based decompression test", end='')

    formats = ["zip", "tar", "tar.gz", "tar.bz", "tar.bz2", "tar.lzma"]
	
    for format in formats:
        # A temporary file filled with random content
        file_name = create_temp_file(tmp_dir)
    
        compressed = f"{file_name}.{format}"

        # The file's SHA3-384 checksum
        checksum = open_and_checksum(file_name)

        # Use ouch to compress the file into the given format
        compress(file_name, compressed)
        
        # Remove the original file
        os.remove(file_name)

        # Reconstruct the original file by decompressing the archive
        decompress(compressed)

        # Ensure ouch didn't mess with the file by checking that
        # the checksum matches the one previously calculated
        assert checksum == open_and_checksum(file_name)

        # Remove left-over files
        os.remove(file_name); os.remove(compressed)
    print("... ok")

# Creates a temporary file in the given temporary directory
def create_temp_file(directory: TemporaryDirectory) -> str:
    with NamedTemporaryFile(dir=directory.name, delete=False) as f:
        f.write(os.urandom(1024))

        # The file will not be deleted when the context manager
        # ends since `delete=False` was used
        return f.name

if __name__ == "__main__":
    # Build a temporary directory at the project's folder
    tmp_dir = TemporaryDirectory(dir=os.path.dirname(__file__))

    print(f"Temporary directory created: {tmp_dir.name}")
    os.chdir(tmp_dir.name)

    # Compress files and ensure they match their
    # expected MIME types
    compression_test_suite(tmp_dir)
    # Builds files, compress and decompress them with ouch and then
    # ensure they still have the expected original md5 checksum
    decompression_test_suite(tmp_dir)

    # Checks if decompressing files in parallel works
    # parallel_decompression_test_suite(tmp_dir)

    tmp_dir.cleanup()

    if os.path.exists(tmp_dir.name):
        os.rmdir(tmp_dir.name)

