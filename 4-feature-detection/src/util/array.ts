// So much for zero-cost abstractions. Let's just hope the JIT is smart enough to optimize this garbage.

import {assert} from "./debug";

// === Generic array algorithms === //

export function *range(stop: number, start: number = 0): IterableIterator<any> {
    for (let i = start; i < stop; i++) {
        yield i;
    }
}

export function shallowCloneArray<T>(array: readonly T[]): T[] {
    return Array.from(array);
}

export function swap<T>(target: T[], a: number, b: number) {
    const tmp = target[a];
    target[a] = target[b];
    target[b] = tmp;
}

export function *enumerate<T>(target: Iterable<T>): IterableIterator<readonly [number, T]> {
    let i = 0;
    for (const elem of target) {
        yield [i++, elem];
    }
}

// === Permutations === //

/// Iterates through all permutations of an array using [Heap's algorithm](heaps_algo).
///
/// [heaps_algo]: https://en.wikipedia.org/wiki/Heap%27s_algorithm
export function *arrayPermutations<T>(original: readonly T[]): IterableIterator<T[]> {
    const c = new Array(original.length).fill(0);

    yield shallowCloneArray(original);

    const temporary = shallowCloneArray(original);
    for (let i = 0; i < original.length; i++) {
        if (c[i] < i) {
            // N.B. We swap the if clauses from the pseudo-code, because pseudo-code is one indexed so an even index of
            // 0 corresponds to an odd index of 1.
            if (i % 2 === 0) {
                swap(temporary, c[i], i);
            } else {
                swap(temporary, 0, i);
            }

            yield shallowCloneArray(temporary);
            c[i]++;
            i = 0;
        } else {
            c[i] = 0;
            i++;
        }
    }
}

export function indexPermutations(n: number): IterableIterator<number[]> {
    return arrayPermutations(Array.from(range(n)));
}

// === Virtual arrays === //

export interface Indexable<T> {
    readonly [index: number]: T;
}

export interface FnIndexable<T> {
    getIndex(key: number): T | undefined;
}

export function overloadIndexing<V, T extends FnIndexable<V>>(instance: T): T & Indexable<V> {
    return new Proxy(instance, {
        get(target, p): any {
            // noinspection SuspiciousTypeOfGuard
            if (typeof p === "string" && !isNaN(parseInt(p))) {
                return target.getIndex(parseInt(p));
            } else {
                return (target as any)[p];
            }
        }
    }) as any;
}

export interface ArrayLikeBase<T> {
    readonly length: number;
    [Symbol.iterator](): Iterator<T>;
}

export interface ArrayLikeFn<T> extends FnIndexable<T>, ArrayLikeBase<T> {}

export interface ArrayLike<T> extends Indexable<T>, ArrayLikeBase<T> {}

class VirtualArray<T> implements ArrayLikeFn<T> {
    constructor(
        private readonly target: ArrayLike<T>,
        private readonly indirections: ArrayLike<number>,
    ) {}

    getIndex(key: number): T | undefined {
        return this.target[this.indirections[key]];
    }

    get length(): number {
        return this.target.length;
    }

    *[Symbol.iterator](): Iterator<T> {
        for (const index of this.indirections) {
            yield this.target[index];
        }
    }
}

export function virtualizeArray<T>(target: ArrayLike<T>, indirections: ArrayLike<number>): ArrayLike<T> {
    assert(target.length === indirections.length);
    return overloadIndexing(new VirtualArray(target, indirections));
}

class MapArray<T, V> implements ArrayLikeFn<V> {
    constructor(
        private readonly target: ArrayLike<T>,
        private readonly map: (elem: T) => V,
    ) {}

    get length(): number {
        return this.target.length;
    }

    *[Symbol.iterator](): Iterator<V> {
        for (const elem of this.target) {
            yield this.map(elem);
        }
    }

    getIndex(key: number): V | undefined {
        const elem = this.target[key];
        return elem !== undefined ? this.map(elem) : undefined;
    }
}

export function mapArray<U, V>(target: ArrayLike<U>, map: (elem: U) => V): ArrayLike<V> {
    return overloadIndexing(new MapArray(target, map));
}

class ConcatArrays<T> implements ArrayLikeFn<T> {
    constructor(
        private readonly left: ArrayLike<T>,
        private readonly right: ArrayLike<T>,
    ) {}

    get length(): number {
        return this.left.length + this.right.length;
    }

    *[Symbol.iterator](): Iterator<T> {
        for (const elem of this.left) {
            yield elem;
        }

        for (const elem of this.right) {
            yield elem;
        }
    }

    getIndex(index: number): T | undefined {
        if (index < this.left.length) {
            return this.left[index];
        }
        index -= this.left.length;

        if (index < this.right.length) {
            return this.right[index];
        }

        return undefined;
    }
}

export function concatArrays<T>(left: ArrayLike<T>, right: ArrayLike<T>): ArrayLike<T> {
    return overloadIndexing(new ConcatArrays(left, right));
}
