import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
export default require("./lib/zip-bomber-win32-x64-msvc.node");
