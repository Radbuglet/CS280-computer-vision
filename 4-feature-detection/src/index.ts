import {Engine} from "./util/engine";
import {VisAppScene} from "./vis";
import {nonNull} from "./util/debug";

const engine = new Engine(new VisAppScene());
engine.start(nonNull(document.querySelector("#container")));
