export class Utils {
    static waitUntil = (condition:any) => {
        return new Promise((resolve:any, reject) => {
            const interval = setInterval(() => {
                if (!condition()) {
                    return;
                }

                clearInterval(interval);
                resolve();
            }, 100);

            setTimeout(() => {
                clearInterval(interval);
                reject('condition was not resolved in time');
            }, 4500);
        });
    };

}
