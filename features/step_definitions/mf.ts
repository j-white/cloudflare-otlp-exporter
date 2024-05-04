import {Log, LogLevel, Miniflare} from "miniflare";
import { MockAgent } from "undici";

export class MiniflareDriver {
    mockAgent = new MockAgent();
    mf: Miniflare | undefined;

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

        let metricsUrl = "";
        let cloudflareApiUrl = "";
        if (options !== undefined) {
            if (options.metricsUrl !== undefined) {
                metricsUrl = options.metricsUrl;
            }
            if (options.cloudflareApiUrl !== undefined) {
                cloudflareApiUrl = options.cloudflareApiUrl;
            }
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
        await this.mf?.dispatchFetch("http://localhost:8787/cdn-cgi/mf/scheduled");
        this.dispose();
    }
}
