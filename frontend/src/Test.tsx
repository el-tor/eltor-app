import React from 'react'
import { Button, Title } from '@mantine/core'
import { MantineWrapper } from './MantineWrapper'

export function Test() {
  return (
    <MantineWrapper>
      <Title>Test Component</Title>
      <Button>Test Button from Shared Library</Button>
    </MantineWrapper>
  )
}
