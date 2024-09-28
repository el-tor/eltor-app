export interface IWallet {
    getWalletTransactions: (walletId:string) => Promise<any>,
    payInvoice: (invoice:string) => Promise<string>,
    requestInvoice: (amount:number, memo: string) => Promise<{paymentHash:string, paymentRequestId: string}>,
    getWalletBalance: () => Promise<any>,
    decodeInvoice: (invoice: string)=> Promise<any>,
    checkPaymentStatus: (paymentId: string)=> Promise<any>
  }