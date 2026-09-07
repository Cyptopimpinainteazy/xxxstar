import { PublicKey } from "@solana/web3.js";

interface PhantomProvider {
  isPhantom: boolean;
  connect(): Promise<{ publicKey: PublicKey }>;
}

async function connectPhantom(provider: PhantomProvider): Promise<PublicKey> {
  if (!provider.isPhantom) throw new Error("Phantom provider required");
  return (await provider.connect()).publicKey;
}

describe("Phantom provider connectivity", () => {
  it("accepts the public key returned by the injected provider", async () => {
    const expected = new PublicKey("11111111111111111111111111111111");
    const provider: PhantomProvider = {
      isPhantom: true,
      connect: jest.fn(async () => ({ publicKey: expected })),
    };

    expect((await connectPhantom(provider)).equals(expected)).toBe(true);
    expect(provider.connect).toHaveBeenCalledTimes(1);
  });

  it("rejects a non-Phantom injected provider", async () => {
    const provider: PhantomProvider = {
      isPhantom: false,
      connect: jest.fn(),
    };

    await expect(connectPhantom(provider)).rejects.toThrow("Phantom provider required");
    expect(provider.connect).not.toHaveBeenCalled();
  });
});
