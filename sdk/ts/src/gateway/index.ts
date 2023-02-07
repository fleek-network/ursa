import { CID } from 'multiformats'
import { CarReader } from '@ipld/car'
import fetch from 'cross-fetch'
import { verify_block } from '../utils'
import { Block } from '@ipld/car/reader'

export const DEFAULT_GATEWAY = 'https://gateway.ursa.earth'

/// Gateway provides a simple interface to the Ursa Gateway API.
export class Gateway {
    public url: string = DEFAULT_GATEWAY
    constructor(url?: URL | string) {
        if (url) {
            const urlStr = typeof url === 'string' ? url : url.toString()
            if (urlStr.endsWith('/')) {
                this.url = urlStr.slice(0, -1)
            } else {
                this.url = urlStr
            }
        }
    }

    // Fetch a car file with a given root cid from the gateway
    async get(
        cid: CID,
        verify = true,
        _origins?: [string]
    ): Promise<Block[] | undefined> {
        const res = await fetch(`${this.url}/${cid.toString()}`, {
            method: 'GET',
            headers: {
                Accept: 'application/vnd.ipfs.car'
            }
        })

        const carFile = await res.arrayBuffer()

        // https://github.com/ipld/js-car/#async-carreaderfrombytesbytes
        const reader = await CarReader.fromBytes(new Uint8Array(carFile))
        const roots = (await reader.getRoots()).map((cid) => {
            return cid.toString()
        })

        if (!roots.includes(cid.toString())) {
            console.log(roots)
            throw new Error('Root CID not found in car file')
        }

        if (verify)
            await Promise.all(
                reader._blocks.map((block) =>
                    verify_block(block.cid, block.bytes)
                )
            )

        return reader._blocks
    }

    // todo: put(carFile, origins) once gateway supports it
}
