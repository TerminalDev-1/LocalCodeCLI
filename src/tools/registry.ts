import type { Tool } from "./types.js";
import { readFileTool } from "./readFile.js";
import { writeFileTool } from "./writeFile.js";
import { editFileTool } from "./editFile.js";
import { listDirTool } from "./listDir.js";
import { globTool } from "./glob.js";
import { grepTool } from "./grep.js";
import { bashTool } from "./bash.js";

export const allTools: Tool[] = [
  readFileTool,
  writeFileTool,
  editFileTool,
  listDirTool,
  globTool,
  grepTool,
  bashTool,
];

export const toolsByName = new Map(allTools.map((t) => [t.definition.name, t]));
