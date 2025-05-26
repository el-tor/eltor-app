import { type FetchWalletBalanceResponseType, IWallet } from "../../IWallet";

export { walletApi };

type PhoenixTypeBalance = {
  balanceSat: number;
  feeCreditSat: number;
};

type PhoenixTypeChannel = {
  state: string;
  channelId: string;
  balanceSat: number;
  inboundLiquiditySat: number;
  capacitySat: number;
  fundingTxId: string;
};

type PhoenixTypeNodeInfo = {
  nodeId: string;
  channels: Array<PhoenixTypeChannel>;
};

declare let window: any
window.electronEvents = {}
window.electronEvents.env = {}

const { electronEvents } = window;




// Client Wallet
const payerUrl = electronEvents.env.TOR_BROWSER_PHOENIXD_URL ?? '';
const username = "";
const payerPassword = electronEvents.env.TOR_BROWSER_PHOENIXD_API_PASSWORD ?? '';
const payerAuth = btoa(`${username}:${payerPassword}`);

// Relay Wallet
const receiverUrl = electronEvents.env.TOR_RELAY_PHOENIXD_URL ?? '';
const receiverPassword = electronEvents.env.TOR_RELAY_PHOENIXD_API_PASSWORD ?? '';
const receiverAuth = btoa(`${username}:${receiverPassword}`) ?? '';

const walletApi: IWallet = {
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
    const res = await fetch(`${receiverUrl}/getoffer`, {
      method: "GET",
      headers: {
        Authorization: `Basic ${receiverAuth}`,
      },
    });
    const offer = await res.text();
    return offer;
  },
  fetchWalletBalance: async (): Promise<FetchWalletBalanceResponseType> => {
    const res = await fetch(`${receiverUrl}/getbalance`, {
      method: "GET",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        Authorization: `Basic ${receiverAuth}`,
      },
    });
    const resp = (await res.json()) as PhoenixTypeBalance;
    return {
      balance: resp.balanceSat,
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
    const res = await fetch(`${receiverUrl}/getinfo`, {
      method: "GET",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        Authorization: `Basic ${receiverAuth}`,
      },
    });
    const resp = (await res.json()) as PhoenixTypeNodeInfo;
    return {
      send: resp.channels[0]?.capacitySat ?? 0,
      receive: resp.channels[0]?.inboundLiquiditySat ?? 0,
    };
  },
  onPaymentReceived: (event: any) => {
    // 1. verify payment
    // 2. write to payment-received file in tor data directory
    //    paymentHash | expires(null) | amount
  },
};

export default walletApi;
