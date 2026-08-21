import { createSignal } from 'solid-js';
import { useAuth } from '../../contexts/AuthContext';
import { Link } from '@solidjs/router';

export function ForgotPassword() {
  const { forgotPassword, state } = useAuth();
  const [email, setEmail] = createSignal('');
  const [emailSent, setEmailSent] = createSignal(false);
  const [validationError, setValidationError] = createSignal<string | null>(null);

  const validateEmail = () => {
    if (!email()) {
      setValidationError('Email is required');
      return false;
    } else if (!/\S+@\S+\.\S+/.test(email())) {
      setValidationError('Please enter a valid email address');
      return false;
    }
    setValidationError(null);
    return true;
  };

  const handleSubmit = async (e: SubmitEvent) => {
    e.preventDefault();
    if (!validateEmail()) return;

    try {
      await forgotPassword(email());
      setEmailSent(true);
    } catch (error) {
      console.error('Failed to send reset email:', error);
    }
  };

  return (
    <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
      <div class="max-w-md w-full space-y-8">
        <div>
          <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
            Reset your password
          </h2>
          <p class="mt-2 text-center text-sm text-gray-600">
            Enter your email address and we'll send you a link to reset your password.
          </p>
        </div>
        
        {!emailSent() ? (
          <form class="mt-8 space-y-6" onSubmit={handleSubmit}>
            <div class="rounded-md shadow-sm">
              <div>
                <label for="email-address" class="sr-only">Email address</label>
                <input
                  id="email-address"
                  name="email"
                  type="email"
                  autocomplete="email"
                  required
                  class="appearance-none rounded-md relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                  placeholder="Email address"
                  value={email()}
                  onInput={(e) => setEmail(e.target.value)}
                />
                {validationError() && (
                  <p class="text-red-500 text-xs mt-1">{validationError()}</p>
                )}
              </div>
            </div>

            {state.error && (
              <div class="text-red-500 text-sm text-center bg-red-50 p-2 rounded">
                {state.error}
              </div>
            )}

            <div class="flex items-center justify-between">
              <div class="text-sm">
                <Link href="/sign-in" class="font-medium text-blue-600 hover:text-blue-500">
                  Back to sign in
                </Link>
              </div>
            </div>

            <div>
              <button
                type="submit"
                disabled={state.isLoading}
                class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:bg-blue-300"
              >
                {state.isLoading ? 'Sending...' : 'Send reset link'}
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
                  <h3 class="text-sm font-medium text-green-800">Email sent</h3>
                  <div class="mt-2 text-sm text-green-700">
                    <p>Check your email for a link to reset your password. If it doesn't appear within a few minutes, check your spam folder.</p>
                  </div>
                </div>
              </div>
            </div>
            <div class="text-center">
              <Link href="/sign-in" class="font-medium text-blue-600 hover:text-blue-500">
                Return to sign in
              </Link>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}