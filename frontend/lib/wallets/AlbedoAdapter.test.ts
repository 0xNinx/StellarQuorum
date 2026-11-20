import { AlbedoAdapter, type AlbedoIntent } from "./AlbedoAdapter";

// Issue #480: Albedo connects and signs through its popup, and a user who
// closes or declines the popup lands in the adapter's rejected state.

const intent = (overrides: Partial<AlbedoIntent> = {}): AlbedoIntent => ({
  publicKey: jest.fn().mockResolvedValue({ pubkey: "GABC" }),
  tx: jest.fn().mockResolvedValue({ signed_envelope_xdr: "SIGNED" }),
  ...overrides,
});

describe("AlbedoAdapter", () => {
  it("is unavailable until the Albedo intent API is present", () => {
    expect(new AlbedoAdapter(() => undefined).isAvailable()).toBe(false);
    expect(new AlbedoAdapter(() => intent()).isAvailable()).toBe(true);
  });

  it("connects and signs on testnet", async () => {
    const api = intent();
    const adapter = new AlbedoAdapter(() => api);

    await expect(adapter.connect()).resolves.toBe("GABC");
    expect(adapter.isConnected()).toBe(true);
    await expect(adapter.signTransaction("XDR")).resolves.toBe("SIGNED");
    expect(api.tx).toHaveBeenCalledWith({ xdr: "XDR", network: "testnet", pubkey: "GABC" });

    adapter.disconnect();
    expect(adapter.isConnected()).toBe(false);
  });

  it("maps a cancelled popup to the rejected state", async () => {
    const cancelled = { code: -4, message: "Action request was rejected by the user" };
    const adapter = new AlbedoAdapter(() =>
      intent({ publicKey: jest.fn().mockRejectedValue(cancelled), tx: jest.fn().mockRejectedValue(cancelled) }),
    );

    await expect(adapter.connect()).rejects.toThrow("Wallet connection was refused.");
    await expect(adapter.signTransaction("XDR")).rejects.toThrow("Wallet connection was refused.");
    expect(adapter.isConnected()).toBe(false);
  });
});
