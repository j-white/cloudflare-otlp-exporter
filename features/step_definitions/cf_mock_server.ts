import http from "http";
import {AddressInfo} from "net";
import fs from "fs";

export class CloudflareMockServer {
    server: http.Server | undefined;

    start() {
        let self = this;
        const workerQuery = fs.readFileSync('./features/data/worker_query_response.json').toString();
        const d1Query = fs.readFileSync('./features/data/d1_query_response.json').toString();
        const durableObjectsQuery = fs.readFileSync('./features/data/durableobjects_query_response.json').toString();
        const queueBacklogQuery = fs.readFileSync('./features/data/queue_backlog_query_response.json').toString();
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
                res.setHeader('Content-Type', 'application/json');
                if (body.indexOf('d1AnalyticsAdaptiveGroups') > -1) {
                    res.end(d1Query);
                } else if (body.indexOf('durableObjectsInvocationsAdaptiveGroups') > -1) {
                    res.end(durableObjectsQuery);
                } else if (body.indexOf('queueBacklogAdaptiveGroups') > -1) {
                    res.end(queueBacklogQuery);
                } else {
                    res.end(workerQuery);
                }
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
