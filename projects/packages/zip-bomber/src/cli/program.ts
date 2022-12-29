import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
    assertCatalogCoverage,
    createCli,
    loadCatalog,
    type Cli,
} from "@vmz/commander";

import { registerDoctorCommand } from "./commands/doctor-cmd.js";
import { registerGenerateCommand } from "./commands/generate-cmd.js";
import { registerVariantsCommand } from "./commands/variants-cmd.js";

/** Locales root shipped with the main package. */
export function localesRoot(): string {
    return join(dirname(fileURLToPath(import.meta.url)), "../../locales");
}

/** Build the `zb` command tree (Living `09`). */
export function buildZbCli(): Cli {
    const root = localesRoot();
    const cli = createCli("zb")
        .locales(root, { envKeys: ["ZB_LOCALE", "LC_ALL", "LANG"] })
        .intro("zb.cli.intro")
        .option("--json", "zb.cli.opt.json")
        .option("--locale <id>", "zb.cli.opt.locale");

    registerDoctorCommand(cli);
    registerVariantsCommand(cli);
    registerGenerateCommand(cli);

    assertCatalogCoverage(cli, loadCatalog("en-US", root));
    assertCatalogCoverage(cli, loadCatalog("zh-CN", root));
    return cli;
}
