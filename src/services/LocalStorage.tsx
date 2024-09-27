import { useEffect, ReactNode } from "react";
import { useLocalStorage } from "usehooks-ts";

export const LocalStorage = ({ children }: { children: ReactNode }) => {
  const [torActive, setTorActive, removeTorActive] = useLocalStorage(
    "torActive",
    "false"
  );

  const [relayActive, setRelayActive, removeRelayActive] = useLocalStorage(
    "relayActive",
    "false"
  );

  useEffect(() => {
    // Initialize the database on load
    setTorActive("false");
    setRelayActive("false");
  }, []);

  return <>{children}</>;
};
