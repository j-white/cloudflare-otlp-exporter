# Cloudflare OpenTelemetry exporter

## Description

This worker was written to push Cloudflare Analytics data to an OpenTelemetry collector.

It is inspired by the [cloudflare-exporter](https://github.com/lablabs/cloudflare-exporter), which is unfortunately no longer maintained.
By running it as a worker and pushing metrics, we avoid the need to deploy a dedicated container and allow the worker to be run on [green compute](https://blog.cloudflare.com/announcing-green-compute).

## Metrics currently support

- [x] Workers
- [x] D1
- [ ] Queues
- [ ] Durable Objects
- [ ] Zones

## Usage

* Clone the repo
* Modify the wrangler.toml file to include your Cloudflare account ID and API token and OTel collector endpoint
* Run `npx wrangler deploy --env dev` to deploy the worker

## How it works

* Scrape the Cloudflare Analytics API via GraphQL
* Build the metrics in a Prometheus registry  (ideally this would be directly with OTel, but ran into some challenges with threading and WASM for these SDKs)
* Convert the Prometheus registry to OTel metrics
* Push the OTel metrics to an OTel collector via protobuf (JSON encoding for OTel Metrics is broken in the Rust SDKs and is only used for testing)

## Next steps

* Add more metrics
* Help improve OTel Rust SDKs and remove dependency on Prometheus
