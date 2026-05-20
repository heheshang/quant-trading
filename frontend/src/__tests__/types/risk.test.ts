import { describe, it, expect } from 'vitest'
import type {
  RiskRules,
  UpdateRiskRulesRequest,
  RiskLog,
  EmergencyCloseResult,
  EmergencyCloseOrder,
  EmergencyCloseResponse,
  PauseResponse,
  ConnectionStatus,
} from '@/types/risk'

describe('risk types', () => {
  describe('RiskRules', () => {
    it('accepts valid fixed stop-loss rules', () => {
      const rules: RiskRules = {
        daily_loss_limit: '1000.00',
        daily_loss_auto_close: true,
        single_trade_loss_ratio: '0.0500',
        max_drawdown_ratio: '0.2000',
        drawdown_auto_close: false,
        stop_loss_type: 'fixed',
        atr_period: null,
        atr_multiplier: null,
        is_active: true,
      }
      expect(rules.stop_loss_type).toBe('fixed')
      expect(rules.atr_period).toBeNull()
      expect(rules.is_active).toBe(true)
    })

    it('accepts valid ATR stop-loss rules', () => {
      const rules: RiskRules = {
        daily_loss_limit: '0',
        daily_loss_auto_close: false,
        single_trade_loss_ratio: '0.1000',
        max_drawdown_ratio: '0.3000',
        drawdown_auto_close: true,
        stop_loss_type: 'atr',
        atr_period: 14,
        atr_multiplier: '2.00',
        is_active: false,
      }
      expect(rules.stop_loss_type).toBe('atr')
      expect(rules.atr_period).toBe(14)
      expect(rules.atr_multiplier).toBe('2.00')
    })

    it('accepts zero-limit rules (disabled)', () => {
      const rules: RiskRules = {
        daily_loss_limit: '0',
        daily_loss_auto_close: false,
        single_trade_loss_ratio: '0',
        max_drawdown_ratio: '0',
        drawdown_auto_close: false,
        stop_loss_type: 'fixed',
        atr_period: null,
        atr_multiplier: null,
        is_active: false,
      }
      expect(rules.daily_loss_limit).toBe('0')
      expect(rules.is_active).toBe(false)
    })

    it('all ratio fields are strings', () => {
      const rules: RiskRules = {
        daily_loss_limit: '500.00',
        daily_loss_auto_close: true,
        single_trade_loss_ratio: '0.0300',
        max_drawdown_ratio: '0.1500',
        drawdown_auto_close: true,
        stop_loss_type: 'atr',
        atr_period: 20,
        atr_multiplier: '3.00',
        is_active: true,
      }
      expect(typeof rules.single_trade_loss_ratio).toBe('string')
      expect(typeof rules.max_drawdown_ratio).toBe('string')
    })
  })

  describe('UpdateRiskRulesRequest', () => {
    it('accepts full update payload', () => {
      const payload: UpdateRiskRulesRequest = {
        daily_loss_limit: '2000.00',
        daily_loss_auto_close: true,
        single_trade_loss_ratio: '0.0300',
        max_drawdown_ratio: '0.1500',
        drawdown_auto_close: true,
        stop_loss_type: 'atr',
        atr_period: 20,
        atr_multiplier: '3.00',
        is_active: true,
      }
      expect(payload.daily_loss_limit).toBe('2000.00')
      expect(payload.stop_loss_type).toBe('atr')
    })

    it('accepts partial payload', () => {
      const payload: UpdateRiskRulesRequest = {
        daily_loss_limit: '500.00',
        is_active: false,
      }
      expect(payload.daily_loss_limit).toBe('500.00')
      expect(payload.is_active).toBe(false)
    })

    it('accepts null ATR fields for fixed stop loss', () => {
      const payload: UpdateRiskRulesRequest = {
        stop_loss_type: 'fixed',
        atr_period: null,
        atr_multiplier: null,
      }
      expect(payload.stop_loss_type).toBe('fixed')
      expect(payload.atr_period).toBeNull()
    })
  })

  describe('RiskLog', () => {
    it('accepts log with all fields', () => {
      const log: RiskLog = {
        id: '1',
        user_id: '1',
        triggered_rule: 'daily_loss_limit',
        rule_type: 'daily',
        action: 'close_position',
        severity: 'critical',
        details: 'Daily loss exceeded: equity 980U < limit 1000U',
        equity_snapshot: '980.00',
        threshold_snapshot: '1000.00',
        created_at: '2026-05-15T10:00:00Z',
      }
      expect(log.severity).toBe('critical')
      expect(log.triggered_rule).toBe('daily_loss_limit')
      expect(log.equity_snapshot).toBe('980.00')
    })

    it('accepts all severity levels', () => {
      const severities: RiskLog['severity'][] = ['low', 'medium', 'high', 'critical']
      severities.forEach((s) => {
        const log: RiskLog = {
          id: '1',
          user_id: '1',
          triggered_rule: 'single_trade_loss_ratio',
          rule_type: 'per_trade',
          action: 'reject_order',
          severity: s,
          details: 'Test',
          equity_snapshot: '1000',
          threshold_snapshot: '1000',
          created_at: '2026-05-15T10:00:00Z',
        }
        expect(log.severity).toBe(s)
      })
    })
  })

  describe('EmergencyCloseResult', () => {
    it('has correct shape', () => {
      const result: EmergencyCloseResult = {
        closed_positions: 2,
        total_pnl: '-150.50',
        closed_orders: [
          { order_id: '1', symbol: 'BTC/USDT', side: 'sell', executed_qty: '0.5', pnl: '-50.00' },
          { order_id: '2', symbol: 'ETH/USDT', side: 'sell', executed_qty: '2.0', pnl: '-100.50' },
        ],
      }
      expect(result.closed_positions).toBe(2)
      expect(result.closed_orders).toHaveLength(2)
      expect(result.closed_orders[0].pnl).toBe('-50.00')
    })
  })

  describe('EmergencyCloseOrder', () => {
    it('has correct shape', () => {
      const order: EmergencyCloseOrder = {
        order_id: '123',
        symbol: 'BTC/USDT',
        side: 'sell',
        executed_qty: '0.5000',
        pnl: '-75.25',
      }
      expect(order.symbol).toBe('BTC/USDT')
      expect(order.side).toBe('sell')
      expect(order.pnl).toBe('-75.25')
    })
  })

  describe('EmergencyCloseResponse', () => {
    it('has correct shape', () => {
      const resp: EmergencyCloseResponse = {
        success: true,
        message: 'All positions closed',
        closed_positions: 1,
        total_pnl: '-75.25',
        details: [
          { order_id: '1', symbol: 'BTC/USDT', side: 'sell', executed_qty: '0.5', pnl: '-75.25' },
        ],
      }
      expect(resp.success).toBe(true)
      expect(resp.message).toBe('All positions closed')
      expect(resp.details).toHaveLength(1)
    })

    it('accepts failed response', () => {
      const resp: EmergencyCloseResponse = {
        success: false,
        message: 'Partial close: 1 position failed',
        closed_positions: 0,
        total_pnl: '0',
        details: [],
      }
      expect(resp.success).toBe(false)
    })
  })

  describe('PauseResponse', () => {
    it('has correct shape', () => {
      const resp: PauseResponse = {
        paused: true,
        reason: 'Manual pause by admin',
      }
      expect(resp.paused).toBe(true)
      expect(resp.reason).toBe('Manual pause by admin')
    })

    it('accepts auto-pause reason', () => {
      const resp: PauseResponse = {
        paused: true,
        reason: 'Auto-pause: disconnected for 90s',
      }
      expect(resp.paused).toBe(true)
      expect(resp.reason).toContain('disconnected')
    })
  })

  describe('ConnectionStatus', () => {
    it('accepts connected status', () => {
      const status: ConnectionStatus = {
        exchange_connected: true,
        disconnect_elapsed_secs: 0,
        strategy_paused: false,
      }
      expect(status.exchange_connected).toBe(true)
      expect(status.strategy_paused).toBe(false)
    })

    it('accepts disconnected status with auto-pause', () => {
      const status: ConnectionStatus = {
        exchange_connected: false,
        disconnect_elapsed_secs: 90,
        strategy_paused: true,
      }
      expect(status.exchange_connected).toBe(false)
      expect(status.disconnect_elapsed_secs).toBe(90)
      expect(status.strategy_paused).toBe(true)
    })

    it('accepts disconnected without auto-pause (threshold not reached)', () => {
      const status: ConnectionStatus = {
        exchange_connected: false,
        disconnect_elapsed_secs: 15,
        strategy_paused: false,
      }
      expect(status.strategy_paused).toBe(false)
      expect(status.disconnect_elapsed_secs).toBe(15)
    })
  })
})
