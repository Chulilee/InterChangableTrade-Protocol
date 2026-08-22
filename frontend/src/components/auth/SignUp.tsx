import { createSignal } from 'solid-js';
import { useAuth } from '../../contexts/AuthContext';
import { useNavigate, Link } from '@solidjs/router';

export function SignUp() {
  const { signUp, state } = useAuth();
  const navigate = useNavigate();
  const [name, setName] = createSignal('');
  const [email, setEmail] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [confirmPassword, setConfirmPassword] = createSignal('');
  const [validationErrors, setValidationErrors] = createSignal<{
    name?: string;
    email?: string;
    password?: string;
    confirmPassword?: string;
  }>({});

  const validateForm = () => {
    const errors: typeof validationErrors = {};
    if (!name()) {
      errors.name = 'Full name is required';
    } else if (name().length < 2) {
      errors.name = 'Name must be at least 2 characters';
    }
    if (!email()) {
      errors.email = 'Email is required';
    } else if (!/\S+@\S+\.\S+/.test(email())) {
      errors.email = 'Please enter a valid email address';
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
    if (!validateForm()) return;

    try {
      await signUp(email(), password(), name());
      navigate('/dashboard');
    } catch (error) {
      console.error('Registration failed:', error);
    }
  };

  return (
    <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
      <div class="max-w-md w-full space-y-8">
        <div>
          <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
            Create your account
          </h2>
          <p class="mt-2 text-center text-sm text-gray-600">
            Already have an account?{' '}
            <Link href="/sign-in" class="font-medium text-blue-600 hover:text-blue-500">
              Sign in
            </Link>
          </p>
        </div>
        <form class="mt-8 space-y-6" onSubmit={handleSubmit}>
          <div class="rounded-md shadow-sm -space-y-px">
            <div>
              <label for="full-name" class="sr-only">Full name</label>
              <input
                id="full-name"
                name="name"
                type="text"
                autocomplete="name"
                required
                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                placeholder="Full name"
                value={name()}
                onInput={(e) => setName(e.target.value)}
              />
              {validationErrors().name && (
                <p class="text-red-500 text-xs mt-1">{validationErrors().name}</p>
              )}
            </div>
            <div>
              <label for="email-address" class="sr-only">Email address</label>
              <input
                id="email-address"
                name="email"
                type="email"
                autocomplete="email"
                required
                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
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
                autocomplete="new-password"
                required
                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                placeholder="Password"
                value={password()}
                onInput={(e) => setPassword(e.target.value)}
              />
              {validationErrors().password && (
                <p class="text-red-500 text-xs mt-1">{validationErrors().password}</p>
              )}
            </div>
            <div>
              <label for="confirm-password" class="sr-only">Confirm password</label>
              <input
                id="confirm-password"
                name="confirmPassword"
                type="password"
                autocomplete="new-password"
                required
                class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
                placeholder="Confirm password"
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
              {state.isLoading ? 'Creating account...' : 'Sign up'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}