// Admin dashboard (issue #26): key counts, user management with ban/unban,
// the listing moderation queue, and combined (server + session-local) activity.
//
// Route protection: /admin is wrapped in <ProtectedRoute requiredRoles={['admin']}>
// and the backend re-checks the role on every call, so non-admins cannot
// access these tools from either side. Every moderation action records an
// audit entry (see utils/auditLog.ts) shown in the activity feed.

import { createResource, createSignal, For, Show, Switch, Match } from 'solid-js';
import {
  fetchActivityFeed,
  fetchAdminStats,
  fetchAdminUsers,
  fetchModerationQueue,
  approveListing,
  banUser,
  removeListing,
  resolveDispute,
  unbanUser
} from '../../api/admin';
import { useApi } from '../../hooks/useApi';
import { useAppNotifications } from '../notifications/AppNotifications';
import {
  getAdminAuditLog,
  clearAdminAuditLog,
  recordAdminAction,
  type AdminAuditEntry
} from '../../utils/auditLog';
import type {
  ActivityEntry,
  AdminDashboardStats,
  AdminUserSummary,
  ModerationItem,
  ResolveDisputeInput
} from '../../types/admin';

const TABS = ['users', 'moderation', 'activity'] as const;
type Tab = (typeof TABS)[number];

const cardClass = 'bg-white border border-gray-200 rounded-lg p-4';
const statLabel = 'text-sm text-gray-500';

export function AdminDashboard() {
  const { apiRequest } = useApi();
  const { notify } = useAppNotifications();

  const [tab, setTab] = createSignal<Tab>('users');
  const [sessionAudit, setSessionAudit] = createSignal<AdminAuditEntry[]>(getAdminAuditLog());

  const [stats, { refetch: refetchStats }] = createResource(async (): Promise<AdminDashboardStats> => fetchAdminStats(apiRequest));
  const [users, { refetch: refetchUsers }] = createResource(
    async (): Promise<AdminUserSummary[]> => {
      const page = await fetchAdminUsers({ limit: 50 }, apiRequest);
      return page.items;
    }
  );
  const [queue, { refetch: refetchQueue }] = createResource(
    async (): Promise<ModerationItem[]> => fetchModerationQueue(apiRequest)
  );
  const [activity, { refetch: refetchActivity }] = createResource(
    async (): Promise<ActivityEntry[]> => fetchActivityFeed(apiRequest)
  );

  const refreshAll = () => {
    void refetchStats();
    void refetchUsers();
    void refetchQueue();
    void refetchActivity();
  };

  /** Wraps a moderation API call with the required audit trail + feedback. */
  const moderate = (action: string, message: string, fn: () => Promise<unknown>) =>
    void (async () => {
      try {
        await fn();
        setSessionAudit(
          getAdminAuditLog()
        );
        notify(message);
        refreshAll();
      } catch (err) {
        const entry = recordAdminAction({
          action,
          message: `${message} failed: ${(err as Error).message}`,
          targetId: '-',
          succeeded: false
        });
        setSessionAudit(getAdminAuditLog());
        notify(entry.message, 'error');
      }
    })();

  return (
    <div class="max-w-5xl mx-auto px-4 py-8">
      <h2 class="text-2xl font-bold text-gray-900 mb-6">Admin Dashboard</h2>

      {/* Key counts header */}
      <Show
        when={stats()}
        fallback={<p class="text-gray-500">{stats.loading ? 'Loading stats...' : 'Failed to load stats.'}</p>}
      >
        {(current) => (
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
            <div class={cardClass} data-testid="stat-total-users">
              <p class="text-2xl font-bold">{current().totalUsers}</p>
              <p class={statLabel}>Total users</p>
            </div>
            <div class={cardClass} data-testid="stat-active-users">
              <p class="text-2xl font-bold">{current().activeUsers}</p>
              <p class={statLabel}>Active users</p>
            </div>
            <div class={cardClass} data-testid="stat-pending-listings">
              <p class="text-2xl font-bold">{current().pendingListings}</p>
              <p class={statLabel}>Pending listings</p>
            </div>
            <div class={cardClass} data-testid="stat-open-disputes">
              <p class="text-2xl font-bold">{current().openDisputes}</p>
              <p class={statLabel}>Open disputes</p>
            </div>
          </div>
        )}
      </Show>

      <div class="flex gap-2 mb-4">
        <For each={[...TABS]}>
          {(t) => (
            <button
              type="button"
              class={
                'px-3 py-1.5 rounded-md text-sm font-medium ' +
                (tab() === t ? 'bg-blue-600 text-white' : 'bg-gray-100 text-gray-700')
              }
              onClick={() => setTab(t)}
            >
              {t === 'users' && 'Users'}
              {t === 'moderation' && 'Moderation queue'}
              {t === 'activity' && 'Activity'}
            </button>
          )}
        </For>
      </div>

      <Switch>
        <Match when={tab() === 'users'}>
          <AdminUsersSection
            users={users()}
            onBan={(target) =>
              moderate('user_ban', `User ${target} banned`, async () => {
                await banUser(target, apiRequest);
                recordAdminAction({
                  action: 'user_ban',
                  message: `Banned user ${target}`,
                  targetId: target,
                  succeeded: true
                });
              })
            }
            onUnban={(target) =>
              moderate('user_unban', `Ban lifted for ${target}`, async () => {
                await unbanUser(target, apiRequest);
                recordAdminAction({
                  action: 'user_unban',
                  message: `Lifted ban for user ${target}`,
                  targetId: target,
                  succeeded: true
                });
              })
            }
            onRefresh={() => void refetchUsers()}
          />
        </Match>

        <Match when={tab() === 'moderation'}>
          <ModerationQueueSection
            items={queue()}
            onApprove={(target) =>
              moderate('listing_approved', `Listing ${target} approved`, async () => {
                await approveListing(target, apiRequest);
                recordAdminAction({
                  action: 'listing_approved',
                  message: `Approved listing ${target}`,
                  targetId: target,
                  succeeded: true
                });
              })
            }
            onRemove={(target) =>
              moderate('listing_removed', `Listing ${target} removed`, async () => {
                await removeListing(target, apiRequest);
                recordAdminAction({
                  action: 'listing_removed',
                  message: `Removed listing ${target}`,
                  targetId: target,
                  succeeded: true
                });
              })
            }
            onResolveDispute={(disputeId, input) =>
              moderate('dispute_resolved', `Dispute ${disputeId} resolved`, async () => {
                await resolveDispute(disputeId, input, apiRequest);
                recordAdminAction({
                  action: 'dispute_resolved',
                  message: `Resolved dispute ${disputeId} (${input.outcome})`,
                  targetId: disputeId,
                  succeeded: true
                });
              })
            }
            onRefresh={() => void refetchQueue()}
          />
        </Match>

        <Match when={tab() === 'activity'}>
          <ActivityFeedSection
            entries={activity()}
            sessionAudit={sessionAudit()}
            onLoadServer={() => void refetchActivity()}
            onClearLocal={() => {
              clearAdminAuditLog();
              setSessionAudit([]);
            }}
          />
        </Match>
      </Switch>
    </div>
  );
}

