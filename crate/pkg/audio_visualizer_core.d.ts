/* tslint:disable */
/* eslint-disable */
export class AudioVisualizer {
  free(): void;
  constructor();
  /**
   * @returns {number}
   */
  get_upload_buffer_ptr(): number;
  /**
   * @param {number} width
   * @param {number} height
   */
  load_image(width: number, height: number): void;
  /**
   * @param {number} factor
   */
  set_decay_factor(factor: number): void;
  /**
   * @param {number} gain
   */
  set_global_gain(gain: number): void;
  /**
   * @param {number} count
   */
  set_color_count(count: number): void;
  /**
   * @param {number} size
   */
  resize_input_buffer(size: number): void;
  /**
   * @returns {number}
   */
  get_input_buffer_ptr(): number;
  /**
   * @returns {number}
   */
  process_frequencies(): number;
  render(): void;
  /**
   * @returns {number}
   */
  get_display_buffer_ptr(): number;
  /**
   * @returns {number}
   */
  get_display_buffer_len(): number;
  /**
   * @returns {number}
   */
  get_width(): number;
  /**
   * @returns {number}
   */
  get_height(): number;
  /**
   * @returns {number}
   */
  get_spectrum_ptr(): number;
  /**
   * @returns {number}
   */
  get_palette_ptr(): number;
  /**
   * @returns {number}
   */
  get_active_color_count(): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_audiovisualizer_free: (a: number, b: number) => void;
  readonly audiovisualizer_new: () => number;
  readonly audiovisualizer_get_upload_buffer_ptr: (a: number) => number;
  readonly audiovisualizer_load_image: (a: number, b: number, c: number) => void;
  readonly audiovisualizer_set_decay_factor: (a: number, b: number) => void;
  readonly audiovisualizer_set_global_gain: (a: number, b: number) => void;
  readonly audiovisualizer_set_color_count: (a: number, b: number) => void;
  readonly audiovisualizer_resize_input_buffer: (a: number, b: number) => void;
  readonly audiovisualizer_get_input_buffer_ptr: (a: number) => number;
  readonly audiovisualizer_process_frequencies: (a: number) => number;
  readonly audiovisualizer_render: (a: number) => void;
  readonly audiovisualizer_get_display_buffer_ptr: (a: number) => number;
  readonly audiovisualizer_get_display_buffer_len: (a: number) => number;
  readonly audiovisualizer_get_width: (a: number) => number;
  readonly audiovisualizer_get_height: (a: number) => number;
  readonly audiovisualizer_get_spectrum_ptr: (a: number) => number;
  readonly audiovisualizer_get_palette_ptr: (a: number) => number;
  readonly audiovisualizer_get_active_color_count: (a: number) => number;
  readonly __wbindgen_export_0: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
