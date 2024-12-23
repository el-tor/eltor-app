export interface Eltord {
    
    // Client Operations
    lookupRelays: () => Promise<Array<Relay>>;
    extendPaidCircuit: ({
      purpose,
      relays,
    }: ExtendCircuitParams) => Promise<string>;
    createPaymentIdHash: () => Promise<string>;
    testBandwidth: (circId: string) => Promise<number>; // Use ATTACHSTREAM RPC to test bandwidth
    killCircuit: (circId: string) => void; // Use ATTACHSTREAM RPC to test bandwidth
    startClientWatcher: () => void; 
    
    // TODO Relay Operations
    setTorrcConf: (key: string, value: string) => void;
    readPaymentsLedger: (paymentIdHash: string) => void;
    writePaymentsLedger: (row: any) => void;
    startRelayWatcher: () => void;
    verfyPayment: (paymentIdHash: string) => void;
  }
  
  export type Relay = {
    fingerprint: string;
    paymentIdHash: string;
  };
  
  export type Circuit = {};
  
  export type ExtendCircuitParams = {
    purpose: number; // 0: general, X: hidden service etc..
    relays: Array<Relay>;
  };
  