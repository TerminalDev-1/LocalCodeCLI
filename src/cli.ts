import { Command } from "commander";
import chalk from "chalk";
import { loadConfig, saveConfig, getConfigPath } from "./config.js";
import { resolveProvider } from "./providers/registry.js";
import { startRepl } from "./repl.js";
import { runSetupWizard, pickModel, pickProvider } from "./ui/setup.js";
import { closeFallbackSelectInterface } from "./ui/boxSelect.js";

export async function run(argv: string[]): Promise<void> {
  const program = new Command();

  program
    .name("local-code")
    .description("A terminal coding agent that works with any model you point it at — local or cloud.")
    .argument("[prompt]", "initial message to send; omit to start an interactive session")
    .option("--provider <id>", "provider id to use (see `local-code config`)")
    .option("-m, --model <name>", "model name to use")
    .option("-y, --yolo", "auto-approve tool calls without asking", false)
    .option("--print", "run one turn non-interactively, print the result, and exit", false)
    .action(async (prompt: string | undefined, options) => {
      const config = loadConfig();
      let providerId = options.provider ?? config.defaultProvider;
      let model = options.model ?? config.defaultModel;

      const interactive = Boolean(process.stdin.isTTY) && !options.print;

      if (!model && interactive) {
        const picked = await runSetupWizard(config, saveConfig);
        closeFallbackSelectInterface();
        if (picked) {
          providerId = picked.providerConfig.id;
          model = picked.model;
        }
      } else if (!model) {
        console.log(
          chalk.yellow(
            `No default model set. Pass --model <name>, or set one with:\n` +
              `  local-code config set-model <name>\n`,
          ),
        );
      }

      let autoApprove = Boolean(options.yolo) || config.autoApprove;
      if (!process.stdin.isTTY) {
        // No TTY to prompt for approval on (piped/scripted invocation) — avoid hanging.
        autoApprove = true;
      }

      await startRepl({
        config,
        providerId,
        model,
        autoApprove,
        cwd: process.cwd(),
        initialPrompt: prompt,
        printOnly: Boolean(options.print),
      });

      if (options.print) process.exit(0);
    });

  program
    .command("models")
    .description("list models available from configured providers")
    .option("--provider <id>", "only check this provider")
    .action(async (options) => {
      const config = loadConfig();
      const targets = options.provider
        ? config.providers.filter((p) => p.id === options.provider)
        : config.providers;

      for (const providerConfig of targets) {
        const provider = resolveProvider(config, providerConfig.id);
        console.log(chalk.bold(`\n${provider.label} (${providerConfig.id}) — ${providerConfig.baseUrl}`));
        try {
          const models = await provider.listModels();
          if (models.length === 0) {
            console.log(chalk.dim("  (no models found — is it running?)"));
          } else {
            for (const m of models) console.log(`  ${m}`);
          }
        } catch (e) {
          console.log(chalk.red(`  unreachable: ${(e as Error).message}`));
        }
      }
      console.log();
    });

  const configCmd = program
    .command("config")
    .description("show or change the resolved configuration")
    .action(() => {
      const config = loadConfig();
      console.log(chalk.dim(`config file: ${getConfigPath()}\n`));
      console.log(JSON.stringify(config, null, 2));
    });

  configCmd
    .command("set-model [name]")
    .description("set the default model (interactive picker if omitted)")
    .action(async (name: string | undefined) => {
      const config = loadConfig();
      let model = name;
      if (!model) {
        const providerConfig = config.providers.find((p) => p.id === config.defaultProvider);
        if (!providerConfig) {
          console.log(chalk.red(`Default provider "${config.defaultProvider}" isn't in the config.`));
          return;
        }
        model = await pickModel(providerConfig);
        closeFallbackSelectInterface();
        if (!model) return;
      }
      config.defaultModel = model;
      saveConfig(config);
      console.log(chalk.green(`Default model set to ${model}`));
    });

  configCmd
    .command("set-provider [id]")
    .description("set the default provider (interactive picker if omitted)")
    .action(async (id: string | undefined) => {
      const config = loadConfig();
      let providerId = id;
      if (!providerId) {
        const providerConfig = await pickProvider(config);
        closeFallbackSelectInterface();
        if (!providerConfig) return;
        providerId = providerConfig.id;
        saveConfig(config); // persist any newly-added custom provider
      } else if (!config.providers.some((p) => p.id === providerId)) {
        console.log(chalk.red(`Unknown provider "${providerId}". Configured: ${config.providers.map((p) => p.id).join(", ")}`));
        return;
      }
      config.defaultProvider = providerId;
      saveConfig(config);
      console.log(chalk.green(`Default provider set to ${providerId}`));
    });

  await program.parseAsync(argv);
}
