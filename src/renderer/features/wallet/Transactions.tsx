import { ScrollArea, Table } from "@mantine/core";

const txns = [
  { id: 6, amount: 12.011, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 7, amount: 14.007, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 39, amount: 88.906, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 56, amount: 137.33, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 58, amount: 140.12, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 77, amount: 12.011, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 86, amount: 14.007, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 99, amount: 88.906, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 102, amount: 137.33, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 201, amount: 140.12, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 203, amount: 88.906, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 205, amount: 137.33, paymentHash: "0x1234", preimage: "0x5678" },
  { id: 333, amount: 140.12, paymentHash: "0x1234", preimage: "0x5678" },
];

export function Transactions({ h }: { h?: number | string }) {
  const rows = txns.map((element) => (
    <Table.Tr key={element.id}>
      <Table.Td>{element.id}</Table.Td>
      <Table.Td>{element.amount}</Table.Td>
      <Table.Td>{element.paymentHash}</Table.Td>
      <Table.Td>{element.preimage}</Table.Td>
    </Table.Tr>
  ));

  return (
    <ScrollArea w="100%" h={h ?? 200}>
      <Table bg="#1e1e1e" withRowBorders={false} highlightOnHover>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>Tx Id</Table.Th>
            <Table.Th>Amount</Table.Th>
            <Table.Th>Payment Hash</Table.Th>
            <Table.Th>Preimage</Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{rows}</Table.Tbody>
      </Table>
    </ScrollArea>
  );
}
