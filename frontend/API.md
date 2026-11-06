# Frontend ↔ Backend API Contract

The frontend in this repository talks to a remote REST API. This document
describes **every endpoint the frontend calls**: method, path, parameters,
request/response bodies, auth requirements and error handling. It is the
contract an implementing backend (InterChangableTrade-Core) must satisfy, and
it is derived from the source of truth in `frontend/src/api/*` and
`frontend/src/contexts/AuthContext.tsx` — if you change either, update this
file in the same PR.

Unless stated otherwise:

- Request bodies are JSON with `Content-Type: application/json`
  (multipart only for the avatar upload).
- Responses are JSON.
- All requests are sent with `credentials: 'include'` (cookies) **and** an
  `Authorization: Bearer <accessToken>` header when authenticated.
- Successful writes return the updated resource; void endpoints return an
  empty body (`204 No Content` is acceptable).

## Configuration

| Variable      | Default                    | Description                                   |
| ------------- | -------------------------- | --------------------------------------------- |
| `VITE_API_URL` | `http://localhost:3000/api` | Base URL of the REST API, including the `/api` prefix. Used by `AuthContext`, `useApi`, and all `src/api/*` modules. |

There is no other per-environment backend configuration in the frontend.
Copy `.env.example` (if present) or export `VITE_API_URL` before running the
dev server.

## Authentication

JWT access tokens + refresh tokens:

1. `POST /auth/login` or `POST /auth/register` returns `{ user, accessToken, refreshToken, expiresIn }`.
2. The frontend persists `{ accessToken, refreshToken, expiresAt }` in
   `localStorage["auth_tokens"]` with `expiresAt = now + expiresIn * 1000`.
3. Authenticated calls build an `apiRequest` via `useApi()` (`src/hooks/useApi.ts`):
   - Proactively refreshes when the access token is within 5 minutes of
     `expiresAt` (`TOKEN_REFRESH_THRESHOLD = 5 * 60 * 1000`).
   - Sends `Authorization: Bearer <accessToken>`.
   - On `401`, refreshes **once** and retries the original request; if the
     refresh fails, throws `Session expired. Please sign in again.`
4. `POST /auth/refresh` is also called on a 60s interval while authenticated.
5. Token payload user object is used as-is: `{ id, email, name, roles?, createdAt }`.

### Auth endpoints

#### `POST /auth/register`
Registers a new user and signs them in.

- **Auth:** none
- **Body:** `{ "email": string, "password": string, "name": string }`
- **Response `200 OK`:**
  ```json
  {
    "user": { "id": "u_123", "email": "ada@example.com", "name": "Ada", "roles": [], "createdAt": "2026-01-02T15:04:05.000Z" },
    "accessToken": "<jwt>",
    "refreshToken": "<jwt>",
    "expiresIn": 3600
  }
  ```
- **Errors:** `400` invalid input, `409` email already registered.

#### `POST /auth/login`
- **Auth:** none
- **Body:** `{ "email": string, "password": string }`
- **Response `200 OK`:** same shape as register.
- **Errors:** `400` invalid input, `401` bad credentials.

#### `POST /auth/refresh`
- **Auth:** none (refresh token in body)
- **Body:** `{ "refreshToken": string }`
- **Response `200 OK`:** `{ "accessToken": string, "refreshToken"?: string, "expiresIn": number }`
  (`refreshToken` optional — the frontend keeps the old one when absent.)
- **Errors:** `401` refresh token rejected.

#### `POST /auth/logout`
- **Auth:** bearer required
- **Body:** none
- **Response `200 OK`:** empty. The frontend clears local tokens regardless of the outcome (logout is fire-and-forget).

#### `GET /auth/profile`
- **Auth:** bearer required
- **Response `200 OK`:** the `User` object as emitted at login,
  `{ id, email, name, roles?, createdAt }`.

#### `POST /auth/forgot-password`
- **Auth:** none
- **Body:** `{ "email": string }`
- **Response `200 OK`:** empty. The frontend treats any non-2xx as an error and displays the server `message`.

#### `POST /auth/reset-password`
- **Auth:** none
- **Body:** `{ "token": string, "newPassword": string }`
- **Response `200 OK`:** empty.

## Listings — `/listings`

Types are defined in `frontend/src/types/listing.ts` (`Listing`, `ListingInput`, `ListingsQuery`, `PaginatedListings`).

#### `GET /listings`
Fetches a page of listings. No auth required (reads are public).

