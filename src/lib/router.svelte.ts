// Enrutador mínimo basado en el hash (#/ruta).
import { on } from "./api";

function current() {
  return decodeURIComponent(location.hash.replace(/^#\/?/, ""));
}

class Router {
  path = $state(current());

  constructor() {
    window.addEventListener("hashchange", () => (this.path = current()));
    on("navigate", (route: string) => this.go(route));
  }

  go(path: string) {
    location.hash = "/" + path;
  }

  /** Primer segmento de la ruta. */
  get section() {
    return this.path.split("/")[0] ?? "";
  }

  get sub() {
    return this.path.split("/")[1] ?? "";
  }
}

export const router = new Router();
