#!/usr/bin/env python3
import sys
import struct
import base64
import argparse

# Dependency Audit
try:
    import nacl.signing
    import nacl.exceptions
    HAS_NACL = True
except ImportError:
    HAS_NACL = False

# BINARY PROTOCOL SPECIFICATION (276 bytes total):
# 00-03: [u8; 4] (Sync: 0xAA, 0xBB, 0xCC, 0xDD)
# 04-11: u64 (Tick)
# 12-43: [u8; 32] (WorldHash)
# 44-47: u32 (Population)
# 48-79: [u64; 4] (TechnologyMask)
# 80-83: u32 (CivIndex)
# 84-211: [u64; 16] (BitGrid)
# 212-275: [u8; 64] (Ed25519 Signature)

FRAME_SIZE = 276
SIG_OFFSET = 212
DATA_TO_SIGN_START = 4
DATA_TO_SIGN_END = 212

def main():
    parser = argparse.ArgumentParser(description="Laplace Oracle Telemetry Observer (Stage 8)")
    parser.add_argument("public_key", help="Public Key in Base64")
    args = parser.parse_args()

    pk_bytes = base64.b64decode(args.public_key)

    print(f"--- Laplace Oracle Observer (Stage 8) ---")
    print(f"{'Tick':>8} | {'Pop':>10} | {'TechBits':>8} | {'Civ':>8} | {'Status'}")
    print("-" * 65)

    try:
        while True:
            # Sync Scanner
            while True:
                byte = sys.stdin.buffer.read(1)
                if not byte: return
                if byte == b'\xaa':
                    header = sys.stdin.buffer.read(3)
                    if header == b'\xbb\xcc\xdd':
                        break
            
            # Read remaining payload (272 bytes)
            payload = sys.stdin.buffer.read(272)
            if len(payload) < 272: break
            
            raw_frame = b'\xaa\xbb\xcc\xdd' + payload
            
            # Unpack
            tick = struct.unpack("<Q", buf_slice(raw_frame, 4, 12))[0]
            pop = struct.unpack("<I", buf_slice(raw_frame, 44, 48))[0]
            tech_mask = struct.unpack("<4Q", buf_slice(raw_frame, 48, 80))
            civ = struct.unpack("<I", buf_slice(raw_frame, 80, 84))[0]
            signature_bytes = raw_frame[212:276]
            
            data_to_verify = raw_frame[4:212]
            
            status = "UNTRIED"
            if HAS_NACL:
                try:
                    verify_key = nacl.signing.VerifyKey(pk_bytes)
                    verify_key.verify(data_to_verify, signature_bytes)
                    status = "VERIFIED"
                except nacl.exceptions.BadSignatureError:
                    status = "INVALID"
            else:
                status = "UNVERIFIED (No nacl)"

            tech_bits = sum(bin(m).count('1') for m in tech_mask)

            print(f"{tick:8} | {pop:10} | {tech_bits:8} | {civ:8} | {status}")
            sys.stdout.flush()

    except KeyboardInterrupt:
        print("\nObservation terminated.")

def buf_slice(buf, start, end):
    return buf[start:end]

if __name__ == "__main__":
    main()