- **Query params** (`ListingsQuery`, all optional):

  | Param       | Type             | Notes                                    |
  | ----------- | ---------------- | ---------------------------------------- |
  | `page`      | integer          | 1-based page number                      |
  | `limit`     | integer          | page size                                |
  | `search`    | string           | free-text across title/description       |
  | `sortBy`    | `date` \| `price` |                                          |
  | `sortOrder` | `asc` \| `desc`  |                                          |
  | `categories` | string, comma-joined | e.g. `categories=electronics,fashion` |
  | `tags`      | string, comma-joined |                                       |
  | `minPrice`  | number           | inclusive                                |
  | `maxPrice`  | number           | inclusive                                |

- **Response `200 OK` (`PaginatedListings`):**
  ```json
  {
    "items": [
      {
        "id": "l_1",
        "title": "Vintage lens",
        "description": "50mm, great condition",
        "price": 120,
        "images": ["https://cdn.example.com/l_1-1.jpg"],
        "sellerId": "u_9",
        "sellerName": "ada",
        "category": "cameras",
        "tags": ["vintage", "lens"],
        "createdAt": "2026-01-02T15:04:05.000Z",
        "updatedAt": "2026-01-02T15:04:05.000Z"
      }
    ],
    "total": 137,
    "page": 1,
    "limit": 24,
    "totalPages": 6
  }
  ```
- **Errors:** `400` invalid query, `429` rate-limited (see below), `5xx` backend failure.

#### `GET /listings/:id`
Fetches one listing. No auth required.

- **Response `200 OK`:** a single `Listing` as above.
- **Errors:** `404` not found.

#### `POST /listings`
Creates a listing. **Auth required.**

- **Body (`ListingInput`):** `{ "title": string, "description": string, "price": number, "images": string[], "category"?: string, "tags"?: string[] }`
- **Response `200 OK` / `201 Created`:** the created `Listing`.
- **Errors:** `400` invalid input, `401` unauthenticated, `403` banned seller.

#### `PUT /listings/:id`
Updates a listing (partial body allowed). **Auth required** (owner or admin).

- **Body:** `Partial<ListingInput>`
- **Response `200 OK`:** the updated `Listing`.
- **Errors:** `400`, `401`, `403` not the owner, `404`.

#### `DELETE /listings/:id`
Deletes a listing. **Auth required** (owner or admin).

- **Response `200 OK` / `204 No Content`:** empty.

## Users — `/users`

#### `GET /users/me`
Returns the authenticated user's profile. **Auth required.**

- **Response `200 OK` (`UserProfile`):**
  ```json
  {
    "id": "u_123",
    "email": "ada@example.com",
    "displayName": "Ada",
    "avatarUrl": "https://cdn.example.com/avatars/u_123.png",
    "createdAt": "2026-01-02T15:04:05.000Z"
  }
  ```
  Note: `avatarUrl` is `null` when unset; `displayName` is the current display
  name (may differ from the auth payload `name` — the profile is the source of
  truth after updates).

#### `PATCH /users/me`
Updates editable profile fields. **Auth required.**

- **Body (`UpdateProfileInput`):** `{ "displayName"?: string, "email"?: string }`
- **Response `200 OK`:** the updated `UserProfile`.
- **Errors:** `400` (e.g. email taken), `401`.

#### `GET /users/me/preferences`
Returns saved notification preferences. **Auth required.**

- **Response `200 OK` (`UserPreferences`):**
  ```json
  { "emailNotifications": true, "pushNotifications": true, "inAppNotifications": true }
  ```

#### `PUT /users/me/preferences`
Replaces notification preferences. **Auth required.**

- **Body (`UserPreferences`):** all three booleans (full replacement).
- **Response `200 OK`:** the saved `UserPreferences`.

#### `POST /users/me/avatar`
Uploads an avatar as `multipart/form-data`. **Auth required.**

- **Body:** multipart form with a single field `avatar` containing the image file (Blob). The browser must supply the `Content-Type: multipart/form-data; boundary=...` header — the frontend deliberately does **not** set `Content-Type` for this call.
- **Response `200 OK` (`AvatarUploadResult`):**
  ```json
  { "avatarUrl": "https://cdn.example.com/avatars/u_123.png" }
  ```
- **Errors:** `400` unsupported/oversized image, `401`.

## Admin — `/admin`

All admin endpoints require a bearer token whose user carries `roles` including
`"admin"`. The frontend additionally guards admin routes (see
`src/utils/roles.ts` + `ProtectedRoute`); the backend **must** enforce the
role server-side on every route. All mutating admin actions are audited
server-side.

#### `GET /admin/stats`
Aggregate counts for the dashboard header cards.

- **Response `200 OK` (`AdminDashboardStats`):**
  ```json
  { "totalUsers": 1204, "activeUsers": 312, "pendingListings": 17, "openDisputes": 3 }
  ```

