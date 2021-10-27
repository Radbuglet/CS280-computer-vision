import {Vector2} from "./util/math";
import {matchNodes} from "./match";

// a --- b
// |     |
// c --- d
const template = [
    new Vector2(0, 0),
    new Vector2(5, 0),
    new Vector2(0, 5),
    new Vector2(5, 5),
];

// c --- a
// |     |
// b --- d
const target_origin = new Vector2(3, 4);
const rot_vec = Vector2.polarUnit(Math.PI / 2);

const target = [
    target_origin.add(new Vector2(0, 2).cross(rot_vec)),
    target_origin.add(new Vector2(2, 0).cross(rot_vec)),
    target_origin.add(new Vector2(0, 0).cross(rot_vec)),
    target_origin.add(new Vector2(2, 2).cross(rot_vec)),
];

console.log("Error:", matchNodes(template, target));
