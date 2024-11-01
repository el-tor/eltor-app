export interface IWallet {
  getWalletTransactions: (walletId: string) => Promise<any>;
  payInvoice: (invoice: string) => Promise<string>;
  getBolt12Offer: () => Promise<string>;
  fetchWalletBalance: () => Promise<FetchWalletBalanceResponseType>;
  decodeInvoice: (invoice: string) => Promise<any>;
  checkPaymentStatus: (paymentId: string) => Promise<any>;
  fetchChannelInfo: (channelId: string) => Promise<FetchChannelInfoResponseType>;
  onPaymentReceived: (event: any) => void;
}

export type {
  FetchWalletBalanceResponseType,
  WalletProviderType,
  FetchChannelInfoResponseType
}


type FetchWalletBalanceResponseType = {
  balance: number;
};

type FetchChannelInfoResponseType = {
  send: number;
  receive: number;
};

type WalletProviderType = "Phoenix" | "Lndk" | "CoreLightning" | "None";