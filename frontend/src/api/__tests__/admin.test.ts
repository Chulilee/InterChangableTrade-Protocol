import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  banUser,
  unbanUser,
  approveListing,
  removeListing,
  resolveDispute,
  fetchAdminStats,
  fetchAdminUsers,
  buildAdminUsersQuery
} from '../admin';

const apiRequest = vi.fn();

function respondOk(data: unknown) {
  apiRequest.mockResolvedValueOnce({
    ok: true,
    json: () => Promise.resolve(data)
  });
}

function respondError(message: string) {
  apiRequest.mockResolvedValueOnce({
    ok: false,
    json: () => Promise.resolve({ message })
  });
}

describe('buildAdminUsersQuery', () => {
  it('serializes pagination, search and banned filter', () => {
    const qs = buildAdminUsersQuery({ page: 2, limit: 25, search: 'ada', banned: true });
    const params = new URLSearchParams(qs.slice(1));
    expect(params.get('page')).toBe('2');
    expect(params.get('limit')).toBe('25');
    expect(params.get('search')).toBe('ada');
    expect(params.get('banned')).toBe('true');
  });

  it('omits unset fields', () => {
    expect(buildAdminUsersQuery({})).toBe('');
  });
});

describe('admin API client', () => {
  beforeEach(() => {
    apiRequest.mockReset();
  });

  it('fetchAdminStats hits the stats endpoint', async () => {
    respondOk({ totalUsers: 10, activeUsers: 7, pendingListings: 2, openDisputes: 1 });
    const stats = await fetchAdminStats(apiRequest);
    expect(stats.totalUsers).toBe(10);
    expect(apiRequest).toHaveBeenCalledWith(expect.stringContaining('/admin/stats'), expect.anything());
  });

  it('fetchAdminUsers requests the paginated user list', async () => {
    respondOk({ items: [], total: 0, page: 1, limit: 25 });
    const result = await fetchAdminUsers({ page: 1, limit: 25 }, apiRequest);
    expect(result.limit).toBe(25);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/admin/users?page=1&limit=25'),
      expect.anything()
    );
  });

  it('surfaces the server message on a failed privileged call', async () => {
    respondError('Forbidden');
    await expect(fetchAdminStats(apiRequest)).rejects.toThrow('Failed to load admin stats');
  });

  it('banUser posts to the user ban endpoint', async () => {
    respondOk({});
    await banUser('user-9', apiRequest);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/admin/users/user-9/ban'),
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('unbanUser posts to the user unban endpoint', async () => {
    respondOk({});
    await unbanUser('user-9', apiRequest);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/admin/users/user-9/unban'),
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('removeListing deletes the listing', async () => {
    respondOk({});
    await removeListing('listing-3', apiRequest);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/admin/listings/listing-3'),
      expect.objectContaining({ method: 'DELETE' })
    );
  });

  it('approveListing posts the listing approval', async () => {
    respondOk({});
    await approveListing('listing-3', apiRequest);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/admin/listings/listing-3/approve'),
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('resolveDispute posts outcome and required resolution note', async () => {
    respondOk({});
    await resolveDispute('dispute-1', { outcome: 'split', resolutionNote: '50/50 agreement' }, apiRequest);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/admin/disputes/dispute-1/resolve'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ outcome: 'split', resolutionNote: '50/50 agreement' })
      })
    );
  });

  it('moderation action errors include the fallback message', async () => {
    respondError('Listing already reviewed');
    await expect(removeListing('listing-3', apiRequest)).rejects.toThrow('Failed to remove listing');
  });
});
