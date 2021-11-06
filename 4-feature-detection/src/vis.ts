import {Engine, EngineHandler, fillCircle, PathConstructor, strokeCircle} from "./util/engine";
import {lerp, Vector2} from "./util/math";
import {matchNodes, MatchResult} from "./match";
import {ArrayLike, concatArrays, enumerate, mapArray} from "./util/array";
import {nonNull} from "./util/debug";

function makeMeshAt(origin: Vector2): Handle[] {
    return [
        new Handle(new Vector2(10, 10).add(origin)),
        new Handle(new Vector2(50, 25).add(origin)),
        new Handle(new Vector2(60, 60).add(origin)),
        new Handle(new Vector2(30, 80).add(origin)),
        new Handle(new Vector2(10, 40).add(origin)),
    ];
}

export class VisAppScene implements EngineHandler {
    private readonly template_nodes: Handle[] = makeMeshAt(new Vector2(200, 200));
    private readonly target_nodes: Handle[] = makeMeshAt(new Vector2(500, 200));
    private matching?: Readonly<MatchResult>;

    private handle?: Handle;

    private get handles(): ArrayLike<Handle> {
        return concatArrays(this.template_nodes, this.target_nodes);
    }

    private getFocusedHandle(engine: Engine): Handle | null {
        for (const handle of this.handles) {
            if (handle.position.distanceTo(engine.mouse) < 10) {
                return handle;
            }
        }
        return null;
    }

    onUpdate(engine: Engine) {
        // Handle dragging
        if (engine.mouse_down) {
            if (this.handle === undefined) {
                // Handle drag start
                const handle = this.getFocusedHandle(engine);
                if (handle !== null) {
                    this.handle = handle;
                }
            } else {
                // Handle drag continue
                this.handle.position = engine.mouse;
            }
        } else if (this.handle !== undefined) {
            // Handle drag end
            this.handle = undefined;
        }

        // Calculate mapping
        this.matching = matchNodes(
            mapArray(this.template_nodes, elem => elem.position),
            mapArray(this.target_nodes, elem => elem.position),
        );
    }

    onRender(engine: Engine) {
        engine.draw(ctx => {
            ctx.clearRect(0, 0, engine.size.x, engine.size.y);

            const { error, expectations, correspondence } = nonNull(this.matching);

            // Render handle connectors
            engine.draw(ctx => {
                const path = new PathConstructor();

                ctx.strokeStyle = "red";
                ctx.beginPath();
                for (const handle of this.template_nodes) {
                    path.add(ctx, handle.position);
                }
                ctx.closePath();
                ctx.stroke();

                ctx.strokeStyle = "blue";
                ctx.beginPath();
                path.reset();
                for (const index of correspondence) {
                    path.add(ctx, this.target_nodes[index].position);
                }
                ctx.closePath();
                ctx.stroke();
            });

            // Render handles
            engine.draw(ctx => {
                ctx.textAlign = "center";
                ctx.textBaseline = "middle";
                ctx.font = "12px monospace";

                // Draw nodes
                for (const [index, handle] of enumerate(this.template_nodes)) {
                    ctx.fillStyle = lerpColor(
                        138, 22, 22,
                        255, 133, 133,
                        index / this.template_nodes.length
                    );
                    fillCircle(ctx, handle.position, 10);

                    ctx.fillStyle = "white";
                    ctx.fillText(index.toString(), handle.position.x, handle.position.y);
                }

                for (const [real_index, handle] of enumerate(this.target_nodes)) {
                    const index = correspondence[real_index];
                    const expected = expectations[real_index]

                    ctx.strokeStyle = ctx.fillStyle = lerpColor(
                        28, 46, 117,
                        156, 176, 255,
                        index / this.target_nodes.length
                    );

                    // Draw real circle
                    fillCircle(ctx, handle.position, 10);

                    ctx.fillStyle = "white";
                    ctx.fillText(index.toString(), handle.position.x, handle.position.y);

                    // Draw expected circle
                    strokeCircle(ctx, expected, 10);

                    ctx.fillStyle = "black";
                    ctx.fillText(index.toString(), expected.x, expected.y);
                }
            });

            // Render HUD
            engine.draw(ctx => {
                ctx.textAlign = "left";
                ctx.textBaseline = "top";
                ctx.font = "24px monospace";
                ctx.fillText(`Error: ${error}`, 0, 0);
            });
        });
    }
}

class Handle {
    constructor(public position: Vector2) {}
}

function lerpColor(
    start_r: number, start_g: number, start_b: number,
    end_r: number, end_g: number, end_b: number,
    coef: number
): string {
    return `rgb(${Math.floor(lerp(start_r, end_r, coef))}, ${Math.floor(lerp(start_g, end_g, coef))}, ${Math.floor(lerp(start_b, end_b, coef))})`;
}
