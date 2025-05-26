import { ScrollArea, Table } from "@mantine/core";
import { useEffect } from "react";
import { useDispatch, useSelector } from "renderer/hooks";
import { fetchTransactions } from "./walletStore";

export function Transactions({ h }: { h?: number | string }) {
  const { transactions } = useSelector((state) => state.wallet);
  const dispatch = useDispatch();

  useEffect(() => {
    console.log("Transactions component mounted");
    dispatch(fetchTransactions(""));
  }, []);

  return (
    <ScrollArea
      w="100%"
      h={h ?? 200}
      p="sm"
      style={{ borderRadius: "6px" }}
      bg="#1e1e1e"
    >
      <Table
        bg="#1e1e1e"
        withRowBorders={false}
        highlightOnHover
        title="Transactions"
      >
        <Table.Thead>
          <Table.Tr>
            <Table.Th>Created at</Table.Th>
            <Table.Th>Amount</Table.Th>
            <Table.Th>Payment Hash</Table.Th>
            <Table.Th>Preimage</Table.Th>
            <Table.Th>Payer Note</Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>
          {transactions?.length > 0 &&
            transactions.map((element) => (
              <Table.Tr key={element.paymentHash}>
                <Table.Td>{element.createdAt}</Table.Td>
                <Table.Td>{Number(element.amountMsats) / 1000}</Table.Td>
                <Table.Td>{element.paymentHash}</Table.Td>
                <Table.Td>{element.preimage}</Table.Td>
                <Table.Td>{element.payerNote}</Table.Td>
              </Table.Tr>
            ))}
        </Table.Tbody>
      </Table>
    </ScrollArea>
  );
}
