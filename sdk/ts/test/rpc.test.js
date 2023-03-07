const { MockServer } = require('jest-mock-server')
const { Gateway, CID } = require('../dist/ursa-sdk.umd')
const { readFileSync } = require('fs')

describe('Gateway', () => {
    const server = new MockServer()

    const gatewayGet = (ctx) => {
        ctx.body = readFileSync('../../test_files/test.car')
        ctx.status = 200
    }

    beforeAll(() => server.start())
    afterAll(() => server.stop())
    beforeEach(() => server.reset())

    it('Simple GET request from gateway', async () => {
        // setup mock

        let cid = CID.parse(
            'bafkreihwcrnsi2tqozwq22k4vl7flutu43jlxgb3tenewysm2xvfuej5i4'
        )

        const route = server.get('/:file').mockImplementation(gatewayGet)

        const url = server.getURL()

        let gateway = new Gateway(url)
        let res = await gateway.get(cid)

        expect(res?.length).toBe(1)
        expect(route).toHaveBeenCalledTimes(1)
    })
})
