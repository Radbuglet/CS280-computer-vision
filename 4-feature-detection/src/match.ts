import {Vector2} from "./util/math";
import {ArrayLike, enumerate, indexPermutations, mapArray, virtualizeArray} from "./util/array";
import {assert} from "./util/debug";

export type MatchResult = CompareResult & {
    correspondence: number[],
};

export type CompareResult = {
    error: number,
    expectations: ArrayLike<Vector2>,
};

export function matchNodes(
    template: ArrayLike<Vector2>,
    target: ArrayLike<Vector2>,
): MatchResult {
    // Validate lengths
    assert(template.length === target.length, "Template and target meshes must have the same length");
    const len = template.length;
    assert(len >= 2, "Length must be at least 2.");

    // Permute the chain to find the minimum
    let best: Partial<MatchResult> = { error: Infinity };
    for (const permutation of indexPermutations(len)) {
        const match = compareChains(template, virtualizeArray(target, permutation));
        if (match.error < best.error!) {
            best = match;
            best.correspondence = permutation;
        }
    }

    return best as MatchResult;  // The other fields were filled out in the process.
}

export function compareChains(template: ArrayLike<Vector2>, target: ArrayLike<Vector2>): CompareResult {
    // Handle special cases
    assert(template.length === target.length);
    const len = template.length;
    assert(len >= 2);


    // Compare chains
    const expectations = getExpectations(template, target[0], target[1]);
    let error = 0;
    for (const [i, expected] of enumerate(expectations)) {
        error += target[i].distanceTo(expected);
    }

    return { error, expectations };
}

export function getExpectations(template: ArrayLike<Vector2>, first: Vector2, second: Vector2): ArrayLike<Vector2> {
    // Handle special cases
    assert(template.length >= 2);

    // Calculate a magical value that I can only describe using a pen and paper.
    // Here's a Desmos project if that helps: https://www.desmos.com/calculator/kjafe1okbq
    // I'm sure there's a really easy way to do this with Linear Algebra but I haven't gotten there yet.
    const b_template = template[1].sub(template[0]);
    const b_target = second.sub(first);
    const template_to_target_rot = b_target.cross(b_template.negArgument()).normalized();
    const template_to_target_cross = template_to_target_rot.scale(b_target.len() / b_template.len());

    // Construct expectation graph
    return mapArray(template, node => node.sub(template[0]).cross(template_to_target_cross).add(first));
}
