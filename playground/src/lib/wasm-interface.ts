import init, { LspServer } from '../wasm/unified_sql_lsp_lsp.js'

let wasmInstance: LspServer | null = null

export async function initWasm(dialect: string = 'mysql'): Promise<LspServer> {
  if (wasmInstance) {
    return wasmInstance
  }

  await init()
  wasmInstance = new LspServer(dialect)
  return wasmInstance
}

export function getWasmInstance(): LspServer | null {
  return wasmInstance
}
