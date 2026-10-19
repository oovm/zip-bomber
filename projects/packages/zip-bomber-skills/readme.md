# 🧰 @sxo/zip-bomber-skills

Agent skills for [@sxo/zip-bomber](https://www.npmjs.com/package/@sxo/zip-bomber). Installing them gives an agent the
four variants, the request document shape, the budget ceilings, and the habit of re-verifying an archive before
reporting success — instead of generating a bomb and calling it done.

```bash
npx @sxo/zip-bomber-skills -y                # project-local install
npx @sxo/zip-bomber-skills -g -y             # user-global install
npx @sxo/zip-bomber-skills -a claude -y      # pick the target agent
```

The command is a thin wrapper around `npx skills add <this package> --skill zip-bomber`, and it forwards any extra
flags, so `-g`, `-y`, and `-a <agent>` behave exactly as they do in the `skills` CLI. It always targets the `zip-bomber`
skill unless you pass `--skill` yourself.

The sources it installs live in `skills/zip-bomber/`: `SKILL.md` plus a `reference.md` with the option tables and report
shapes. Read or copy them by hand if you would rather review them before an installer writes into your agent directory.
