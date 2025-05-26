const path = require("path");
const os = require("os");

// Determine the correct path to the native module
const platform = os.platform();
const arch = os.arch();
// TODO from npm package
const nativeModulePath = path.join(
  __dirname,
  `../../../../lni/bindings/lni_nodejs/lni_js.${platform}-${arch}.node`
);
const { PhoenixdNode } = require(nativeModulePath);
import type { PhoenixdNode as PhoenixdNodeType } from "../../../../../lni/bindings/lni_nodejs/index.d.ts";


export const lni = () => {
  let node: PhoenixdNodeType;
  node = new PhoenixdNode({
    url: process.env.TOR_RELAY_PHOENIXD_URL!,
    password: process.env.TOR_RELAY_PHOENIXD_API_PASSWORD!,
  });

  return {
    listTransactions: async () => {
      const txns = await node.listTransactions({
        from: 0,
        until: 0,
        limit: 10,
        offset: 0,
        unpaid: false,
        invoiceType: "all",
      });
      return txns;
    },
    getInfo: async () => {
      const info = await node.getInfo();
      return info;
    }
  };
};

export type Transaction = Awaited<ReturnType<Awaited<ReturnType<typeof lni>>["listTransactions"]>>[number];

