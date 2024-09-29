import { useEffect, ReactNode } from "react";
import { useLocalStorage } from "usehooks-ts";

export {
  type WalletProviderType,
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

type WalletProviderType = "Phoenix" | "Lndk" | "CoreLightning" | "None";
