export function assert(cond: boolean, message: string = "Assertion failed.") {
    if (!cond) {
        throw new Error(message);
    }
}

export function nonNull<T>(obj: T | null | undefined, message: string = "value was 'null'."): T {
    assert(obj != null, message);
    return obj!;
}

export function tryCall<R>(cb: () => R, done: () => void): R {
    try {
        const ret = cb();
        done();
        return ret;
    } catch (e) {
        done();
        throw e;
    }
}
