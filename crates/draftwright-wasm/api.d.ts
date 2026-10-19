export class Engine {
  [Symbol.dispose](): void;
  apply(ops: string, commit: boolean): string;
  applyFragments(ops: string): string;
  calloutOffset(address: string): string;
  constructor(name: string, bytes: Uint8Array);
  faceCount(): number;
  featureCount(): number;
  fragments(): string;
  free(): void;
  labelOffset(address: string): string;
  placementOptions(): string;
  plan(): string;
  planCount(): number;
  pmi(): string;
  preview(ops: string): string;
  redo(): string;
  renderPdf(): Uint8Array;
  renderSvg(): Uint8Array;
  setPmi(source: string, bytes: Uint8Array): void;
  setPmiOps(source: string, bytes: Uint8Array, ops: string): void;
  setSpec(json: string): void;
  sheetFragments(sheet: number): string;
  sheetSvg(sheet: number): Uint8Array;
  snapshot(): string;
  spec(): string;
  undo(): string;
}

export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;

export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

export function start(): void;

export interface InitOutput {
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbg_engine_free: (a: number, b: number) => void;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_start: () => void;
  readonly engine_apply: (a: number, b: number, c: number, d: number) => [number, number, number, number];
  readonly engine_applyFragments: (a: number, b: number, c: number) => [number, number, number, number];
  readonly engine_calloutOffset: (a: number, b: number, c: number) => [number, number, number, number];
  readonly engine_faceCount: (a: number) => number;
  readonly engine_featureCount: (a: number) => number;
  readonly engine_fragments: (a: number) => [number, number];
  readonly engine_labelOffset: (a: number, b: number, c: number) => [number, number, number, number];
  readonly engine_open: (a: number, b: number, c: number, d: number) => [number, number, number];
  readonly engine_placementOptions: (a: number) => [number, number, number, number];
  readonly engine_plan: (a: number) => [number, number, number, number];
  readonly engine_planCount: (a: number) => number;
  readonly engine_pmi: (a: number) => [number, number];
  readonly engine_preview: (a: number, b: number, c: number) => [number, number, number, number];
  readonly engine_redo: (a: number) => [number, number, number, number];
  readonly engine_renderPdf: (a: number) => [number, number, number, number];
  readonly engine_renderSvg: (a: number) => [number, number, number, number];
  readonly engine_setPmi: (a: number, b: number, c: number, d: number, e: number) => [number, number];
  readonly engine_setPmiOps: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
  readonly engine_setSpec: (a: number, b: number, c: number) => [number, number];
  readonly engine_sheetFragments: (a: number, b: number) => [number, number, number, number];
  readonly engine_sheetSvg: (a: number, b: number) => [number, number, number, number];
  readonly engine_snapshot: (a: number) => [number, number, number, number];
  readonly engine_spec: (a: number) => [number, number];
  readonly engine_undo: (a: number) => [number, number, number, number];
  readonly memory: WebAssembly.Memory;
  readonly start: () => void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export type SyncInitInput = BufferSource | WebAssembly.Module;
