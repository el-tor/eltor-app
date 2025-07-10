import {
  Group,
  TextInput,
  Checkbox,
  Button,
  Anchor,
  Text,
  PasswordInput,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useSelector, useDispatch } from '../../hooks'
import { useForm } from '@mantine/form'
import {
  upsertLightningConfig,
  deleteLightningConfig,
  fetchLightningConfigs,
} from './walletStore'
import type { LightningConfigRequest } from '../../services/walletApiService'
import { useEffect } from 'react'

export function WalletConfigModal({ close }: { close: () => void }) {
  const dispatch = useDispatch()
  const {
    clickedWallet,
    lightningConfigs,
    lightningConfigsLoading,
    lightningConfigsError,
  } = useSelector((state) => state.wallet)

  // Find existing config for the clicked wallet
  const existingConfig = lightningConfigs?.find(
    (config) => config.node_type === clickedWallet,
  )
  const form = useForm({
    mode: 'uncontrolled',
    initialValues: {
      url:
        existingConfig?.url ||
        (() => {
          switch (clickedWallet) {
            case 'phoenixd':
              return 'http://localhost:9740'
            case 'lnd':
              return 'https://localhost:8080'
            case 'cln':
              return 'https://localhost:3001'
            case 'strike':
              return 'https://api.strike.me'
            default:
              return ''
          }
        })(),
      password: existingConfig?.password || '',
      setAsDefault: existingConfig?.is_default || false,
    },
    validate: {
      url: (value) =>
        value.length < 5 ? 'URL must be at least 5 characters long' : null,
      password: (value) => {
        // For new configs or when password is provided, enforce minimum length
        return value.length < 1
          ? 'Password must be at least 1 character long'
          : null
      },
    },
  })

  // Helper function to mask password for display when editing existing config
  const maskCredential = (credential: string, visibleChars: number = 6) => {
    // Handle null, undefined, or empty strings
    if (
      !credential ||
      typeof credential !== 'string' ||
      credential.length === 0
    ) {
      return '***'
    }

    // If credential is too short to mask meaningfully, just show asterisks
    if (credential.length <= visibleChars * 2) {
      return '*'.repeat(Math.min(credential.length, 10))
    }

    const start = credential.substring(0, visibleChars)
    const end = credential.substring(credential.length - visibleChars)
    const middle = '*'.repeat(
      Math.min(credential.length - visibleChars * 2, 20),
    )
    return `${start}${middle}${end}`
  }

  const handleSave = async () => {
    const validation = form.validate()
    if (validation.hasErrors) {
      console.error('Validation errors:', validation.errors)
      return
    }

    const val = form.getValues()

    const configData: LightningConfigRequest = {
      node_type: clickedWallet as any,
      url: val.url,
      password: val.password,
      set_as_default: val.setAsDefault,
    }

    try {
      await dispatch(upsertLightningConfig(configData)).unwrap()
      // Success notification
      notifications.show({
        title: 'Configuration Saved',
        message: `Successfully ${
          existingConfig ? 'updated' : 'created'
        } ${clickedWallet} configuration`,
        color: 'green',
      })
      close()
    } catch (error) {
      console.error('Failed to save config:', error)
      // Error notification
      notifications.show({
        title: 'Save Failed',
        message: `Failed to save ${clickedWallet} configuration: ${error}`,
        color: 'red',
      })
    }
  }

  const handleDelete = async () => {
    if (!existingConfig) return

    if (
      window.confirm(
        `Are you sure you want to delete the ${clickedWallet} configuration?`,
      )
    ) {
      try {
        await dispatch(
          deleteLightningConfig({
            node_type: clickedWallet as any,
            url: existingConfig.url,
          }),
        ).unwrap()
        // Success notification
        notifications.show({
          title: 'Configuration Deleted',
          message: `Successfully deleted ${clickedWallet} configuration`,
          color: 'green',
        })
      } catch (error) {
        console.error('Failed to delete config:', error)
        // Error notification
        notifications.show({
          title: 'Delete Failed',
          message: `Failed to delete ${clickedWallet} configuration: ${error}`,
          color: 'red',
        })
      }
    }
  }

  return (
    <div>
      {lightningConfigsError && (
        <Text c="red" size="sm" mb="md">
          Error: {lightningConfigsError}
        </Text>
      )}

      {existingConfig && existingConfig.password && (
        <Text size="sm" c="dimmed" mb="md">
          Editing existing configuration. Current password:{' '}
          {maskCredential(existingConfig.password)}
        </Text>
      )}

      <TextInput
        mt="md"
        label={(() => {
          switch (clickedWallet) {
            case 'phoenixd':
              return 'REST Url'
            case 'lnd':
              return 'REST Url'
            case 'cln':
              return 'CLN REST Url'
            case 'strike':
              return 'Strike REST Url'
            default:
              return 'Url'
          }
        })()}
        {...form.getInputProps('url')}
      />
      <PasswordInput
        mt="md"
        label={(() => {
          switch (clickedWallet) {
            case 'phoenixd':
              return 'HTTP Password'
            case 'lnd':
              return 'Macaroon'
            case 'cln':
              return 'Rune'
            case 'strike':
              return 'Strike Api Key'
            default:
              return 'Api Key'
          }
        })()}
        placeholder={
          existingConfig ? 'Leave empty to keep current password' : undefined
        }
        {...form.getInputProps('password')}
      />

      <Checkbox
        mt="xl"
        label="Default Wallet - Click here to make this the default wallet that will be used to pay for bandwidth. If you are running a relay this wallet will be used to create a BOLT12 offer that you will receive payments for sharing your bandwidth."
        {...form.getInputProps('setAsDefault', { type: 'checkbox' })}
      />

      <Group justify="center" mt="xl">
        <Button
          w="20rem"
          mb="1rem"
          loading={lightningConfigsLoading}
          onClick={handleSave}
        >
          {existingConfig ? 'Update' : 'Save'}
        </Button>
      </Group>

      {existingConfig && (
        <Group justify="center">
          <Text
            onClick={handleDelete}
            style={{ cursor: 'pointer', fontSize: '.85rem' }}
            c="red"
          >
            Delete Config
          </Text>
        </Group>
      )}
    </div>
  )
}