/* ------------------------------ Users ------------------------------ */

function AdminUsersSection(props: {
  users?: AdminUserSummary[];
  onBan: (targetId: string) => void;
  onUnban: (targetId: string) => void;
  onRefresh: () => void;
}) {
  return (
    <section class={cardClass}>
      <div class="flex items-center justify-between mb-3">
        <h3 class="font-semibold text-gray-900">Users</h3>
        <button type="button" class="text-sm text-blue-600" onClick={props.onRefresh}>
          Refresh
        </button>
      </div>
      <Show when={props.users} fallback={<p class="text-gray-500">Loading users...</p>}>
        <table class="w-full text-sm">
          <thead>
            <tr class="text-left text-gray-500">
              <th class="py-2">User</th>
              <th class="py-2">Email</th>
              <th class="py-2">Roles</th>
              <th class="py-2">Status</th>
              <th class="py-2" />
            </tr>
          </thead>
          <tbody>
            <For each={props.users ?? []}>
              {(user) => (
                <tr class="border-t border-gray-100" data-testid={`admin-user-${user.id}`}>
                  <td class="py-2">{user.displayName || user.id}</td>
                  <td class="py-2">{user.email}</td>
                  <td class="py-2">{user.roles.join(', ') || 'user'}</td>
                  <td class="py-2">
                    <Show
                      when={user.banned}
                      fallback={<span class="text-green-700">Active</span>}
                    >
                      <span class="text-red-600">Banned</span>
                    </Show>
                  </td>
                  <td class="py-2 text-right">
                    <button
                      type="button"
                      class={'text-sm px-2 ' + (user.banned ? 'text-green-700' : 'text-red-600')}
                      onClick={() => (user.banned ? props.onUnban(user.id) : props.onBan(user.id))}
                    >
                      {user.banned ? 'Unban' : 'Ban'}
                    </button>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </Show>
    </section>
  );
}

/* --------------------------- Moderation queue --------------------------- */

function ModerationQueueSection(props: {
  items?: ModerationItem[];
  onApprove: (listingId: string) => void;
  onRemove: (listingId: string) => void;
  onResolveDispute: (disputeId: string, input: ResolveDisputeInput) => void;
  onRefresh: () => void;
}) {
  const [note, setNote] = createSignal('');
  const [outcome, setOutcome] = createSignal<ResolveDisputeInput['outcome']>('dismissed');
  const [disputeId, setDisputeId] = createSignal('');

  return (
    <section class="space-y-4">
      <section class={cardClass}>
        <div class="flex items-center justify-between mb-3">
          <h3 class="font-semibold text-gray-900">Moderation queue</h3>
          <button type="button" class="text-sm text-blue-600" onClick={props.onRefresh}>
            Refresh
          </button>
        </div>
        <Show when={props.items} fallback={<p class="text-gray-500">Loading queue...</p>}>
          <Show
            when={(props.items ?? []).length > 0}
            fallback={<p class="text-gray-500">Nothing pending — all listings reviewed.</p>}
          >
            <ul class="divide-y divide-gray-100">
              <For each={props.items ?? []}>
                {(item) => (
                  <li class="py-2 flex items-center justify-between" data-testid={`mod-item-${item.id}`}>
                    <div>
                      <p class="text-gray-900">{item.title}</p>
                      <p class="text-xs text-gray-500">by {item.sellerName || item.sellerId}</p>
                    </div>
                    <div class="flex gap-2">
                      <button
                        type="button"
                        class="text-sm px-3 py-1 border border-green-600 text-green-700 rounded"
                        onClick={() => props.onApprove(item.id)}
                      >
                        Approve
                      </button>
                      <button
                        type="button"
                        class="text-sm px-3 py-1 border border-red-600 text-red-600 rounded"
                        onClick={() => props.onRemove(item.id)}
                      >
                        Remove
                      </button>
                    </div>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </Show>
      </section>

      <section class={cardClass}>
        <h3 class="font-semibold text-gray-900 mb-3">Resolve dispute</h3>
        <div class="flex flex-col gap-2 sm:flex-row">
          <input
            class="border border-gray-300 rounded px-3 py-2 flex-1"
            placeholder="Dispute id"
            value={disputeId()}
            onChange={(e) => setDisputeId(e.currentTarget.value)}
          />
          <select
            class="border border-gray-300 rounded px-3 py-2"
            value={outcome()}
            onChange={(e) => setOutcome(e.currentTarget.value as ResolveDisputeInput['outcome'])}
          >
            <option value="seller">Favor seller</option>
            <option value="buyer">Favor buyer</option>
            <option value="split">Split</option>
            <option value="dismissed">Dismiss</option>
          </select>
          <input
            class="border border-gray-300 rounded px-3 py-2 flex-1"
            placeholder="Resolution note"
            value={note()}
            onChange={(e) => setNote(e.currentTarget.value)}
          />
          <button
            type="button"
            class="px-3 py-2 bg-blue-600 text-white rounded disabled:opacity-50"
            disabled={!disputeId().trim() || !note().trim()}
            onClick={() =>
              props.onResolveDispute(disputeId().trim(), {
                outcome: outcome(),
                resolutionNote: note().trim()
              })
            }
          >
            Resolve
          </button>
        </div>
      </section>
    </section>
  );
}

/* ----------------------------- Activity feed ----------------------------- */

function ActivityFeedSection(props: {
  entries?: ActivityEntry[];
  sessionAudit: AdminAuditEntry[];
  onLoadServer: () => void;
  onClearLocal: () => void;
}) {
  return (
    <section class="space-y-4">
      <section class={cardClass}>
        <div class="flex items-center justify-between mb-3">
          <h3 class="font-semibold text-gray-900">Server activity</h3>
          <button type="button" class="text-sm text-blue-600" onClick={props.onLoadServer}>
            Refresh
          </button>
        </div>
        <Show when={props.entries} fallback={<p class="text-gray-500">Loading activity...</p>}>
          <ul class="text-sm divide-y divide-gray-100">
            <For each={props.entries ?? []}>
              {(entry) => (
                <li class="py-2" data-testid={`activity-${entry.id}`}>
                  <span class="text-gray-900">{entry.message}</span>
                  <span class="text-gray-500"> · {new Date(entry.createdAt).toLocaleString()}</span>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </section>

      <section class={cardClass}>
        <div class="flex items-center justify-between mb-3">
          <h3 class="font-semibold text-gray-900">Session audit log (this browser)</h3>
          <button type="button" class="text-sm text-gray-500" onClick={props.onClearLocal}>
            Clear
          </button>
        </div>
        <Show
          when={props.sessionAudit.length > 0}
          fallback={<p class="text-gray-500">No moderation actions were taken in this session.</p>}
        >
          <ul class="text-sm divide-y divide-gray-100">
            <For each={props.sessionAudit}>
              {(entry) => (
                <li class="py-2">
                  {entry.succeeded ? '✓' : '✗'} {entry.message}
                  <span class="text-gray-500"> · {new Date(entry.createdAt).toLocaleString()}</span>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </section>
    </section>
  );
}
