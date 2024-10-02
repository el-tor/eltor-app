export interface IWallet {
  getWalletTransactions: (walletId: string) => Promise<any>;
  payInvoice: (invoice: string) => Promise<string>;
  requestInvoice: (
    amount: number,
    memo: string
  ) => Promise<{ paymentHash: string; paymentRequestId: string }>;
  fetchWalletBalance: () => Promise<FetchWalletBalanceResponseType>;
  decodeInvoice: (invoice: string) => Promise<any>;
  checkPaymentStatus: (paymentId: string) => Promise<any>;
  fetchChannelInfo?: (channelId: string) => Promise<FetchChannelBalanceResponseType>;
}

export type {
  FetchWalletBalanceResponseType,
  WalletProviderType,
  FetchChannelBalanceResponseType
}


type FetchWalletBalanceResponseType = {
  balance: number;
};

type FetchChannelBalanceResponseType = {
  send: number;
  receive: number;
};

type WalletProviderType = "Phoenix" | "Lndk" | "CoreLightning" | "None";