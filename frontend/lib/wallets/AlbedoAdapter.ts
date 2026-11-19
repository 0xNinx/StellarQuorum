import { WalletAdapter } from "./types";
import { t } from "@/lib/i18n";

/**
 * The subset of the `@albedo-link/intent` API this adapter uses. Albedo is a
 * web signer: each call opens an albedo.link popup and resolves when the user
 * confirms, or rejects when they close or decline it.
 */
export interface AlbedoIntent {
  publicKey(params: Record<string, unknown>): Promise<{ pubkey: string }>;
  tx(params: { xdr: string; network?: string; pubkey?: string }): Promise<{ signed_envelope_xdr: string }>;
}

declare global {
  interface Window {
    albedo?: AlbedoIntent;
  }
}

/** Albedo's intent library rejects with code -4 when the user declines or closes the popup. */
function isUserRejection(cause: unknown): boolean {
  if (!cause || typeof cause !== "object") return false;
  const { code, message } = cause as { code?: number; message?: string };
  return code === -4 || /reject|cancel|denied/i.test(message ?? "");
}

function toError(cause: unknown): Error {
  if (isUserRejection(cause)) return new Error(t("nav.refused"));
  if (cause instanceof Error) return new Error(cause.message);
  const message = (cause as { message?: string } | null)?.message;
  return new Error(message || t("nav.refused"));
}

export class AlbedoAdapter implements WalletAdapter {
  id = "albedo";
  name = "Albedo";
  private publicKey: string | null = null;

  constructor(private readonly getIntent: () => AlbedoIntent | undefined = () =>
    typeof window !== "undefined" ? window.albedo : undefined) {}

  isAvailable(): boolean {
    return !!this.getIntent();
  }

  isConnected(): boolean {
    return this.publicKey !== null;
  }

  async connect(): Promise<string> {
    const albedo = this.getIntent();
    if (!albedo) throw new Error(t("nav.noWallet"));
    try {
      const { pubkey } = await albedo.publicKey({});
      this.publicKey = pubkey;
      return pubkey;
    } catch (cause) {
      throw toError(cause);
    }
  }

  disconnect(): void {
    // Albedo keeps no session to close; forgetting the key is enough.
    this.publicKey = null;
  }

  async signTransaction(transaction: string): Promise<string> {
    const albedo = this.getIntent();
    if (!albedo) throw new Error(t("nav.noWallet"));
    try {
      const { signed_envelope_xdr } = await albedo.tx({
        xdr: transaction,
        network: "testnet",
        ...(this.publicKey ? { pubkey: this.publicKey } : {}),
      });
      return signed_envelope_xdr;
    } catch (cause) {
      throw toError(cause);
    }
  }
}
