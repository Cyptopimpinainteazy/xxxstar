import { BrowserProvider } from "ethers";

describe("MetaMask EIP-1193 connectivity", () => {
  it("connects ethers to an injected provider and reads its chain and accounts", async () => {
    const request = jest.fn(async ({ method }: { method: string }) => {
      if (method === "eth_chainId") return "0x539";
      if (method === "eth_accounts") {
        return ["0x0000000000000000000000000000000000000001"];
      }
      throw new Error(`unsupported RPC method: ${method}`);
    });

    const provider = new BrowserProvider({ request });
    const network = await provider.getNetwork();
    const accounts = await provider.send("eth_accounts", []);

    expect(network.chainId).toBe(BigInt("1337"));
    expect(accounts).toEqual(["0x0000000000000000000000000000000000000001"]);
    expect(request).toHaveBeenCalledWith(expect.objectContaining({ method: "eth_chainId" }));
  });

  it("propagates provider rejection instead of silently connecting", async () => {
    const request = jest.fn(async ({ method }: { method: string }) => {
      if (method === "eth_chainId") return "0x539";
      throw Object.assign(new Error("User rejected request"), { code: 4001 });
    });
    const provider = new BrowserProvider({
      request,
    });

    await expect(provider.send("eth_requestAccounts", [])).rejects.toThrow(
      "User rejected request",
    );
    expect(request).toHaveBeenCalledWith(expect.objectContaining({ method: "eth_requestAccounts" }));
  });
});
