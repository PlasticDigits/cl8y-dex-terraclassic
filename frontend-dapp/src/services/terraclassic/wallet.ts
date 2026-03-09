import {
  ConnectedWallet,
  CosmostationController,
  GalaxyStationController,
  KeplrController,
  LeapController,
  LUNCDashController,
  StationController,
  WalletController,
  WalletName,
  WalletType,
} from '@goblinhunt/cosmes/wallet';
import { NETWORKS, DEFAULT_NETWORK } from '@/utils/constants';

const networkConfig = NETWORKS[DEFAULT_NETWORK].terra;
const TERRA_CLASSIC_CHAIN_ID = networkConfig.chainId;
const WC_PROJECT_ID = '2ce7811b869be33ffad28cff05c93c15';

const GAS_PRICE = {
  amount: '28.325',
  denom: 'uluna',
};

const STATION_CONTROLLER = new StationController();
const KEPLR_CONTROLLER = new KeplrController(WC_PROJECT_ID);
const LUNCDASH_CONTROLLER = new LUNCDashController();
const GALAXY_CONTROLLER = new GalaxyStationController(WC_PROJECT_ID);
const LEAP_CONTROLLER = new LeapController(WC_PROJECT_ID);
const COSMOSTATION_CONTROLLER = new CosmostationController(WC_PROJECT_ID);

const CONTROLLERS: Partial<Record<WalletName, WalletController>> = {
  [WalletName.STATION]: STATION_CONTROLLER,
  [WalletName.KEPLR]: KEPLR_CONTROLLER,
  [WalletName.LUNCDASH]: LUNCDASH_CONTROLLER,
  [WalletName.GALAXYSTATION]: GALAXY_CONTROLLER,
  [WalletName.LEAP]: LEAP_CONTROLLER,
  [WalletName.COSMOSTATION]: COSMOSTATION_CONTROLLER,
};

const connectedWallets: Map<string, ConnectedWallet> = new Map();

function getChainInfo() {
  return {
    chainId: TERRA_CLASSIC_CHAIN_ID,
    rpc: networkConfig.rpc,
    gasPrice: GAS_PRICE,
  };
}

export function isStationInstalled(): boolean {
  return typeof window !== 'undefined' && 'station' in window;
}

export function isKeplrInstalled(): boolean {
  return typeof window !== 'undefined' && !!window.keplr;
}

