declare module 'circomlibjs' {
  export interface Poseidon {
    (inputs: (bigint | Uint8Array)[]): Uint8Array;
    F: {
      toString(value: bigint | Uint8Array, radix?: number): string;
      e(value: bigint | string | number): bigint;
    };
  }

  export function buildPoseidon(): Promise<Poseidon>;
}
