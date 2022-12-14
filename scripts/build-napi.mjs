import { execSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const napiDir = join(root, "projects/crates/zb-napi");

/** @type {Record<string, { packageDir: string, fileName: string }>} */
const ARTIFACTS = {
    "index.win32-x64-msvc.node": {
        packageDir: "zip-bomber-win32-x64",
        fileName: "zip-bomber-win32-x64-msvc.node",
    },
    "zip-bomber.win32-x64-msvc.node": {
        packageDir: "zip-bomber-win32-x64",
        fileName: "zip-bomber-win32-x64-msvc.node",
    },
    "index.linux-x64-gnu.node": {
        packageDir: "zip-bomber-linux-x64",
        fileName: "zip-bomber-linux-x64-gnu.node",
    },
    "zip-bomber.linux-x64-gnu.node": {
        packageDir: "zip-bomber-linux-x64",
        fileName: "zip-bomber-linux-x64-gnu.node",
    },
    "index.darwin-x64.node": {
        packageDir: "zip-bomber-darwin-x64",
        fileName: "zip-bomber-darwin-x64.node",
    },
    "zip-bomber.darwin-x64.node": {
        packageDir: "zip-bomber-darwin-x64",
        fileName: "zip-bomber-darwin-x64.node",
    },
    "index.darwin-arm64.node": {
        packageDir: "zip-bomber-darwin-arm64",
        fileName: "zip-bomber-darwin-arm64.node",
    },
    "zip-bomber.darwin-arm64.node": {
        packageDir: "zip-bomber-darwin-arm64",
        fileName: "zip-bomber-darwin-arm64.node",
    },
    "index.linux-arm64-gnu.node": {
        packageDir: "zip-bomber-linux-arm64",
        fileName: "zip-bomber-linux-arm64-gnu.node",
    },
    "zip-bomber.linux-arm64-gnu.node": {
        packageDir: "zip-bomber-linux-arm64",
        fileName: "zip-bomber-linux-arm64-gnu.node",
    },
};

function findBuiltNodes(dir) {
    return readdirSync(dir).filter((name) => name.endsWith(".node"));
}

execSync("pnpm exec napi build --platform --release", { cwd: napiDir, stdio: "inherit", shell: true });

for (const generated of ["index.js", "index.d.ts", "zip-bomber.js", "zip-bomber.d.ts"]) {
    const path = join(napiDir, generated);
    if (existsSync(path)) {
        unlinkSync(path);
    }
}

const built = findBuiltNodes(napiDir);
if (built.length === 0) {
    throw new Error(`No .node artifacts under ${napiDir}`);
}

let copied = 0;
for (const name of built) {
    const mapping = ARTIFACTS[name];
    if (!mapping) {
        console.warn(`skip unmapped artifact: ${name}`);
        continue;
    }
    const libDir = join(root, "projects/packages", mapping.packageDir, "lib");
    mkdirSync(libDir, { recursive: true });
    for (const existing of readdirSync(libDir).filter((entry) => entry.endsWith(".node"))) {
        unlinkSync(join(libDir, existing));
    }
    const dest = join(libDir, mapping.fileName);
    copyFileSync(join(napiDir, name), dest);
    console.log(`copied ${name} -> ${dest}`);
    copied += 1;
}

if (copied === 0) {
    throw new Error(`Built nodes (${built.join(", ")}) did not match any platform package mapping`);
}
