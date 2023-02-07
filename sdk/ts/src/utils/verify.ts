import { bytes, CID } from 'multiformats'
import * as dagCbor from '@ipld/dag-cbor'
import * as dagPb from '@ipld/dag-pb'
import * as dagJson from '@ipld/dag-json'
import * as raw from 'multiformats/codecs/raw'
import * as json from 'multiformats/codecs/json'
import { sha256 } from 'multiformats/hashes/sha2'
import { from as hasher, Hasher } from 'multiformats/hashes/hasher'
import { blake2b256 } from '@multiformats/blake2/blake2b'

const { toHex } = bytes

const codecs = new Map<number, any>([
    [dagCbor.code, dagCbor],
    [dagPb.code, dagPb],
    [dagJson.code, dagJson],
    [raw.code, raw],
    [json.code, json]
])

const hashes = new Map<number, Hasher<string, number>>([
    [sha256.code, sha256],
    [blake2b256.code, hasher(blake2b256)]
])

export const verify_block = async (cid: CID, bytes: Uint8Array) => {
    const codec = codecs.get(cid.code)
    if (!codec) {
        throw new Error(`Unexpected codec: 0x${cid.code.toString(16)}`)
    }

    const hasher = hashes.get(cid.multihash.code)
    if (!hasher) {
        throw new Error(
            `Unexpected multihash code: 0x${cid.multihash.code.toString(16)}`
        )
    }

    // compare the digest of the bytes to the digest in the CID
    const hash = await hasher.digest(bytes)
    if (toHex(hash.digest) !== toHex(cid.multihash.digest)) {
        throw new Error(
            `Digest mismatch: ${toHex(hash.digest)} != ${toHex(
                cid.multihash.digest
            )}`
        )
    }

    // optional: round-trip the object and get the same CID for the re-encoded bytes, see
    //           https://github.com/ipld/js-car/blob/master/examples/verify-car.js#L63-L72
}
