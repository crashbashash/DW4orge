#!/usr/bin/env python3
"""Experimental DW4 Xbox SaveData.dat inspector and re-signing probe.

Requires a local retail XBE to derive the signing key. This is research tooling,
not yet an Xbox backend for DW4orge. Never writes the input file.
"""
import argparse
import hashlib
import hmac
import json
from pathlib import Path
import struct

SLOT_SIZE = 0x2000
SIGNED_SIZE = 4 + 4 * SLOT_SIZE
RECORD_SIZE = SIGNED_SIZE + 20
SAVE_SIZE = 2 * RECORD_SIZE
CERTIFICATE_KEY = bytes.fromhex("5c0733ae0401f7e8ba7993fdcd2f1fe0")


def u32(data, offset):
    return struct.unpack_from("<I", data, offset)[0]


def signing_key(xbe):
    if len(xbe) < 0x11C or xbe[:4] != b"XBEH":
        raise ValueError("not an XBE")
    cert = u32(xbe, 0x118) - u32(xbe, 0x104)
    if cert < 0 or cert + 0xD0 > len(xbe) or u32(xbe, cert) < 0xD0:
        raise ValueError("truncated XBE certificate")
    if u32(xbe, cert + 8) != 0x42410002:
        raise ValueError("XBE title ID is not Digimon World 4 / World X")
    return hmac.digest(CERTIFICATE_KEY, xbe[cert + 0xC0:cert + 0xD0], "sha1")[:16]


def validate_size(data):
    if len(data) != SAVE_SIZE:
        raise ValueError(f"expected {SAVE_SIZE} bytes, got {len(data)}")
    if any(u32(data, i * RECORD_SIZE) != 4 for i in range(2)):
        raise ValueError("unsupported Xbox save wrapper version")


def verify(data, key):
    validate_size(data)
    return [hmac.compare_digest(
        hmac.digest(key, data[i * RECORD_SIZE:i * RECORD_SIZE + SIGNED_SIZE], "sha1"),
        data[i * RECORD_SIZE + SIGNED_SIZE:(i + 1) * RECORD_SIZE],
    ) for i in range(2)]


def decode_name(data):
    # The marker and encoding are the same as the PS2 little-endian name codec.
    if data[:2] != b"\xff\xff":
        return ""
    return data[2:18].decode("utf-16-le").split("\0", 1)[0]


def inspect(data, key):
    checks = verify(data, key)
    records = []
    for mirror in range(2):
        boxes = []
        for box in range(4):
            start = mirror * RECORD_SIZE + 4 + box * SLOT_SIZE
            slot = data[start:start + SLOT_SIZE]
            boxes.append({
                "box": box + 1, "in_use": u32(slot, 0),
                "unique": f"{u32(slot, 4):08x}",
                "species": slot[8:24].split(b"\0", 1)[0].decode("ascii", errors="replace"),
                "name": decode_name(slot[0x28:0x3A]),
                "level": u32(slot, 0x48), "xdata": u32(slot, 0x5C),
                "bit": u32(slot, 0x60),
            })
        records.append({"mirror": mirror, "signature_valid": checks[mirror], "boxes": boxes})
    return {"size": len(data), "sha256": hashlib.sha256(data).hexdigest(),
            "mirrors_identical": data[:RECORD_SIZE] == data[RECORD_SIZE:], "records": records}


def edit(data, key, box, name=None, bit=None):
    if not all(verify(data, key)):
        raise ValueError("refusing to edit a save with an invalid signature")
    if not 1 <= box <= 4:
        raise ValueError("box must be 1..4")
    if bit is not None and not 0 <= bit <= 9_999_999:
        raise ValueError("BIT must be 0..9999999")
    encoded = None
    if name is not None:
        if not 1 <= len(name) <= 8 or any(not 0x21 <= ord(c) <= 0x7E for c in name):
            raise ValueError("probe names must contain 1..8 printable ASCII characters, without spaces")
        encoded = b"\xff\xff" + "".join(chr(ord(c) + 0xFEE0) for c in name).encode("utf-16-le")
        encoded = encoded.ljust(18, b"\0")
    out = bytearray(data)
    for mirror in range(2):
        base = mirror * RECORD_SIZE
        slot = base + 4 + (box - 1) * SLOT_SIZE
        if u32(data, slot) != 1:
            raise ValueError(f"box {box} is empty in mirror {mirror}")
        if encoded is not None:
            out[slot + 0x28:slot + 0x3A] = encoded
        if bit is not None:
            struct.pack_into("<I", out, slot + 0x60, bit)
        out[base + SIGNED_SIZE:base + RECORD_SIZE] = hmac.digest(key, out[base:base + SIGNED_SIZE], "sha1")
    return bytes(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("save", type=Path)
    ap.add_argument("--xbe", type=Path, required=True)
    ap.add_argument("--box", type=int, default=1)
    ap.add_argument("--set-name")
    ap.add_argument("--set-bit", type=int)
    ap.add_argument("--out", type=Path)
    args = ap.parse_args()
    try:
        key = signing_key(args.xbe.read_bytes())
        data = args.save.read_bytes()
        if args.set_name is not None or args.set_bit is not None:
            if args.out is None:
                raise ValueError("edits require --out pointing to a new file")
            data = edit(data, key, args.box, args.set_name, args.set_bit)
        elif args.out is not None:
            raise ValueError("--out requires an edit")
        report = inspect(data, key)
        if args.out is not None:
            with args.out.open("xb") as f:
                f.write(data)
        print(json.dumps(report, indent=2, ensure_ascii=False))
    except (ValueError, OSError, UnicodeError) as error:
        ap.exit(1, f"error: {error}\n")


if __name__ == "__main__":
    main()
