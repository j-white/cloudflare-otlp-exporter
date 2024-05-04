import http from "http";
import {AddressInfo} from "net";

export class CloudflareMockServer {
    server: http.Server | undefined;

    start() {
        let self = this;
        this.server = http.createServer((req, res) => {
            var body = "";
            req.on('readable', function() {
                let part = req.read();
                if (part !== undefined && part !== null) {
                    body += part;
                }
            });
            req.on('end', function() {
                res.statusCode = 200;
                res.setHeader('Content-Type', 'text/plain');
                res.end('OK');
            });
        });
        this.server.listen(() => {
            console.log('opened server on', self.server?.address());
        });
    }

    url() {
        const { port } = this.server?.address() as AddressInfo;
        return `http://localhost:${port}/`;
    }

    async dispose() {
        if (this.server != undefined) {
            this.server.close();
            this.server = undefined;
        }
    }
}
