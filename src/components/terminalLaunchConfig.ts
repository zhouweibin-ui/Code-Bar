export interface TerminalLaunchConfig {
  command: string;
  args: string[];
}

function shellQuote(value: string) {
  if (!value) return "''";
  return `'${value.replace(/'/g, `'"'"'`)}'`;
}

function powershellSingleQuote(value: string) {
  return `'${value.replace(/'/g, "''")}'`;
}

export function buildTerminalLaunchConfig(workdir: string, isWindows: boolean): TerminalLaunchConfig {
  if (isWindows) {
    return {
      command: "pwsh.exe",
      args: [
        "-NoLogo",
        "-NoExit",
        "-Command",
        `Set-Location -LiteralPath ${powershellSingleQuote(workdir)}`,
      ],
    };
  }

  return {
    command: "sh",
    args: ["-lc", `cd ${shellQuote(workdir)} && exec zsh -i`],
  };
}
