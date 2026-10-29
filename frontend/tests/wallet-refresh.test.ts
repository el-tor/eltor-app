import test from 'node:test'
import assert from 'node:assert/strict'
import { configureStore } from '@reduxjs/toolkit'
import { refreshWallet, walletReducer } from '../src/features/wallet/walletStore'
import { walletApiService } from '../src/services/walletApiService'

test('startup not ready then ready reloads balance and transactions without navigation; stale failure cannot undo ready', async () => {
  const calls: string[] = []
  let ready = false
  walletApiService.listLightningConfigs = async () => [{node_type:'phoenixd',url:'http://127.0.0.1:9740',password:'synthetic',password_type:'password',is_default:true}]
  walletApiService.getNodeInfo = async () => {
    calls.push('info')
    if (!ready) throw new Error('not ready')
    return {node_type:'phoenixd',send_balance_msat:45000,receive_balance_msat:12000} as any
  }
  walletApiService.getTransactions = async () => { calls.push('transactions'); return [{payment_hash:'synthetic',created_at:1,amount_msats:3000}] }
  walletApiService.getBolt12Offer = async () => ({payment_request:'synthetic-offer'} as any)
  const store = configureStore({reducer:{wallet:walletReducer}})
  await store.dispatch(refreshWallet())
  assert.equal(store.getState().wallet.walletReady,false)
  ready = true // start/ready event triggers the same snapshot used at app boot.
  await store.dispatch(refreshWallet())
  assert.equal(store.getState().wallet.walletReady,true)
  assert.equal(store.getState().wallet.send,45)
  assert.equal(store.getState().wallet.transactions.length,1)
  assert.equal(store.getState().wallet.bolt12Offer,'synthetic-offer')
  assert.ok(calls.includes('transactions'))
  let rejectOld!: (error: Error) => void
  walletApiService.getNodeInfo = () => new Promise((_,reject) => { rejectOld=reject })
  const old = store.dispatch(refreshWallet())
  while (!rejectOld) await Promise.resolve()
  walletApiService.getNodeInfo = async () => ({node_type:'phoenixd',send_balance_msat:99000,receive_balance_msat:0} as any)
  await store.dispatch(refreshWallet())
  rejectOld(new Error('late startup response'))
  await old
  assert.equal(store.getState().wallet.send,99)
  assert.equal(store.getState().wallet.walletReady,true)
  assert.equal(store.getState().wallet.transactions.length,1)
})
