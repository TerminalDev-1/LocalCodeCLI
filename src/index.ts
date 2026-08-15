#!/usr/bin/env node
import { run } from "./cli.js";

run(process.argv).catch((e) => {
  console.error(e instanceof Error ? e.message : e);
  process.exit(1);
});
