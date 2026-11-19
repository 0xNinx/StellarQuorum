import { render, screen, waitFor } from "@testing-library/react";
import DelegateStatus from "./DelegateStatus";
import { getDelegate } from "@/lib/delegation";

jest.mock("@/lib/delegation", () => ({ getDelegate: jest.fn() }));
const mockGetDelegate = getDelegate as jest.MockedFunction<typeof getDelegate>;

const HOLDER = "GHOLDERAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA1";
const DELEGATE = "GDELEGATEBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB8YPQ";

describe("DelegateStatus", () => {
  afterEach(() => mockGetDelegate.mockReset());

  it("shows Self when the connected address has not delegated", async () => {
    mockGetDelegate.mockResolvedValue(HOLDER);
    render(<DelegateStatus address={HOLDER} />);

    const link = await screen.findByRole("link", { name: /delegate: self/i });
    expect(link).toHaveTextContent("Delegate: Self");
    expect(link).toHaveAttribute("href", "/delegation");
    expect(mockGetDelegate).toHaveBeenCalledWith(HOLDER);
  });

  it("shows the delegate's address when voting power is delegated", async () => {
    mockGetDelegate.mockResolvedValue(DELEGATE);
    render(<DelegateStatus address={HOLDER} />);

    const link = await screen.findByRole("link", { name: new RegExp(DELEGATE) });
    expect(link).toHaveTextContent("Delegate: …BB8YPQ");
    expect(link).toHaveAttribute("href", "/delegation");
  });

  it("renders nothing when the read fails", async () => {
    mockGetDelegate.mockRejectedValue(new Error("RPC unavailable"));
    const { container } = render(<DelegateStatus address={HOLDER} />);

    await waitFor(() => expect(mockGetDelegate).toHaveBeenCalled());
    expect(container).toBeEmptyDOMElement();
  });
});
