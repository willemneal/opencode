import { Tool } from "./tool"
import { spawn } from "child_process"
import { Instance } from "../project/instance"
import { Config } from "../config/config"
import { Log } from "../util/log"
import z from "zod"
import path from "path"

const log = Log.create({ service: "wasi-tool" })

export namespace WasiTool {
  export interface Metadata {
    name: string
    version: string
    description: string
    args: {
      name: string
      type: string
      description: string
      required: boolean
      default?: string
    }[]
    errors: { code: number; message: string }[]
    capabilities: {
      read: boolean
      write: boolean
      net: boolean
    }
  }

  export interface ToolConfig {
    path: string
    capabilities?: {
      read?: string[]
      write?: string[]
      net?: boolean
    }
  }

  async function getRunnerPath(): Promise<string> {
    const config = await Config.get()
    const wasiConfig = (config as Record<string, unknown>).wasi as { runner?: string } | undefined
    if (wasiConfig?.runner) {
      return path.resolve(Instance.directory, wasiConfig.runner)
    }
    return path.resolve(Instance.directory, "crates/runner/target/release/wasi-runner")
  }

  async function runRunner(args: string[]): Promise<{ stdout: string; stderr: string; code: number }> {
    const runner = await getRunnerPath()

    return new Promise((resolve, reject) => {
      const proc = spawn(runner, args, {
        cwd: Instance.directory,
        stdio: ["ignore", "pipe", "pipe"],
      })

      let stdout = ""
      let stderr = ""

      proc.stdout.on("data", (data) => {
        stdout += data.toString()
      })

      proc.stderr.on("data", (data) => {
        stderr += data.toString()
      })

      proc.on("close", (code) => {
        resolve({ stdout, stderr, code: code ?? 1 })
      })

      proc.on("error", (err) => {
        reject(err)
      })
    })
  }

  export async function describe(toolPath: string): Promise<Metadata> {
    const result = await runRunner(["describe", toolPath])
    if (result.code !== 0) {
      throw new Error(`Failed to describe tool: ${result.stderr}`)
    }
    return JSON.parse(result.stdout)
  }

  export async function list(dir: string): Promise<string[]> {
    const result = await runRunner(["list", dir])
    if (result.code !== 0) {
      throw new Error(`Failed to list tools: ${result.stderr}`)
    }
    return result.stdout.trim().split("\n").filter(Boolean)
  }

  // Use a passthrough record schema that accepts any string-keyed object
  // The actual validation happens at the Rust level
  const WasiArgsSchema = z.record(z.string(), z.unknown())

  export function create(id: string, toolPath: string, config: ToolConfig): Tool.Info {
    return {
      id,
      init: async () => {
        let metadata: Metadata

        try {
          metadata = await describe(toolPath)
        } catch (err) {
          log.error("failed to get wasi tool metadata", { id, toolPath, err })
          return {
            description: `WASI tool (metadata unavailable): ${toolPath}`,
            parameters: z.object({}),
            async execute(_args: Record<string, never>, _ctx: Tool.Context) {
              return {
                title: "Error",
                output: `Failed to load WASI tool metadata: ${err}`,
                metadata: { error: true },
              }
            },
          }
        }

        // Build description with argument info
        const argDescriptions = metadata.args
          .map((arg) => `- ${arg.name} (${arg.type}${arg.required ? ", required" : ""}): ${arg.description}`)
          .join("\n")

        const fullDescription = `${metadata.description}\n\nArguments:\n${argDescriptions}`

        return {
          description: fullDescription,
          parameters: WasiArgsSchema,
          async execute(args: Record<string, unknown>, _ctx: Tool.Context) {
            const runnerArgs = ["run", toolPath]

            // Add capability flags
            if (config.capabilities?.read) {
              for (const dir of config.capabilities.read) {
                const resolved = dir.replace("${PROJECT}", Instance.directory)
                runnerArgs.push("--allow-read=" + resolved)
              }
            }

            if (config.capabilities?.write) {
              for (const dir of config.capabilities.write) {
                const resolved = dir.replace("${PROJECT}", Instance.directory)
                runnerArgs.push("--allow-write=" + resolved)
              }
            }

            if (config.capabilities?.net) {
              runnerArgs.push("--allow-net")
            }

            // Add separator and tool arguments
            runnerArgs.push("--")

            // Convert args object to positional arguments based on metadata order
            for (const argSpec of metadata.args) {
              const value = args[argSpec.name]
              if (value !== undefined) {
                if (Array.isArray(value)) {
                  runnerArgs.push(...value.map(String))
                } else {
                  runnerArgs.push(String(value))
                }
              }
            }

            const result = await runRunner(runnerArgs)

            // Check for known error codes
            if (result.code !== 0) {
              const errorSpec = metadata.errors.find((e) => e.code === result.code)
              if (errorSpec) {
                return {
                  title: "Error",
                  output: `Tool error (code ${result.code}): ${errorSpec.message}\n${result.stderr}`,
                  metadata: { exitCode: result.code, errorMessage: errorSpec.message },
                }
              }

              return {
                title: "Error",
                output: `Tool failed with exit code ${result.code}\n${result.stderr}`,
                metadata: { exitCode: result.code },
              }
            }

            return {
              title: id,
              output: result.stdout,
              metadata: { exitCode: 0 },
            }
          },
        }
      },
    }
  }
}
