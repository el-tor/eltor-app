import { IWallet } from "../../IWallet";
// TODO: implement coreLightning api

export const Wallet: IWallet = {
    getWalletTransactions: async (walletId: string) => {
        return {
            transactions: [
                {
                    amount: 100,
                    date: new Date(),
                    memo: "Payment from Bob"
                },
                {
                    amount: -50,
                    date: new Date(),
                    memo: "Payment to Alice"
                }
            ]
        }
    },
    payInvoice: async (invoice: string) => {
        return "Payment successful"
    },
    requestInvoice: async (amount: number, memo: string) => {
        return {
            paymentHash: "paymentHash",
            paymentRequestId: "paymentRequestId"
        }
    },
    fetchWalletBalance: async () => {
        return {
            balance: 1000
        }
    },
    decodeInvoice: async (invoice: string) => {
        return {
            amount: 100,
            memo: "Payment from Bob"
        }
    },
    checkPaymentStatus: async (paymentId: string) => {
        return {
            status: "PAID"
        }
    },
    fetchChannelInfo: async (channelId: string) => {
        return {
            send: 100,
            receive: 50
        }
    }
}