#### `GET /admin/users`
Paginated user list with moderation-relevant fields.

- **Query params** (`AdminUsersQuery`, all optional):

  | Param    | Type    | Notes                                  |
  | -------- | ------- | -------------------------------------- |
  | `page`   | integer | 1-based                                |
  | `limit`  | integer | page size                              |
  | `search` | string  | email/name match                       |
  | `banned` | boolean | `true` returns only banned accounts    |

- **Response `200 OK` (`Paginated<AdminUserSummary>`):** `{ items, total, page, limit }`
  ```json
  {
    "items": [
      {
        "id": "u_9",
        "email": "bob@example.com",
        "displayName": "Bob",
        "avatarUrl": null,
        "roles": ["user"],
        "banned": false,
        "createdAt": "2026-01-02T15:04:05.000Z"
      }
    ],
    "total": 1204,
    "page": 1,
    "limit": 25
  }
  ```

#### `GET /admin/moderation-queue`
Listings awaiting moderation.

- **Response `200 OK`:** `{ "items": ModerationItem[] }` or a bare `ModerationItem[]` (the frontend accepts both):
  ```json
  {
    "items": [
      { "id": "l_1", "title": "Vintage lens", "sellerId": "u_9", "sellerName": "bob", "submittedAt": "2026-01-02T15:04:05.000Z", "status": "pending" }
    ]
  }
  ```
  `status` is `pending` until the listing is approved or removed.

#### `GET /admin/activity`
Recent admin activity feed.

- **Response `200 OK`:** `{ "items": ActivityEntry[] }` or a bare `ActivityEntry[]`:
  ```json
  {
    "items": [
      { "id": "a_1", "type": "user_ban", "message": "Banned u_9", "actorId": "u_1", "createdAt": "2026-01-02T15:04:05.000Z" }
    ]
  }
  ```

#### `POST /admin/users/:id/ban`
Bans a user from signing in and listing. Audited.

- **Response `200 OK` / `204 No Content`:** empty.

#### `POST /admin/users/:id/unban`
Lifts a ban. Audited.

- **Response `200 OK` / `204 No Content`:** empty.

#### `DELETE /admin/listings/:id`
Removes (unpublishes) a non-compliant listing. Audited.

- **Response `200 OK` / `204 No Content`:** empty.

#### `POST /admin/listings/:id/approve`
Approves a pending listing out of the moderation queue. Audited.

- **Response `200 OK` / `204 No Content`:** empty.

#### `POST /admin/disputes/:id/resolve`
Resolves a dispute with an explicit outcome and a required note. Audited.

- **Body (`ResolveDisputeInput`):** `{ "outcome": "seller" | "buyer" | "split" | "dismissed", "resolutionNote": string }`
- **Response `200 OK` / `204 No Content`:** empty.

## Error format

All non-2xx responses are expected to carry a JSON body with a human-readable
`message` string; the frontend surfaces it directly:

```json
{ "message": "Listing not found" }
```

`throwForErrorResponse` falls back to a generic message when the body is
missing or not JSON (e.g. upstream HTML error pages). Common mappings the
backend should honor:

| Status | Meaning                       | Frontend behavior                          |
| ------ | ----------------------------- | ------------------------------------------ |
| `200/201` | OK/created                  | parse JSON                                 |
| `204`  | No content                    | empty body, success                        |
| `400`  | Validation failed             | throws `Error(message)`                    |
| `401`  | Unauthenticated/token expired | `apiRequest` refreshes once and retries; else throws `Session expired…` |
| `403`  | Forbidden (role/ban)          | throws `Error(message)`                    |
| `404`  | Not found                     | throws `Error(message)`                    |
| `409`  | Conflict (e.g. duplicate email) | throws `Error(message)`                   |
| `429`  | Rate limited                  | see below                                  |
| `5xx`  | Backend failure               | throws `Error(message)` or generic fallback |

### Rate limiting

The frontend **does not** currently special-case `429 Too Many Requests`: it
throws the server-provided `message` (or a generic fallback) and lets the UI
surface it. Backends behind `VITE_API_URL` are expected to rate limit per
IP/token and to return a `Retry-After` header on `429`. Client-side
backoff/retry for `429` is tracked separately; until it lands, implementers
should prefer burst-tolerant limits (e.g. token bucket) and consistent
`Retry-After` values.

## Versioning

No version prefix is used today (`/listings`, not `/v1/listings`). Introduce
one only via a coordinated backend + frontend release; the frontend builds
URLs from `VITE_API_URL` verbatim and will not rewrite paths.
