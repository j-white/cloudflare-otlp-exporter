import {After, Given, When, Then} from '@cucumber/cucumber';
import {cloudflareMockServer, mf, otelServer} from "./state";
import {expect} from "chai";

Given('Worker is configured to point to mock Cloudflare API', function () {
    cloudflareMockServer.start();
    mf.config.cloudflareApiUrl = cloudflareMockServer.url();
});

Given('Worker is configured to send metrics to a mock OpenTelemetry collector', function () {
    otelServer.start();
    mf.config.metricsUrl = otelServer.metricsUrl();
});

When('Worker is triggered', async function () {
    await mf.trigger();
});

Then('Worker metrics are published', async function () {
    await new Promise(r => setTimeout(r, 5000));
    let metrics = otelServer.getMetrics();
    expect(metrics).to.have.length.gte(1);
});

Then('Meter name should include {string}', function (metricName: string) {
    let metricNames = otelServer.getMetricNames();
    expect(metricNames).to.contain(metricName);
});

After(async function () {
    await mf.dispose();
    await cloudflareMockServer.dispose();
    await otelServer.dispose();
})
