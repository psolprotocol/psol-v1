/**
 * Anchor Program Wrapper for pSol Privacy Pool
 * 
 * This is a placeholder module. To use with Anchor:
 * 1. Generate IDL: anchor build
 * 2. Copy target/idl/psol_privacy.json to src/idl/
 * 3. Import and use Program from @coral-xyz/anchor
 */

import { Connection, PublicKey } from '@solana/web3.js';
import { AnchorProvider, Wallet } from '@coral-xyz/anchor';
import { PROGRAM_ID } from '../types';

/**
 * Wrapper for Anchor program interactions
 * Extend this class with actual Anchor program methods
 */
export class PsolProgram {
  public connection: Connection;
  public wallet: Wallet;
  public programId: PublicKey;
  public provider: AnchorProvider;

  constructor(connection: Connection, wallet: Wallet, programId: PublicKey = PROGRAM_ID) {
    this.connection = connection;
    this.wallet = wallet;
    this.programId = programId;
    this.provider = new AnchorProvider(connection, wallet, {
      commitment: 'confirmed',
    });
  }

  /**
   * Get the program instance
   * Implement this after generating IDL
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  getProgram(): any {
    throw new Error(
      'Not implemented. Import your IDL and create Program instance:\n' +
      'import { Program } from "@coral-xyz/anchor";\n' +
      'import idl from "./idl/psol_privacy.json";\n' +
      'return new Program(idl, this.provider);'
    );
  }
}

export default PsolProgram;
