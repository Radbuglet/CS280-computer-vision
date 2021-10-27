export function assert(cond: boolean, message?: string) {
    if (!cond) {
        throw new Error(message || "Assertion failed.");
    }
}
