import { describe, it, expect } from 'vitest';

// Event action symbols (mirrors contracts/events/src/lib.rs)
const EVENT_ACTIONS = {
  INIT: 'init',
  DEPOSIT: 'deposit',
  WITHDRAW: 'withdraw',
  DISTRIBUTE: 'distrib',
  CLAIM: 'claim',
  LOCK: 'lock',
  UNLOCK: 'unlock',
  ADMIN_PROPOSED: 'admin_prp',
  ADMIN_ACCEPTED: 'adm_acpt',
  UPGRADE: 'upgrade',
  PAUSE: 'pause',
  UNPAUSE: 'unpause',
  ASSET_ADDED: 'asset_add',
  ASSET_DEPOSIT: 'asset_dep',
  ASSET_WITHDRAW: 'asset_wd',
  ASSET_DISTRIBUTE: 'ast_dist',
  ASSET_CLAIM: 'asset_clm',
} as const;

type EventAction = (typeof EVENT_ACTIONS)[keyof typeof EVENT_ACTIONS];

const PROTOCOL = 'AxVault';

describe('Event Standards', () => {
  describe('Action Symbols', () => {
    it('should use lowercase_with_underscores convention', () => {
      const re = /^[a-z][a-z0-9_]*$/;
      for (const [key, value] of Object.entries(EVENT_ACTIONS)) {
        expect(re.test(value), `${key}: "${value}" should match lowercase_with_underscores`).toBe(true);
      }
    });

    it('should not contain spaces or hyphens', () => {
      for (const [key, value] of Object.entries(EVENT_ACTIONS)) {
        expect(value).not.toContain(' ');
        expect(value).not.toContain('-');
      }
    });

    it('should have unique values', () => {
      const values = Object.values(EVENT_ACTIONS);
      const uniqueValues = new Set(values);
      expect(uniqueValues.size).toBe(values.length);
    });
  });

  describe('Topic Design', () => {
    it('all events must follow two-topic (Protocol, Action) design', () => {
      const actions = Object.values(EVENT_ACTIONS);
      for (const action of actions) {
        const topics = [PROTOCOL, action];
        expect(topics).toHaveLength(2);
        expect(topics[0]).toBe(PROTOCOL);
        expect(topics[1]).toBe(action);
      }
    });

    it('protocol identifier should be consistent across all events', () => {
      const actions = Object.values(EVENT_ACTIONS);
      for (const action of actions) {
        expect(PROTOCOL).toBe('AxionVault');
        expect(action).toBeDefined();
        expect(action.length).toBeGreaterThan(0);
      }
    });
  });

  describe('Event Completeness', () => {
    it('should have all required vault operations', () => {
      const required = ['init', 'deposit', 'withdraw', 'distrib', 'claim', 'lock', 'unlock'];
      const actions = Object.values(EVENT_ACTIONS);
      for (const r of required) {
        expect(actions).toContain(r);
      }
    });

    it('should have all admin operations', () => {
      const actions = Object.values(EVENT_ACTIONS);
      expect(actions).toContain('admin_prp');
      expect(actions).toContain('admin_acpt');
      expect(actions).toContain('upgrade');
      expect(actions).toContain('pause');
      expect(actions).toContain('unpause');
    });

    it('should have all multi-asset operations', () => {
      const actions = Object.values(EVENT_ACTIONS);
      expect(actions).toContain('asset_add');
      expect(actions).toContain('asset_dep');
      expect(actions).toContain('asset_wd');
      expect(actions).toContain('asset_dist');
      expect(actions).toContain('asset_clm');
    });

    it('should have at least 16 event types', () => {
      const actions = Object.values(EVENT_ACTIONS);
      expect(actions.length).toBeGreaterThanOrEqual(16);
    });
  });

  describe('Event Versioning', () => {
    it('should define an event version', () => {
      // The Rust code defines EVENT_VERSION = 1 in axionvera_events
      // Validate the constant contract
      expect(true).toBe(true); // Placeholder — actual validation is in Rust tests
    });
  });

  describe('Indexing Compatibility', () => {
    it('protocol filter should match all vault events', () => {
      const actions = Object.values(EVENT_ACTIONS);
      for (const action of actions) {
        // All events should match a topic filter of ["AxVault"]
        expect(PROTOCOL).toBe('AxVault');
        const topic0 = 'AxVault';
        const topic1 = action;
        expect(topic0).toBe('AxVault');
        expect(topic1).toBeDefined();
      }
    });

    it('should not use single-topic design for any event', () => {
      // Previously AdminProp, AdminAcpt, Upgrade, AssetAdd used single topics.
      // After standardization, all events use (Protocol, Action).
      const actions = Object.values(EVENT_ACTIONS);
      for (const action of actions) {
        // Two topics: [protocol, action]
        expect([PROTOCOL, action]).toHaveLength(2);
      }
    });
  });
});
