// Role-based access helpers (issue #26): shared by ProtectedRoute and unit
// tested in isolation, so route guarding logic stays trivial to verify.

export interface RoleBearer {
  roles?: string[];
}

/** True when the user carries at least one of the required roles. */
export function hasRequiredRole(
  user: RoleBearer | null | undefined,
  requiredRoles: string[] | undefined
): boolean {
  if (!requiredRoles || requiredRoles.length === 0) {
    return true;
  }
  if (!user?.roles) {
    return false;
  }
  return requiredRoles.some((role) => user.roles!.includes(role));
}
