// world root:component/root
export type City = import('./interfaces/teeline-solver-types.js').City;
export type SolveOptions = import('./interfaces/teeline-solver-types.js').SolveOptions;
export type Solution = import('./interfaces/teeline-solver-types.js').Solution;
export type * as TeelineSolverTypes010 from './interfaces/teeline-solver-types.js'; // import teeline:solver/types@0.1.0
export type * as WasiCliEnvironment023 from './interfaces/wasi-cli-environment.js'; // import wasi:cli/environment@0.2.3
export type * as WasiCliExit023 from './interfaces/wasi-cli-exit.js'; // import wasi:cli/exit@0.2.3
export type * as WasiCliStderr023 from './interfaces/wasi-cli-stderr.js'; // import wasi:cli/stderr@0.2.3
export type * as WasiCliStdin023 from './interfaces/wasi-cli-stdin.js'; // import wasi:cli/stdin@0.2.3
export type * as WasiCliStdout023 from './interfaces/wasi-cli-stdout.js'; // import wasi:cli/stdout@0.2.3
export type * as WasiClocksMonotonicClock023 from './interfaces/wasi-clocks-monotonic-clock.js'; // import wasi:clocks/monotonic-clock@0.2.3
export type * as WasiClocksWallClock023 from './interfaces/wasi-clocks-wall-clock.js'; // import wasi:clocks/wall-clock@0.2.3
export type * as WasiFilesystemPreopens023 from './interfaces/wasi-filesystem-preopens.js'; // import wasi:filesystem/preopens@0.2.3
export type * as WasiFilesystemTypes023 from './interfaces/wasi-filesystem-types.js'; // import wasi:filesystem/types@0.2.3
export type * as WasiIoError023 from './interfaces/wasi-io-error.js'; // import wasi:io/error@0.2.3
export type * as WasiIoStreams023 from './interfaces/wasi-io-streams.js'; // import wasi:io/streams@0.2.3
export type * as WasiRandomRandom023 from './interfaces/wasi-random-random.js'; // import wasi:random/random@0.2.3
export function solve(solver: string, cities: Array<City>, options: SolveOptions): Solution;
export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };
