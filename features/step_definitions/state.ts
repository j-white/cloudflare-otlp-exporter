import {MiniflareDriver} from "./mf";
import {OpenTelemetryServer} from "./otel_server";
import {CloudflareMockServer} from "./cf_mock_server";

const mf = new MiniflareDriver();
const otelServer = new OpenTelemetryServer();
const cloudflareMockServer = new CloudflareMockServer();

type MfConfig = {
    metricsUrl: string|undefined;
    cloudflareApiUrl: string|undefined
};

const mfConfig: MfConfig = {
    metricsUrl: undefined,
    cloudflareApiUrl: undefined,
}

export { mf, mfConfig, otelServer, cloudflareMockServer };
