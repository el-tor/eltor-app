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
import { upsertLightningConfig, deleteLightningConfig, refreshWallet } from './walletStore'
import type { LightningConfigRequest } from '../../services/walletApiService'
import { walletApiService } from '../../services/walletApiService'
import { useState, useEffect, useRef } from 'react'
import { IconPlayerPlay, IconPlayerStop } from '@tabler/icons-react';


export function WalletConfigModal({ close }: { close: () => void }) {
  const dispatch = useDispatch()
  const phoenixOperation = useRef(0)
  useEffect(() => () => { phoenixOperation.current += 1 }, [])
  const [deleteConfirmOpened, setDeleteConfirmOpened] = useState(false)
  const [phoenixLoading, setPhoenixLoading] = useState(false)
  const [phoenixRunning, setPhoenixRunning] = useState(false)
  const [statusCheckLoading, setStatusCheckLoading] = useState(true)
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

  // Check Phoenix status when modal opens (only for phoenixd)
  useEffect(() => {
    phoenixOperation.current += 1
    let cancelled = false
    const checkPhoenixStatus = async () => {
      if (clickedWallet !== 'phoenixd') {
        setStatusCheckLoading(false)
        return
      }

      try {
        // Try to detect Phoenix config - this also tells us if it's running
        const configResponse = await walletApiService.detectPhoenixConfig()
        if (cancelled) return
        if (configResponse.success) {
          const isRunning = configResponse.is_running ?? false
          setPhoenixRunning(isRunning)
          
          // Auto-populate form fields if available and not already set
          if (configResponse.url && configResponse.password) {
            if (!form.getValues().url || form.getValues().url === 'http://localhost:9740') {
              form.setFieldValue('url', configResponse.url)
            }
            if (!form.getValues().password) {
              form.setFieldValue('password', configResponse.password)
            }
            if (!existingConfig) {
              form.setFieldValue('is_embedded', true)
            }
          }
        } else {
          setPhoenixRunning(false)
        }
      } catch (error) {
        // If detection fails, Phoenix is probably not running
        console.log('Phoenix not detected (likely not running):', error)
        if (!cancelled) setPhoenixRunning(false)
      } finally {
        if (!cancelled) setStatusCheckLoading(false)
      }
    }

    setStatusCheckLoading(true)
    checkPhoenixStatus()
    return () => { cancelled = true }
  }, [clickedWallet])

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
    const operation = ++phoenixOperation.current
    setPhoenixLoading(true)
    try {
      const response = await walletApiService.startPhoenixDaemon()
      await dispatch(refreshWallet())
      if (operation !== phoenixOperation.current) return
      if (!response.success) throw new Error(response.message)
      setPhoenixRunning(response.is_running ?? true)
      if (response.url && response.password) {
        form.setFieldValue('url', response.url)
        form.setFieldValue('password', response.password)
        form.setFieldValue('is_embedded', true)
      }
      notifications.show({title:'Phoenix Ready',message:existingConfig ? 'Wallet status, balance and transactions refreshed.' : 'Wallet ready. Save the configuration to select it as your wallet.',color:'green'})
    } catch {
      // A slow process remains owned and stoppable even if readiness timed out.
      try {
        const status = await walletApiService.detectPhoenixConfig()
        if (operation === phoenixOperation.current) setPhoenixRunning(status.is_running ?? false)
      } catch { /* No existing process/configuration was detected. */ }
      if (operation === phoenixOperation.current) notifications.show({title:'Phoenix Not Ready',message:'Phoenix has not reached readiness. Check its status and retry.',color:'orange'})
    } finally {
      if (operation === phoenixOperation.current) setPhoenixLoading(false)
    }
  }

  const handlePhoenixStop = async () => {
    const operation = ++phoenixOperation.current
    setPhoenixLoading(true)
    try {
      const response = await walletApiService.stopPhoenixDaemon()
      await dispatch(refreshWallet())
      if (operation !== phoenixOperation.current) return
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
      if (operation === phoenixOperation.current) setPhoenixLoading(false)
    }
  }

  return (
    <div>
      {clickedWallet === 'phoenixd' && (
        <>
          <Text mb="sm">
            {statusCheckLoading 
              ? 'Checking Phoenix status...' 
              : phoenixRunning 
                ? 'Embedded Phoenix Node is running. Configure below or stop to use a remote node.'
                : 'Click Start if you want to use the Embedded Phoenix Node. Or configure below for your remote Phoenix Node'
            }
          </Text>
          <Button.Group>
            <Button 
              radius="md" 
              loading={phoenixLoading || statusCheckLoading}
              disabled={phoenixRunning}
              onClick={handlePhoenixStart}
            >
              <IconPlayerPlay />&nbsp;Start
            </Button>
            <Button 
              variant='light' 
              radius="md" 
              loading={phoenixLoading || statusCheckLoading}
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
