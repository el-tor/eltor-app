import { type ClassValue, clsx } from 'clsx'
import { WalletProviderType } from 'renderer/drivers/IWallet';
// import { twMerge } from 'tailwind-merge'

export {
  cn,
  dynamicWalletImport
}

function cn(...inputs: ClassValue[]) {
  // return twMerge(clsx(inputs))
}


function dynamicWalletImport<T>(defaultWallet: WalletProviderType) {
  const module = import.meta.glob("../drivers/wallets/**/*.ts", {
    eager: true,
  });
  let dynImport: any;
  // Find the path that matches the defaultWallet (dynamicImport) name
  for (const path in module) {
    if (path.includes(`${defaultWallet.toLocaleLowerCase()}`)) {
      // Here we're assuming there's only one file per wallet in its directory,
      // or we're interested in the first match. Adjust as necessary if this assumption doesn't hold.
      dynImport = module[path];
      break;
    }
  }
  return dynImport.default as T;
}
