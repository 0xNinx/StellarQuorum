import { rpc } from '@stellar/stellar-sdk';
import { jest } from '@jest/globals';
import { QuorumClient, TESTNET } from '../src/client';
import type { QuorumClientConfig } from '../src/types';

const getAccount = jest.spyOn(rpc.Server.prototype, 'getAccount');
const prepareTransaction = jest.spyOn(rpc.Server.prototype, 'prepareTransaction');

const CONTRACT = 'CAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQC526';
const ADMIN = 'GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H';
const NEW_ADMIN = 'GDZST3XVCDTUJ76ZAV2HA72KYDN4F37RUAMN5QXPJHX2W3UMMXDAN2X';
const TARGET = 'CA7QYNF5E6QXB5RBUP7W5ODCPQYBZQXKZ5FKBRSGGN3RNSHXR2QK2QJ';

const client = new QuorumClient({
  ...TESTNET,
  governanceContractId: CONTRACT,
  tokenContractId: CONTRACT,
} as QuorumClientConfig);

const mockAccount = {
  account_id: ADMIN,
  sequence: '123',
};

afterEach(() => {
  getAccount.mockReset();
  prepareTransaction.mockReset();
});

describe('admin transaction builders (issue #457)', () => {
  beforeEach(() => {
    getAccount.mockResolvedValue(mockAccount);
    prepareTransaction.mockImplementation((tx) => Promise.resolve(tx));
  });

  it('buildPause returns XDR', async () => {
    const xdr = await client.buildPause(ADMIN);
    expect(typeof xdr).toBe('string');
  });

  it('buildUnpause returns XDR', async () => {
    const xdr = await client.buildUnpause(ADMIN);
    expect(typeof xdr).toBe('string');
  });

  it('buildSetGuardian returns XDR', async () => {
    const xdr = await client.buildSetGuardian(ADMIN, NEW_ADMIN);
    expect(typeof xdr).toBe('string');
  });

  it('buildSetProposalDeposit returns XDR', async () => {
    const xdr = await client.buildSetProposalDeposit(ADMIN, 500n);
    expect(typeof xdr).toBe('string');
  });

  it('buildSetProposerCooldown returns XDR', async () => {
    const xdr = await client.buildSetProposerCooldown(ADMIN, 1000);
    expect(typeof xdr).toBe('string');
  });

  it('buildAllowTarget returns XDR', async () => {
    const xdr = await client.buildAllowTarget(ADMIN, TARGET);
    expect(typeof xdr).toBe('string');
  });

  it('buildDisallowTarget returns XDR', async () => {
    const xdr = await client.buildDisallowTarget(ADMIN, TARGET);
    expect(typeof xdr).toBe('string');
  });

  it('buildTransferAdmin returns XDR', async () => {
    const xdr = await client.buildTransferAdmin(ADMIN, NEW_ADMIN);
    expect(typeof xdr).toBe('string');
  });

  it('buildAcceptAdmin returns XDR', async () => {
    const xdr = await client.buildAcceptAdmin(NEW_ADMIN);
    expect(typeof xdr).toBe('string');
  });

  it('buildCancelAdminTransfer returns XDR', async () => {
    const xdr = await client.buildCancelAdminTransfer(ADMIN);
    expect(typeof xdr).toBe('string');
  });

  it('buildUpgrade returns XDR', async () => {
    const wasmHash = 'aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899';
    const xdr = await client.buildUpgrade(ADMIN, wasmHash);
    expect(typeof xdr).toBe('string');
  });

  it('uses source account for fee when specified', async () => {
    await client.buildPause(ADMIN, NEW_ADMIN);
    const account = getAccount.mock.calls[0][0];
    expect(account).toBe(NEW_ADMIN);
  });
});
