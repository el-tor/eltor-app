import { useEffect, ReactNode } from "react";
import { useLocalStorage } from "usehooks-ts";
import { type WalletProviderType } from "renderer/drivers/IWallet";

export {
  LocalStorage
}

const LocalStorage = ({ children }: { children: ReactNode }) => {
  const [torActive, setTorActive, removeTorActive] = useLocalStorage(
    "torActive",
    "false"
  );

  const [relayActive, setRelayActive, removeRelayActive] = useLocalStorage(
    "relayActive",
    "false"
  );

  
  const [defaultWallet, setDefaultWallet, removeDefaultWallet] =
    useLocalStorage("defaultWallet", "None" as WalletProviderType);

  useEffect(() => {
    // Initialize the database on load
    setTorActive("false");
    setRelayActive("false");
  }, []);

  return <>{children}</>;
};

