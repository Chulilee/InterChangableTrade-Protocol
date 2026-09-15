import { createSignal } from 'solid-js';
import { useAuth } from '../../contexts/AuthContext';
import { useNavigate, Link } from '@solidjs/router';

export function SignIn() {
  const { signIn, state } = useAuth();
  const navigate = useNavigate();
  const [email, setEmail] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [validationErrors, setValidationErrors] = createSignal<{email?: string; password?: string}>({});

  const validateForm = () => {
    const errors: {email?: string; password?: string} = {};
    if (!email()) {
      errors.email = 'Email is required';
    } else if (!/\S+@\S+\.\S+/.test(email())) {
      errors.email = 'Please enter a valid email address';
    }
    if (!password()) {
      errors.password = 'Password is required';
    } else if (password().length < 6) {
      errors.password = 'Password must be at least 6 characters';
    }
    setValidationErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleSubmit = async (e: SubmitEvent) => {
    e.preventDefault();
    if (!validateForm()) return;

    try {
      await signIn(email(), password());
      // Redirect to the page the user was trying to access, or home
      const redirectPath = sessionStorage.getItem('redirectAfterLogin') || '/dashboard';
      sessionStorage.removeItem('redirectAfterLogin');
      navigate(redirectPath);
    } catch (error) {
      // Error is already stored in state.error
      console.error('Login failed:', error);
    }
  };

  return (
    <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
      <div class="max-w-md w-full space-y-8">
        <div>
          <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
            Sign in to your account
          </h2>
          <p class="mt-2 text-center text-sm text-gray-600">
            Or{' '}
            <Link href="/sign-up" class="font-medium text-blue-600 hover:text-blue-500">
              create a new account
            </Link>
          </p>
        </div>
        <form class="mt-8 space-y-6" onSubmit={handleSubmit}>
          <div class="rounded-md shadow-sm -space-y-px">
            <div>
              <label for="email-address" class="sr-only">Email address</label>
              <input
                id="email-address"
                name="email"
                type="email"
                autocomplete="email"
                required
                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                placeholder="Email address"
                value={email()}
                onInput={(e) => setEmail(e.target.value)}
              />
              {validationErrors().email && (
                <p class="text-red-500 text-xs mt-1">{validationErrors().email}</p>
              )}
            </div>
            <div>
              <label for="password" class="sr-only">Password</label>
              <input
                id="password"
                name="password"
                type="password"
                autocomplete="current-password"
                required
                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                placeholder="Password"
                value={password()}
                onInput={(e) => setPassword(e.target.value)}
              />
              {validationErrors().password && (
                <p class="text-red-500 text-xs mt-1">{validationErrors().password}</p>
              )}
            </div>
          </div>

          <div class="flex items-center justify-between">
            <div class="text-sm">
              <Link href="/forgot-password" class="font-medium text-blue-600 hover:text-blue-500">
                Forgot your password?
              </Link>
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
              {state.isLoading ? 'Signing in...' : 'Sign in'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}