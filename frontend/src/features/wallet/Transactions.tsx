import { ScrollArea, Table, Collapse, ActionIcon, Text, Group, Box, CopyButton, Tooltip, Center } from '@mantine/core'
import React, { useEffect, useState } from 'react'
import { IconChevronDown, IconChevronRight, IconCopy, IconCheck } from '@tabler/icons-react'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { useDispatch, useSelector } from '../../hooks'
import { fetchTransactions } from './walletStore'

// Extend dayjs with relativeTime plugin
dayjs.extend(relativeTime)

export function Transactions({ h }: { h?: number | string }) {
  const { transactions, defaultLightningConfig } = useSelector((state) => state.wallet)
  const dispatch = useDispatch()
  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set())

  const toggleRow = (paymentHash: string) => {
    const newExpandedRows = new Set(expandedRows)
    if (newExpandedRows.has(paymentHash)) {
      newExpandedRows.delete(paymentHash)
    } else {
      newExpandedRows.add(paymentHash)
    }
    setExpandedRows(newExpandedRows)
  }

  const formatDate = (timestamp: number) => {
    const adjustedTimestamp = timestamp < 10000000000 ? timestamp * 1000 : timestamp
    const date = dayjs(adjustedTimestamp)
    const now = dayjs()
    
    if (date.isSame(now, 'day')) {
      return `Today ${date.format('h:mm A')}`
    } else if (date.isSame(now.subtract(1, 'day'), 'day')) {
      return `Yesterday ${date.format('h:mm A')}`
    } else if (date.isAfter(now.subtract(7, 'day'))) {
      return `${date.fromNow()} ${date.format('h:mm A')}`
    } else {
      return date.format('MMM D, YYYY')
    }
  }

  useEffect(() => {
    console.log('Transactions component mounted')
    // Only fetch transactions if there's a default lightning config
    if (defaultLightningConfig) {
      dispatch(fetchTransactions(''))
    }
  }, [defaultLightningConfig])

  return (
    <ScrollArea
      w="100%"
      h={h ?? 200}
      p="sm"
      style={{ borderRadius: '6px' }}
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
            <Table.Th></Table.Th>
            <Table.Th>Amount</Table.Th>
            <Table.Th>Settled At</Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>
          {!defaultLightningConfig ? (
            <Table.Tr>
              <Table.Td colSpan={3}>
                <Center p="xl">
                  <Text c="dimmed" size="sm">
                    No default wallet configured
                  </Text>
                </Center>
              </Table.Td>
            </Table.Tr>
          ) : transactions?.length > 0 ? (
            transactions
              .slice()
              .sort((a, b) => (b.settled_at || 0) - (a.settled_at || 0))
              .map((element) => {
                const isExpanded = expandedRows.has(element.payment_hash)
                return (
                  <React.Fragment key={element.payment_hash}>
                    <Table.Tr 
                      style={{ cursor: 'pointer' }}
                      onClick={() => toggleRow(element.payment_hash)}
                    >
                      <Table.Td onClick={(e) => e.stopPropagation()}>
                        <ActionIcon 
                          variant="subtle" 
                          size="sm"
                          onClick={() => toggleRow(element.payment_hash)}
                        >
                          {isExpanded ? <IconChevronDown size={16} /> : <IconChevronRight size={16} />}
                        </ActionIcon>
                      </Table.Td>
                      <Table.Td style={{ fontFamily: 'monospace', fontWeight: 'bold' }}>
                        {Math.round(Number(element.amount_msats) / 1000)?.toLocaleString()} sats
                      </Table.Td>
                      <Table.Td>
                        {element.settled_at ? formatDate(element.settled_at) : ''}
                      </Table.Td>
                    </Table.Tr>
                    {isExpanded && (
                      <Table.Tr>
                        <Table.Td colSpan={3}>
                          <Box p="md" bg="dark.7" style={{ borderRadius: '8px', margin: '8px 0' }}>
                            <Group align="flex-start" gap="xl">
                              <Box>
                                <Text size="sm" fw={500} mb={4}>Payment Hash</Text>
                                <Group gap={4}>
                                  <Text size="xs" style={{ fontFamily: 'monospace', wordBreak: 'break-all' }} c="dimmed">
                                    {element.payment_hash}
                                  </Text>
                                  <CopyButton value={element.payment_hash}>
                                    {({ copied, copy }) => (
                                      <Tooltip label={copied ? 'Copied' : 'Copy'}>
                                        <ActionIcon size="sm" variant="subtle" onClick={copy}>
                                          {copied ? <IconCheck size={14} /> : <IconCopy size={14} />}
                                        </ActionIcon>
                                      </Tooltip>
                                    )}
                                  </CopyButton>
                                </Group>
                              </Box>
                              
                              {element.preimage && (
                                <Box>
                                  <Text size="sm" fw={500} mb={4}>Preimage</Text>
                                  <Group gap={4}>
                                    <Text size="xs" style={{ fontFamily: 'monospace', wordBreak: 'break-all' }} c="dimmed">
                                      {element.preimage}
                                    </Text>
                                    <CopyButton value={element.preimage}>
                                      {({ copied, copy }) => (
                                        <Tooltip label={copied ? 'Copied' : 'Copy'}>
                                          <ActionIcon size="sm" variant="subtle" onClick={copy}>
                                            {copied ? <IconCheck size={14} /> : <IconCopy size={14} />}
                                          </ActionIcon>
                                        </Tooltip>
                                      )}
                                    </CopyButton>
                                  </Group>
                                </Box>
                              )}
                              
                              {element.payer_note && (
                                <Box>
                                  <Text size="sm" fw={500} mb={4}>Note</Text>
                                  <Text size="sm" c="dimmed">{element.payer_note}</Text>
                                </Box>
                              )}
                            </Group>
                          </Box>
                        </Table.Td>
                      </Table.Tr>
                    )}
                  </React.Fragment>
                )
              })
          ) : (
            <Table.Tr>
              <Table.Td colSpan={3}>
                <Center p="xl">
                  <Text c="dimmed" size="sm">
                    No transactions found
                  </Text>
                </Center>
              </Table.Td>
            </Table.Tr>
          )}
        </Table.Tbody>
      </Table>
    </ScrollArea>
  )
}
