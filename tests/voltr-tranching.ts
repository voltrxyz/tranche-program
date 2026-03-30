import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { VoltrTranching } from "../target/types/voltr_tranching";

describe("voltr-tranching", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.VoltrTranching as Program<VoltrTranching>;

  it("loads the scaffolded program", async () => {
    expect(program.programId.toBase58()).to.equal(
      "EvVxBE31HKE3QdRZXZB7gkkPTcMcAPKGTWJKAvoGuZoX"
    );
    expect(program.idl.instructions).to.have.length(0);
  });
});
