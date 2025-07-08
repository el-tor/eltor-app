import { Group, TextInput, Checkbox, Button } from '@mantine/core'
import { useSelector } from '../../hooks'
import { useForm } from '@mantine/form'

export function WalletConfigModal() {
  const { clickedWallet } = useSelector((state) => state.wallet)
  const form = useForm({
    mode: 'uncontrolled',
    initialValues: {
      url: (() => {
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
      password: (() => {
        switch (clickedWallet) {
          case 'phoenixd':
            return 'your password'
          case 'lnd':
            return 'your macaroon'
          case 'cln':
            return 'your rune'
          case 'strike':
            return 'your api key'
          default:
            return ''
        }
      })(),
    },
    validate: {
      url: (value) =>
        value.length < 5 ? 'URL must be at least 5 characters long' : null,
      password: (value) =>
        value.length < 4 ? 'Password must be at least 4 characters long' : null,
    },
  })

  return (
    <div>
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
      <TextInput
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
        {...form.getInputProps('password')}
      />

      <Checkbox mt="xl" defaultChecked label="Default Wallet - Click here to make this the default wallet that will be used to pay for bandwidth. If you are running a relay this wallet will be used to create a BOLT12 offer that you will receive payments for sharing your bandwidth." />

      <Group justify="center" mt="xl">
        <Button
          w="20rem"
          mb="1rem"
          onClick={() => {
            const validation = form.validate()
            if (validation.hasErrors) {
              console.error('Validation errors:', validation.errors)
            } else {
              console.log('Form values:', form.values)
              // Here you would typically dispatch an action to save the wallet config
              // For example: dispatch(saveWalletConfig(form.values))
            }
          }}
        >
          Save
        </Button>
      </Group>
    </div>
  )
}
