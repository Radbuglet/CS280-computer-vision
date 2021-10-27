import {Vector2} from "./util/math";
import {ArrayLike, indexPermutations, virtualizeArray} from "./util/array";
import {assert} from "./util/debug";

export function matchNodes(
    template: readonly Vector2[],
    target: readonly Vector2[],
): readonly [number, number[]] {
    // Validate lengths
    assert(template.length === target.length, "Template and target meshes must have the same length");
    const len = template.length;
    assert(len > 0, "Length must be non-zero");

    // Permute the chain to find the minimum
    let minimum: readonly [number, number[] | null] = [Infinity, null];
    for (const permutation of indexPermutations(template.length)) {
        const error = compareChains(template, virtualizeArray(target, permutation));
        console.log(permutation, error);

        if (error < minimum[0])
            minimum = [error, permutation];
    }

    return minimum as readonly [number, number[]];
}

export function compareChains(template: ArrayLike<Vector2>, target: ArrayLike<Vector2>): number {
    // Handle special cases
    assert(template.length === target.length);
    const len = template.length;
    if (len <= 2) {
        return 0;
    }

    // Compute standard scale
    const base_scale_template = template[0].distanceTo(template[1]);
    const base_scale_target = target[0].distanceTo(target[1]);

    // Sum link errors
    let error = 0;
    for (let i = 2; i < len; i++) {
        // FIXME
    }

    return error;
}
