import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
export default require("./lib/zip-bomber-darwin-x64.node");
