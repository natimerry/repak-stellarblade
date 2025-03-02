from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
from cryptography.hazmat.backends import default_backend
import base64
import binascii
import os

class AesKey:
    def __init__(self, key_bytes):
        self.key = key_bytes

    @classmethod
    def from_str(cls, s):
        try:
            if s.startswith("0x"):
                key_bytes = binascii.unhexlify(s[2:])
            else:
                s = s.rstrip('=')
                key_bytes = base64.b64decode(s)

            reversed_bytes = bytearray(key_bytes)
            for i in range(0, len(reversed_bytes), 4):
                if i + 4 <= len(reversed_bytes):
                    reversed_bytes[i:i + 4] = reversed_bytes[i:i + 4][::-1]

            return cls(bytes(reversed_bytes))

        except (binascii.Error, base64.binascii.Error, ValueError):
            return None

def decrypt_chunks(data, key):
    if len(data) % 16 != 0:
        raise ValueError("Data length must be a multiple of 16 bytes.")

    decrypted_data = bytearray()
    for i in range(0, len(data), 16):
        chunk = bytearray(data[i:i + 16])

        for j in range(0, 16, 4):
            chunk[j:j + 4] = chunk[j:j + 4][::-1]

        cipher = Cipher(algorithms.AES(key), modes.ECB(), backend=default_backend())
        decryptor = cipher.decryptor()
        decrypted_block = decryptor.update(bytes(chunk)) + decryptor.finalize()

        decrypted_chunk = bytearray(decrypted_block)

        for j in range(0, 16, 4):
            decrypted_chunk[j:j + 4] = decrypted_chunk[j:j + 4][::-1]

        decrypted_data.extend(decrypted_chunk)

    return bytes(decrypted_data)

def decrypt_data(encrypted_data, aes_key):
    return decrypt_chunks(encrypted_data, aes_key.key)

try:
    with open("encrypted_index.bin", 'rb') as f:
        encrypted_data = f.read()
        encrypted_size = len(encrypted_data)

        key_hex = "0x0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74"
        aes_key = AesKey.from_str(key_hex)

        if aes_key is None:
            raise ValueError("Invalid key format.")

        unencrypted_bytes = decrypt_data(encrypted_data, aes_key)

        with open("decrypted_index.bin", "wb") as out_file:
            out_file.write(unencrypted_bytes[:encrypted_size]) #truncate to the size of the encrypted file.

        print(f"Decryption complete. Wrote {encrypted_size} bytes to decrypted_index.bin")

except FileNotFoundError:
    print("Error: encrypted_index.bin not found.")
except ValueError as e:
    print(f"Error: {e}")
except Exception as e:
    print(f"An unexpected error occurred: {e}")