export async function connectTerraWallet(
  walletName: WalletName = WalletName.STATION,
  walletType: WalletType = WalletType.EXTENSION
): Promise<{ address: string; walletType: 'station' | 'keplr' | 'luncdash' | 'galaxy' | 'leap' | 'cosmostation'; connectionType: WalletType }> {
  const controller = CONTROLLERS[walletName];
  if (!controller) {
    throw new Error(`Unsupported wallet: ${walletName}`);
  }

  try {
    const chainInfo = getChainInfo();
    console.log(`[Wallet] Connecting ${walletName} (${walletType}) to chain ${chainInfo.chainId}`, {
      rpc: chainInfo.rpc,
      gasPrice: chainInfo.gasPrice,
    });

    let wallets: Map<string, ConnectedWallet>;
    try {
      wallets = await controller.connect(walletType, [chainInfo]);
    } catch (connectError: unknown) {
      console.error(`[Wallet] Controller.connect() threw an error:`, connectError);
      const errorMessage = connectError instanceof Error ? connectError.message : String(connectError);
      const errorStack = connectError instanceof Error ? connectError.stack : undefined;
      console.error(`[Wallet] Error details:`, { errorMessage, errorStack });
      throw connectError;
    }

    console.log(`[Wallet] Controller returned ${wallets.size} wallet(s)`, {
      walletName,
      walletType,
      chainIds: Array.from(wallets.keys()),
    });

    if (wallets.size === 0) {
      const isLuncDashWC = walletType === WalletType.WALLETCONNECT && walletName === WalletName.LUNCDASH;
      const isStationWC = walletType === WalletType.WALLETCONNECT && walletName === WalletName.STATION;

      if (isLuncDashWC || isStationWC) {
        const sessionKey = isLuncDashWC
          ? 'cosmes.wallet.luncdash.wcSession'
          : 'cosmes.wallet.station.wcSession';
        const walletDisplayName = isLuncDashWC ? 'LUNC Dash' : 'Station';

        const cachedSession = typeof window !== 'undefined' ? localStorage.getItem(sessionKey) : null;

        console.log(`[${walletDisplayName}] Checking for cached WalletConnect session`, {
          sessionKey,
          hasCachedSession: !!cachedSession,
        });

        let session: { accounts?: string[]; chainId?: number; peerMeta?: unknown } | null = null;
        if (cachedSession) {
          try {
            session = JSON.parse(cachedSession);
          } catch (parseError) {
            console.error(`[${walletDisplayName}] Failed to parse cached session JSON:`, parseError);
            session = null;
          }
        }

        if (session) {
          console.log(`[${walletDisplayName}] Cached session found`, {
            hasAccounts: !!(session && session.accounts),
            accountCount: session?.accounts?.length || 0,
            accounts: session?.accounts,
            peerMeta: session?.peerMeta,
          });

          if (session.accounts && session.accounts.length > 0) {
            const address = session.accounts[0];
            console.log(`[${walletDisplayName}] WalletConnect succeeded but controller returned 0 wallets. Diagnosing...`, {
              address,
              chainId: chainInfo.chainId,
              rpc: chainInfo.rpc,
            });

            const lcdUrl = networkConfig.lcd.replace(':443', '');
            const accountUrl = `${lcdUrl}/cosmos/auth/v1beta1/account_info/${address}`;

            console.log(`[${walletDisplayName}] Manually fetching account info from LCD:`, accountUrl);

            try {
              const accountResponse = await fetch(accountUrl);
              const accountData = await accountResponse.json();

              console.log(`[${walletDisplayName}] Manual account fetch result:`, {
                status: accountResponse.status,
                ok: accountResponse.ok,
                data: accountData,
              });

              if (accountResponse.ok && accountData.info) {
                console.log(`[${walletDisplayName}] Account data retrieved successfully!`);

                const info = accountData.info;
                const hasPubKey = !!info.pub_key;

                console.log(`[${walletDisplayName}] Account details:`, {
                  address: info.address,
                  hasPubKey: hasPubKey,
                  pubKey: info.pub_key,
                  accountNumber: info.account_number,
                  sequence: info.sequence,
                });

                if (!hasPubKey) {
                  throw new Error(
                    `${walletDisplayName} WalletConnect succeeded, but your account (${address}) does not have a public key on-chain yet. ` +
                    `This happens when an account has received funds but never sent a transaction. ` +
                    `Please send any transaction from this wallet first (e.g., a small LUNC transfer to yourself), then try connecting again.`
                  );
                }

                console.log(`[${walletDisplayName}] Account has pub key. The issue is likely that cosmes uses RPC instead of LCD for account queries.`);
                throw new Error(
                  `${walletDisplayName} WalletConnect succeeded and account has pub key, but the cosmes library failed to retrieve it. ` +
                  `This is likely because cosmes uses the RPC endpoint which may not support account queries on Terra Classic. ` +
                  `Address: ${address}. Please check browser console for more details.`
                );
              } else {
                throw new Error(
                  `${walletDisplayName} WalletConnect succeeded, but failed to fetch account info. ` +
                  `Address: ${address}. Status: ${accountResponse.status}. ` +
                  `Response: ${JSON.stringify(accountData)}`
                );
              }
            } catch (fetchError: unknown) {
              if (fetchError instanceof Error &&
                  (fetchError.message.includes('WalletConnect succeeded') ||
                   fetchError.message.includes('does not have a public key'))) {
                throw fetchError;
              }

              const fetchErrorMessage = fetchError instanceof Error ? fetchError.message : String(fetchError);
              console.error(`[${walletDisplayName}] Failed to manually fetch account:`, fetchError);

              throw new Error(
                `${walletDisplayName} WalletConnect succeeded, but cannot reach LCD to verify account. ` +
                `Address: ${address}. LCD: ${accountUrl}. ` +
                `Error: ${fetchErrorMessage}`
              );
            }
          }
        }

        throw new Error(
          `${walletDisplayName} connection failed: Unable to retrieve wallet information. ` +
          'The mobile wallet may be connected, but the dapp could not verify the connection. ' +
          'Please check the browser console for detailed logs and try disconnecting and reconnecting.'
        );
      }

      throw new Error('No wallets connected');
    }

    const wallet = wallets.get(TERRA_CLASSIC_CHAIN_ID);
    if (!wallet) {
      throw new Error(`Failed to connect to Terra Classic chain (${TERRA_CLASSIC_CHAIN_ID})`);
    }

    connectedWallets.set(TERRA_CLASSIC_CHAIN_ID, wallet);

    let walletTypeStr: 'station' | 'keplr' | 'luncdash' | 'galaxy' | 'leap' | 'cosmostation';
    if (walletName === WalletName.STATION) {
      walletTypeStr = 'station';
    } else if (walletName === WalletName.LUNCDASH) {
      walletTypeStr = 'luncdash';
    } else if (walletName === WalletName.GALAXYSTATION) {
      walletTypeStr = 'galaxy';
    } else if (walletName === WalletName.LEAP) {
      walletTypeStr = 'leap';
    } else if (walletName === WalletName.COSMOSTATION) {
      walletTypeStr = 'cosmostation';
    } else {
      walletTypeStr = 'keplr';
    }

    return {
      address: wallet.address,
      walletType: walletTypeStr,
      connectionType: walletType,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : 'Unknown error';

    if (walletName === WalletName.KEPLR) {
      if (errorMessage.includes('not installed') || errorMessage.includes('Keplr')) {
        throw new Error('Keplr wallet is not installed. Please install the Keplr extension.');
      }
      if (errorMessage.includes('chain') && errorMessage.includes('not found')) {
        throw new Error('Terra Classic chain not found in Keplr. Please add it manually or refresh the page.');
      }
    }

    if (walletName === WalletName.STATION) {
      if (errorMessage.includes('not installed') || errorMessage.includes('Station')) {
        throw new Error('Station wallet is not installed. Please install the Station extension.');
      }
    }

    if (errorMessage.includes('User rejected') || errorMessage.includes('rejected')) {
      throw new Error('Connection rejected by user');
    }

    let walletDisplayName = 'wallet';
    if (walletName === WalletName.STATION) {
      walletDisplayName = 'Station';
    } else if (walletName === WalletName.KEPLR) {
      walletDisplayName = 'Keplr';
    } else if (walletName === WalletName.LUNCDASH) {
      walletDisplayName = 'LuncDash';
    } else if (walletName === WalletName.GALAXYSTATION) {
      walletDisplayName = 'Galaxy';
    } else if (walletName === WalletName.LEAP) {
      walletDisplayName = 'Leap';
    } else if (walletName === WalletName.COSMOSTATION) {
      walletDisplayName = 'Cosmostation';
    }

    throw new Error(`Failed to connect ${walletDisplayName}: ${errorMessage}`);
  }
}

export async function disconnectTerraWallet(): Promise<void> {
  const wallet = connectedWallets.get(TERRA_CLASSIC_CHAIN_ID);
  if (wallet) {
    const controller = CONTROLLERS[wallet.id];
    if (controller) {
      controller.disconnect([TERRA_CLASSIC_CHAIN_ID]);
    }
    connectedWallets.delete(TERRA_CLASSIC_CHAIN_ID);
  }
}

export function getConnectedWallet(): ConnectedWallet | null {
  return connectedWallets.get(TERRA_CLASSIC_CHAIN_ID) || null;
}

export async function getCurrentTerraAddress(): Promise<string | null> {
  const wallet = connectedWallets.get(TERRA_CLASSIC_CHAIN_ID);
  if (wallet) {
    return wallet.address;
  }

  try {
    if (isStationInstalled()) {
      const result = await connectTerraWallet(WalletName.STATION, WalletType.EXTENSION);
      return result.address;
    } else if (isKeplrInstalled()) {
      const result = await connectTerraWallet(WalletName.KEPLR, WalletType.EXTENSION);
      return result.address;
    }
  } catch {
    // Ignore errors on auto-connect
  }

  return null;
}

export async function isTerraWalletConnected(): Promise<boolean> {
  const address = await getCurrentTerraAddress();
  return address !== null;
}

declare global {
  interface Window {
    station?: {
      connect: () => Promise<void>;
      disconnect: () => Promise<void>;
    };
    keplr?: {
      enable: (chainId: string) => Promise<void>;
      getOfflineSigner: (chainId: string) => unknown;
    };
  }
}
