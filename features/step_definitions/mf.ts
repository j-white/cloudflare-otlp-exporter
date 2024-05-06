import {Log, LogLevel, Miniflare} from "miniflare";
import { MockAgent } from "undici";

type MfConfig = {
    metricsUrl: string|undefined;
    cloudflareApiUrl: string|undefined
};

export class MiniflareDriver {
    mockAgent = new MockAgent();
    mf: Miniflare | undefined;
    config: MfConfig = {
        metricsUrl: undefined,
        cloudflareApiUrl: undefined,
    }

    start(options?: {metricsUrl?: string, cloudflareApiUrl?: string}): Miniflare {
        this.mockAgent
            .get("https://cloudflare.com")
            .intercept({ path: "/" })
            .reply(200, "cloudflare!");

        this.mockAgent
            .get("https://jsonplaceholder.typicode.com")
            .intercept({ path: "/todos/1" })
            .reply(
                200,
                {
                    userId: 1,
                    id: 1,
                    title: "delectus aut autem",
                    completed: false,
                },
                {
                    headers: {
                        "content-type": "application/json",
                    },
                }
            );

        let self = this;
        if(self.config.metricsUrl === undefined) {
            throw new Error("metricsUrl is not defined!");
        }
        if(self.config.cloudflareApiUrl === undefined) {
            throw new Error("cloudflareApiUrl is not defined!");
        }

        this.mf = new Miniflare({
            log: new Log(LogLevel.DEBUG), // Enable debug messages
            cachePersist: false,
            d1Persist: false,
            kvPersist: false,
            r2Persist: false,
            workers: [{
                scriptPath: "./build/worker/shim.mjs",
                compatibilityDate: "2022-04-05",
                cache: true,
                modules: true,
                bindings: {
                    METRICS_URL: self.config.metricsUrl,
                    CLOUDFLARE_API_URL: self.config.cloudflareApiUrl,
                    CLOUDFLARE_API_KEY: "fake-key",
                },
                modulesRules: [
                    { type: "CompiledWasm", include: ["**/*.wasm"], fallthrough: true },
                ],
                fetchMock: this.mockAgent,
            }]});
        return this.mf;
    }

    dispose() {
        if (this.mf === undefined) {
            return;
        }
        let promise = this.mf.dispose();
        this.mf = undefined;
        return promise;
    }

    async trigger() {
        this.start({});
        const res = await this.mf?.dispatchFetch("http://localhost:8787/");

//        await this.mf?.dispatchFetch("http://fake.host/cdn-cgi/mf/scheduled");
        console.log("Triggered worker");
        this.dispose();
    }
}
