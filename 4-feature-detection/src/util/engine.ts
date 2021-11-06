import {assert, nonNull, tryCall} from "./debug";
import {Tau, Vector2} from "./math";

type EngineState = {
    container: Element,
    canvas: HTMLCanvasElement,
    ctx: CanvasRenderingContext2D,
    request_stop: boolean,
    size: Vector2,
    mouse: Vector2,
    mouse_down: boolean,
    on_mouse_down: (ev: MouseEvent) => void,
    on_mouse_up: (ev: MouseEvent) => void,
    on_mouse_move: (ev: MouseEvent) => void,
};

export class Engine {
    //> Properties
    private state?: EngineState;

    get container(): Element {
        return nonNull(this.state).container;
    }

    get canvas(): HTMLCanvasElement {
        return nonNull(this.state).canvas;
    }

    get ctx(): CanvasRenderingContext2D {
        return nonNull(this.state).ctx;
    }

    get request_stop(): boolean {
        return nonNull(this.state).request_stop;
    }

    set request_stop(value: boolean) {
        nonNull(this.state).request_stop = value;
    }

    get size(): Vector2 {
        return nonNull(this.state).size;
    }

    get mouse(): Vector2 {
        return nonNull(this.state).mouse;
    }

    get mouse_down(): boolean {
        return nonNull(this.state).mouse_down;
    }

    //> Run loop
    constructor(public handler: EngineHandler) {}

    start(container: Element) {
        assert(this.state === undefined, "Cannot start Engine more than once!");

        // Construct canvas
        const canvas = document.createElement("canvas");
        const ctx = nonNull(canvas.getContext("2d"));
        this.state = {
            request_stop: false,
            container,
            canvas,
            ctx,
            size: new Vector2(),  // Initialized in fitInContainer().
            mouse: new Vector2(),
            mouse_down: false,
            on_mouse_down: this.onMouseDown.bind(this),
            on_mouse_up: this.onMouseUp.bind(this),
            on_mouse_move: this.onMouseMove.bind(this),
        };

        // Attach it
        container.append(canvas);
        document.body.addEventListener("mousedown", this.state.on_mouse_down);
        document.body.addEventListener("mouseup", this.state.on_mouse_up);
        document.body.addEventListener("mousemove", this.state.on_mouse_move);

        // Run
        requestAnimationFrame(this.tick.bind(this));
    }

    stop() {
        assert(this.state !== undefined, "Engine must be running to stop it!");
    }

    private onMouseDown(_e: MouseEvent) {
        const state = nonNull(this.state);
        state.mouse_down = true;
    }

    private onMouseUp(_e: MouseEvent) {
        const state = nonNull(this.state);
        state.mouse_down = false;
    }

    private onMouseMove(e: MouseEvent) {
        const state = nonNull(this.state);
        const rect = state.container.getBoundingClientRect();
        state.mouse = new Vector2(e.clientX - rect.left, e.clientY - rect.top);
    }

    private tick() {
        const state = nonNull(this.state);

        // Handle stop requests (when is this ever used?! why did I support this?!)
        if (state.request_stop) {
            document.body.removeEventListener("mousedown", state.on_mouse_down);
            document.body.removeEventListener("mouseup", state.on_mouse_up);
            document.body.removeEventListener("mousemove", state.on_mouse_move);
            this.state = undefined;
            return;
        } else {
            requestAnimationFrame(this.tick.bind(this));
        }

        // Handle resizes, ignore DPI
        const dpi = window.devicePixelRatio;
        {
            const {container, canvas} = state;
            const {width, height} = container.getBoundingClientRect();
            canvas.style.width = `${width}px`;
            canvas.style.height = `${height}px`;
            canvas.width = width * dpi;
            canvas.height = height * dpi;
            state.size = new Vector2(width, height);
        }

        // Run loop
        this.draw(ctx => {
            ctx.scale(dpi, dpi);
            this.handler.onUpdate(this);
            this.handler.onRender(this);
        });
    }

    //> Utilities
    draw<R>(cb: (ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement) => R): R {
        const {canvas, ctx} = this;
        ctx.save();
        return tryCall(
            () => cb(ctx, canvas),
            () => ctx.restore(),
        );
    }
}

export interface EngineHandler {
    onUpdate(engine: Engine): void;
    onRender(engine: Engine): void;
}

export function fillCircle(ctx: CanvasRenderingContext2D, pos: Vector2, radius: number) {
    ctx.beginPath();
    ctx.arc(pos.x, pos.y, radius, 0, Tau);
    ctx.fill();
}

export function strokeCircle(ctx: CanvasRenderingContext2D, pos: Vector2, radius: number) {
    ctx.beginPath();
    ctx.arc(pos.x, pos.y, radius, 0, Tau);
    ctx.stroke();
}

export class PathConstructor {
    private is_first_: boolean = true;

    get is_first(): boolean {
        return this.is_first_;
    }

    reset() {
        this.is_first_ = false;
    }

    add(ctx: CanvasRenderingContext2D, pos: Vector2) {
        if (this.is_first_) {
            ctx.moveTo(pos.x, pos.y);
            this.is_first_ = false;
        } else {
            ctx.lineTo(pos.x, pos.y);
        }
    }
}
