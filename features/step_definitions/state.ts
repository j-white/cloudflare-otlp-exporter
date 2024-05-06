import {MiniflareDriver} from "./mf";
import {OpenTelemetryServer} from "./otel_server";
import {CloudflareMockServer} from "./cf_mock_server";

const mf = new MiniflareDriver();
const otelServer = new OpenTelemetryServer();
const cloudflareMockServer = new CloudflareMockServer();

export { mf, otelServer, cloudflareMockServer };
