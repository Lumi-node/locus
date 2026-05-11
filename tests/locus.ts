// Anchor integration tests for the Locus program.
//
// Run with:  anchor test
//
// Covers the full lifecycle: initialize_agent -> commit_memory -> attest_retrieval.

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Locus } from "../target/types/locus";
import { PublicKey, Keypair, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";
import { createHash } from "crypto";
import { assert } from "chai";

describe("locus", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Locus as Program<Locus>;
  const owner = (provider.wallet as anchor.Wallet).payer;

  const [agentPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("agent"), owner.publicKey.toBuffer()],
    program.programId,
  );

  it("initializes an agent", async () => {
    await program.methods
      .initializeAgent(new anchor.BN(1000), "ipfs://demo")
      .accounts({
        agentMemory: agentPda,
        owner: owner.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const acct = await program.account.agentMemory.fetch(agentPda);
    assert.equal(acct.owner.toBase58(), owner.publicKey.toBase58());
    assert.equal(acct.readFeeLamports.toNumber(), 1000);
    assert.equal(acct.version.toNumber(), 0);
  });

  it("commits a memory root", async () => {
    const root = Array.from(createHash("sha256").update("first root").digest());
    await program.methods
      .commitMemory(root)
      .accounts({ agentMemory: agentPda, owner: owner.publicKey })
      .rpc();
    const acct = await program.account.agentMemory.fetch(agentPda);
    assert.equal(acct.version.toNumber(), 1);
    assert.equal(acct.writeCount.toNumber(), 1);
  });

  it("attests a retrieval and pays the fee", async () => {
    const requester = Keypair.generate();
    const sig = await provider.connection.requestAirdrop(requester.publicKey, LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig);

    const queryHash = Array.from(createHash("sha256").update("q1").digest());
    const resultHash = Array.from(createHash("sha256").update("r1").digest());
    const nonce = new anchor.BN(1);
    const acctBefore = await program.account.agentMemory.fetch(agentPda);

    const [attestPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("attest"),
        agentPda.toBuffer(),
        acctBefore.version.toArrayLike(Buffer, "le", 8),
        nonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId,
    );

    await program.methods
      .attestRetrieval(queryHash, resultHash, nonce)
      .accounts({
        agentMemory: agentPda,
        owner: owner.publicKey,
        attestation: attestPda,
        requester: requester.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([requester])
      .rpc();

    const att = await program.account.retrievalAttestation.fetch(attestPda);
    assert.deepEqual(Array.from(att.queryHash), queryHash);
    assert.deepEqual(Array.from(att.resultHash), resultHash);
    assert.equal(att.requester.toBase58(), requester.publicKey.toBase58());

    const acctAfter = await program.account.agentMemory.fetch(agentPda);
    assert.equal(acctAfter.readCount.toNumber(), 1);
  });
});
