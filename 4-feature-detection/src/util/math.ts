export class Vector2 {
    // noinspection JSSuspiciousNameCombination
    constructor(public readonly x: number = 0, public readonly y: number = x) {}

    static polarUnit(rad: number): Vector2 {
        return new Vector2(Math.cos(rad), Math.sin(rad));
    }

    static polar(rad: number, mag: number): Vector2 {
        return Vector2.polarUnit(rad).scale(mag);
    }

    zip(rhs: Vector2, fn: (lhs: number, rhs: number) => number): Vector2 {
        return new Vector2(
            fn(this.x, rhs.x),
            fn(this.y, rhs.y),
        )
    }

    map(fn: (comp: number) => number): Vector2 {
        return new Vector2(
            fn(this.x),
            fn(this.y),
        )
    }

    add(rhs: Vector2): Vector2 {
        return this.zip(rhs, (a, b) => a + b);
    }

    sub(rhs: Vector2): Vector2 {
        return this.zip(rhs, (a, b) => a - b);
    }

    mul(rhs: Vector2): Vector2 {
        return this.zip(rhs, (a, b) => a * b);
    }

    div(rhs: Vector2): Vector2 {
        return this.zip(rhs, (a, b) => a / b);
    }

    cross(rhs: Vector2): Vector2 {
        const [a, b] = [this, rhs];
        return new Vector2(
            a.x * b.x - a.y * b.y,
            a.x * b.y + a.y * b.x,
        );
    }

    crossAround(origin: Vector2, rhs: Vector2): Vector2 {
        return this.sub(origin).cross(rhs).add(origin);
    }

    rotated(angle: number): Vector2 {
        return this.cross(Vector2.polarUnit(angle));
    }

    rotatedAround(origin: Vector2, angle: number) {
        return this.crossAround(origin, Vector2.polarUnit(angle));
    }

    normalized(): Vector2 {
        return this.scale(1 / this.len());  // Now with 100% less 0x5F3759DF!
    }

    scale(scalar: number): Vector2 {
        return this.map(comp => comp * scalar);
    }

    neg(): Vector2 {
        return this.scale(-1);
    }

    negArgument(): Vector2 {
        return this.y < 0
            ? new Vector2(-this.x,  this.y)
            : new Vector2( this.x, -this.y);
    }

    lenSquared(): number {
        return this.x ** 2 + this.y ** 2;
    }

    len(): number {
        return Math.sqrt(this.lenSquared());
    }

    angle(): number {
        const angle = Math.atan(this.y / this.x);
        return this.y > 0 ?
            angle :
            angle + Math.PI;
    }

    distanceToSquared(other: Vector2): number {
        return other.sub(this).lenSquared();
    }

    distanceTo(other: Vector2): number {
        return other.sub(this).len();
    }
}

export const Tau = 2 * Math.PI;

export function inRange(min: number, val: number, max: number): boolean {
    return min <= val && val <= max;
}

export function signedMod(a: number, n: number): number {
    return a - Math.floor(a/n) * n;
}

export function wrapAngleRad(a: number): number {
    return signedMod(a, Tau);
}

export function angleDiff(a: number, b: number): number {
    return Math.min(wrapAngleRad(a - b), wrapAngleRad(b - a));
}

export function lerp(a: number, b: number, coef: number): number {
    return a + (b - a) * coef;
}
