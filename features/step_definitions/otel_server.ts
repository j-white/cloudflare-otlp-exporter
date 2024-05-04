import http from 'http';
import {IExportMetricsServiceRequest} from "@opentelemetry/otlp-transformer";
import {AddressInfo} from "net";

export class OpenTelemetryServer {
    server: http.Server | undefined;
    metrics: IExportMetricsServiceRequest[] = [];
    metricNames: Map<string, number> = new Map<string, number>();

    private reset() {
        this.metrics = [];
        this.indexMetrics();
    }

    start() {
        let self = this;
        self.reset();
        this.server = http.createServer((req, res) => {
            var body = "";
            req.on('readable', function() {
                let part = req.read();
                if (part !== undefined && part !== null) {
                    body += part;
                }
            });
            req.on('end', function() {
                const metrics = JSON.parse(body) as IExportMetricsServiceRequest;
                self.metrics.push(metrics);
                self.indexMetrics();
                res.statusCode = 200;
                res.setHeader('Content-Type', 'text/plain');
                res.end('OK');
            });
        });
        this.server.listen(() => {
            console.log('opened server on', self.server?.address());
        });
    }

    indexMetrics() {
        let self = this;
        this.metricNames.clear();
        for (let metrics of this.metrics) {
            for (let resourceMetrics of metrics.resourceMetrics) {
                for (let scopeMetrics of resourceMetrics.scopeMetrics) {
                    for (let metric of scopeMetrics.metrics) {
                        self.metricNames.set(metric.name, 1);
                    }
                }
            }
        }
    }

    metricsUrl() {
        const { port } = this.server?.address() as AddressInfo;
        return `http://localhost:${port}/v1/metrics`;
    }

    async dispose() {
        if (this.server != undefined) {
            this.server.close();
            this.server = undefined;
        }
    }

    getMetrics() {
        return this.metrics;
    }

    getMetricNames() {
        return this.metricNames.keys();
    }
}
