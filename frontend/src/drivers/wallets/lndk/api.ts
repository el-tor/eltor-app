import { IWallet } from "../../IWallet";
// TODO: implement lndk api

export const Wallet: IWallet = {
  getWalletTransactions: async (walletId: string) => {
    return {
      transactions: [
        {
          amount: 100,
          date: new Date(),
          memo: "Payment from Bob",
        },
        {
          amount: -50,
          date: new Date(),
          memo: "Payment to Alice",
        },
      ],
    };
  },
  payInvoice: async (invoice: string) => {
    return "Payment successful";
  },
  getBolt12Offer: async () => {
    return "lno";
  },
  fetchWalletBalance: async () => {
    return {
      balance: 1000,
    };
  },
  decodeInvoice: async (invoice: string) => {
    return {
      amount: 100,
      memo: "Payment from Bob",
    };
  },
  checkPaymentStatus: async (paymentId: string) => {
    return {
      status: "PAID",
    };
  },
  fetchChannelInfo: async (channelId: string) => {
    return {
      send: 100,
      receive: 50,
    };
  },
  onPaymentReceived: (event: any) => {
    // 1. verify payment
    // 2. write to payment-received file in tor data directory
    //    paymentHash | expires(null) | amount
  },
};
