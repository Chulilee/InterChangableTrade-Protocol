import { describe, it, expect, beforeEach } from 'vitest';
import {
  clearAdminAuditLog,
  getAdminAuditLog,
  recordAdminAction,
  type AdminAuditEntry
} from '../auditLog';

describe('admin audit log', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('starts empty and records entries', () => {
    expect(getAdminAuditLog()).toEqual([]);

    recordAdminAction({
      action: 'user_ban',
      message: 'Banned user u-1',
      targetId: 'u-1',
      succeeded: true
    });

    const log = getAdminAuditLog();
    expect(log).toHaveLength(1);
    expect(log[0]).toMatchObject({
      action: 'user_ban',
      targetId: 'u-1',
      succeeded: true
    });
    expect(log[0].id).toBeTruthy();
    expect(log[0].createdAt).toBeTruthy();
  });

  it('returns the newest first', () => {
    recordAdminAction({
      action: 'user_ban',
      message: 'older',
      targetId: 'u-1',
      succeeded: true,
      createdAt: '2026-01-01T00:00:00.000Z'
    });
    recordAdminAction({
      action: 'listing_removed',
      message: 'newer',
      targetId: 'l-2',
      succeeded: true,
      createdAt: '2026-02-01T00:00:00.000Z'
    });

    const log = getAdminAuditLog();
    expect(log[0].message).toBe('newer');
    expect(log[1].message).toBe('older');
  });

  it('caps the stored trail at 100 entries', () => {
    for (let i = 0; i < 120; i++) {
      recordAdminAction({
        action: 'dispute_resolved',
        message: `action ${i}`,
        targetId: `d-${i}`,
        succeeded: true,
        createdAt: new Date(2026, 0, 1, 0, 0, i).toISOString()
      });
    }
    expect(getAdminAuditLog().length).toBe(100);
  });

  it('clear removes all entries', () => {
    recordAdminAction({
      action: 'user_ban',
      message: 'Banned user u-1',
      targetId: 'u-1',
      succeeded: true
    });
    clearAdminAuditLog();
    expect(getAdminAuditLog()).toEqual([]);
  });

  it('survives a corrupted store', () => {
    localStorage.setItem('admin_audit_log', 'not json');
    expect(getAdminAuditLog()).toEqual([]);
    const entry: AdminAuditEntry = recordAdminAction({
      action: 'listing_approved',
      message: 'approved',
      targetId: 'l-1',
      succeeded: true
    });
    expect(entry.succeeded).toBe(true);
  });
});
