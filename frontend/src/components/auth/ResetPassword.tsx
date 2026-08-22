import { createSignal, createEffect } from 'solid-js';
import { useAuth } from '../../contexts/AuthContext';
import { useNavigate, useSearchParams, Link } from '@solidjs/router';

export function ResetPassword() {
  const { resetPassword, state } = useAuth();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [password, setPassword] = createSignal('');
  const [confirmPassword, setConfirmPassword] = createSignal('');
  const [resetComplete, setResetComplete] = createSignal(false);
  const [validationErrors, setValidationErrors] = createSignal<{
    password?: string;
    confirmPassword?: string;
    token?: string;
  }>({});

  const token = searchParams.get('token');

  createEffect(() => {
    if (!token) {
      setValidationErrors(prev => ({ ...prev, token: 'Invalid or missing reset token' }));
    }
  });

  const validateForm = () => {
    const errors: typeof validationErrors = {};
    if (!token) {
      errors.token = 'Invalid or missing reset token';
    }
    if (!password()) {
      errors.password = 'Password is required';
    } else if (password().length < 8) {
      errors.password = 'Password must be at least 8 characters';
    } else if (!/(?=.*[a-z])(?=.*[A-Z])(?=.*\d)/.test(password())) {
      errors.password = 'Password must contain at least one uppercase letter, one lowercase letter, and one number';
    }
    if (password() !== confirmPassword()) {
      errors.confirmPassword = 'Passwords do not match';
    }
    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleSubmit = async (e: SubmitEvent) => {
    e.preventDefault();
    if (!validateForm() || !token) return;

    try {
      await resetPassword(token, password());
      setResetComplete(true);
    } catch (error) {
      console.error('Password reset failed:', error);
    }
  };

  if (validationErrors().token && !resetComplete()) {
    return (
      <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
        <div class="max-w-md w-full space-y-8">
          <div class="bg-red-50 border border-red-200 rounded-md p-4">
            <div class="flex">
              <div class="flex-shrink-0">
                <svg class="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
                  <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
                </svg>
              </div>
              <div class="ml-3">
                <h3 class="text-sm font-medium text-red-800">Invalid reset link</h3>
                <div class="mt-2 text-sm text-red-700">
                  <p>{validationErrors().token}</p>
                </div>
              </div>
            </div>
          </div>
          <div class="text-center">
            <Link href="/forgot-password" class="font-medium text-blue-600 hover:text-blue-500">
              Request a new password reset link
            </Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
      <div class="max-w-md w-full space-y-8">
        <div>
          <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
            Set new password
          </h2>
          <p class="mt-2 text-center text-sm text-gray-600">
            Please enter your new password.
          </p>
        </div>

        {!resetComplete() ? (
          <form class="mt-8 space-y-6" onSubmit={handleSubmit}>
            <div class="rounded-md shadow-sm -space-y-px">
              <div>
                <label for="password" class="sr-only">New password</label>
                <input
                  id="password"
                  name="password"
                  type="password"
                  autocomplete="new-password"
                  required
                  class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                  placeholder="New password"
                  value={password()}
                  onInput={(e) => setPassword(e.target.value)}
                />
                {validationErrors().password && (
                  <p class="text-red-500 text-xs mt-1">{validationErrors().password}</p>
                )}
              </div>
              <div>
                <label for="confirm-password" class="sr-only">Confirm new password</label>
                <input
                  id="confirm-password"
                  name="confirmPassword"
                  type="password"
                  autocomplete="new-password"
                  required
                  class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                  placeholder="Confirm new password"
                  value={confirmPassword()}
                  onInput={(e) => setConfirmPassword(e.target.value)}
                />
                {validationErrors().confirmPassword && (
                  <p class="text-red-500 text-xs mt-1">{validationErrors().confirmPassword}</p>
                )}
              </div>
            </div>

            {state.error && (
              <div class="text-red-500 text-sm text-center bg-red-50 p-2 rounded">
                {state.error}
              </div>
            )}

            <div>
              <button
                type="submit"
                disabled={state.isLoading}
                class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:bg-blue-300"
              >
                {state.isLoading ? 'Resetting...' : 'Reset password'}
              </button>
            </div>
          </form>
        ) : (
          <div class="mt-8 space-y-6">
            <div class="bg-green-50 border border-green-200 rounded-md p-4">
              <div class="flex">
                <div class="flex-shrink-0">
                  <svg class="h-5 w-5 text-green-400" viewBox="0 0 20 20" fill="currentColor">
                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
                  </svg>
                </div>
                <div class="ml-3">
                  <h3 class="text-sm font-medium text-green-800">Password reset successful</h3>
                  <div class="mt-2 text-sm text-green-700">
                    <p>Your password has been reset successfully. You can now sign in with your new password.</p>
                  </div>
                </div>
              </div>
            </div>
            <div class="text-center">
              <Link href="/sign-in" class="font-medium text-blue-600 hover:text-blue-500">
                Go to sign in
              </Link>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}