import {
  Group,
  TextInput,
  Checkbox,
  Button,
  Anchor,
  Text,
  PasswordInput,
  Modal,
  Alert,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconClock } from '@tabler/icons-react'
import { useSelector, useDispatch } from '../../hooks'
import { useForm } from '@mantine/form'
import { upsertLightningConfig, deleteLightningConfig } from './walletStore'
import type { LightningConfigRequest } from '../../services/walletApiService'
import { walletApiService } from '../../services/walletApiService'
import { useState } from 'react'
import { IconPlayerPlay, IconPlayerStop } from '@tabler/icons-react';


export function WalletConfigModal({ close }: { close: () => void }) {
  const dispatch = useDispatch()
  const [deleteConfirmOpened, setDeleteConfirmOpened] = useState(false)
  const [phoenixLoading, setPhoenixLoading] = useState(false)
  const [phoenixRunning, setPhoenixRunning] = useState(false)
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

  // Check if this wallet type is coming soon
  const isComingSoon = clickedWallet === 'lnd' || clickedWallet === 'strike'
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
      is_embedded: existingConfig?.is_embedded || false,
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
      is_embedded: val.is_embedded,
    }

    try {
      await dispatch(upsertLightningConfig(configData)).unwrap()

      // Success notification
      notifications.show({
        title: 'Configuration Saved',
        message: `Successfully ${
          existingConfig ? 'updated' : 'created'
        } ${clickedWallet} configuration${
          configData.set_as_default
            ? '. Wallet data will refresh automatically.'
            : ''
        }`,
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
    setDeleteConfirmOpened(true)
  }

  const confirmDelete = async () => {
    if (!existingConfig) return

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
        message: `Successfully deleted ${clickedWallet} configuration. Wallet data will refresh automatically.`,
        color: 'green',
      })
      setDeleteConfirmOpened(false)
      close()
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

  const handlePhoenixStart = async () => {
    setPhoenixLoading(true)
    try {
      const response = await walletApiService.startPhoenixDaemon()
      
      if (response.success) {
        setPhoenixRunning(true)
        
        // Auto-populate form fields with Phoenix config if available
        if (response.url && response.password) {
          form.setFieldValue('url', response.url)
          form.setFieldValue('password', response.password)
          form.setFieldValue('setAsDefault', true) // Default to making it the default wallet
          form.setFieldValue('is_embedded', true) // Mark as embedded Phoenix instance
          
          notifications.show({
            title: 'Phoenix Started',
            message: `${response.message}. Form fields have been auto-populated with the Phoenix configuration.`,
            color: 'green',
            autoClose: 6000,
          })
        } else {
          notifications.show({
            title: 'Phoenix Started',
            message: `${response.message}. Please configure the URL and password manually.`,
            color: 'yellow',
          })
        }
      } else {
        notifications.show({
          title: 'Phoenix Start Failed',
          message: response.message,
          color: 'red',
        })
      }
    } catch (error) {
      console.error('Failed to start Phoenix daemon:', error)
      
      // Check if error is about Phoenix already running
      const errorMessage = String(error)
      if (errorMessage.includes('already running') || errorMessage.includes('port 9740')) {
        try {
          // Attempt to detect existing Phoenix configuration
          console.log('Attempting to detect existing Phoenix configuration...')
          const configResponse = await walletApiService.detectPhoenixConfig()
          
          if (configResponse.success && configResponse.url && configResponse.password) {
            // Auto-populate form with detected config
            form.setFieldValue('url', configResponse.url)
            form.setFieldValue('password', configResponse.password)
            form.setFieldValue('setAsDefault', true)
            form.setFieldValue('is_embedded', true) // Mark as embedded since it's running locally
            
            notifications.show({
              title: 'Phoenix Already Running',
              message: 'Phoenix is already running. Auto-detected configuration and populated the form fields.',
              color: 'blue',
              autoClose: 8000,
            })
          } else {
            notifications.show({
              title: 'Phoenix Already Running',
              message: 'Phoenix appears to be running but configuration could not be auto-detected. Please enter the URL and password manually.',
              color: 'yellow',
            })
          }
        } catch (detectError) {
          console.error('Failed to detect Phoenix config:', detectError)
          notifications.show({
            title: 'Phoenix Already Running',
            message: 'Phoenix appears to be running but configuration could not be auto-detected. Please enter the URL and password manually.',
            color: 'yellow',
          })
        }
      } else {
        // Other error types
        notifications.show({
          title: 'Phoenix Start Failed',
          message: `Failed to start Phoenix daemon: ${error}`,
          color: 'red',
        })
      }
    } finally {
      setPhoenixLoading(false)
    }
  }

  const handlePhoenixStop = async () => {
    setPhoenixLoading(true)
    try {
      const response = await walletApiService.stopPhoenixDaemon()
      
      if (response.success) {
        setPhoenixRunning(false)
        notifications.show({
          title: 'Phoenix Stopped',
          message: response.message,
          color: 'blue',
        })
      } else {
        notifications.show({
          title: 'Phoenix Stop Failed',
          message: response.message,
          color: 'red',
        })
      }
    } catch (error) {
      console.error('Failed to stop Phoenix daemon:', error)
      notifications.show({
        title: 'Phoenix Stop Failed',
        message: `Failed to stop Phoenix daemon: ${error}`,
        color: 'red',
      })
    } finally {
      setPhoenixLoading(false)
    }
  }

  return (
    <div>
      {clickedWallet === 'phoenixd' && (
        <>
          <Text mb="sm">Click Start if you want to use the Embedded Phoenix Node. Or configure below for your remote Phoenix Node</Text>
          <Button.Group>
            <Button 
              radius="md" 
              loading={phoenixLoading}
              disabled={phoenixRunning}
              onClick={handlePhoenixStart}
            >
              <IconPlayerPlay />&nbsp;Start
            </Button>
            <Button 
              variant='light' 
              radius="md" 
              loading={phoenixLoading}
              disabled={!phoenixRunning}
              onClick={handlePhoenixStop}
            >
              <IconPlayerStop />&nbsp;Stop
            </Button>
          </Button.Group>
        </>
      )}

      {/* Coming Soon Banner */}
      {isComingSoon && (
        <Alert
          variant="light"
          color="blue"
          title={`${clickedWallet.toUpperCase()} Integration Coming Soon`}
          icon={<IconClock size={16} />}
          mb="md"
        >
          <Text size="sm">
            We're working on {clickedWallet.toUpperCase()} support. This feature
            will be available in a future update.
          </Text>
        </Alert>
      )}

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
        disabled={isComingSoon}
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
        disabled={isComingSoon}
        {...form.getInputProps('password')}
      />

      <Checkbox
        mt="xl"
        label="Default Wallet - Click here to make this the default wallet that will be used to pay for bandwidth. If you are running a relay this wallet will be used to create a BOLT12 offer that you will receive payments for sharing your bandwidth."
        disabled={isComingSoon}
        {...form.getInputProps('setAsDefault', { type: 'checkbox' })}
      />

      <Group justify="center" mt="xl">
        <Button
          w="20rem"
          mb="1rem"
          loading={lightningConfigsLoading}
          disabled={isComingSoon}
          onClick={handleSave}
        >
          {existingConfig ? 'Update' : 'Save'}
        </Button>
      </Group>

      {existingConfig && !isComingSoon && (
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

      {/* Delete Confirmation Modal */}
      <Modal
        opened={deleteConfirmOpened}
        onClose={() => setDeleteConfirmOpened(false)}
        title="Confirm Deletion"
        centered
      >
        <Text mb="md">
          Are you sure you want to delete the {clickedWallet} configuration?
        </Text>
        <Group justify="flex-end">
          <Button
            variant="outline"
            onClick={() => setDeleteConfirmOpened(false)}
          >
            Cancel
          </Button>
          <Button
            color="red"
            onClick={confirmDelete}
            loading={lightningConfigsLoading}
          >
            Delete
          </Button>
        </Group>
      </Modal>
    </div>
  )
}
