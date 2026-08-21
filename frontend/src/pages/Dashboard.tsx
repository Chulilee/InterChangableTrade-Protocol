import { useAuth } from '../contexts/AuthContext';
import { useNavigate } from '@solidjs/router';

export function Dashboard() {
  const { state, signOut } = useAuth();
  const navigate = useNavigate();

  const handleSignOut = async () => {
    await signOut();
    navigate('/sign-in');
  };

  return (
    <div class="min-h-screen bg-gray-100">
      <nav class="bg-white shadow-sm">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div class="flex justify-between h-16">
            <div class="flex items-center">
              <h1 class="text-xl font-semibold text-gray-900">Dashboard</h1>
            </div>
            <div class="flex items-center gap-4">
              <span class="text-gray-700">Welcome, {state.user?.name}</span>
              <button
                onClick={handleSignOut}
                class="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 transition-colors"
              >
                Sign out
              </button>
            </div>
          </div>
        </div>
      </nav>
      
      <main class="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
        <div class="px-4 py-6 sm:px-0">
          <div class="border-4 border-dashed border-gray-200 rounded-lg p-8 bg-white">
            <h2 class="text-2xl font-bold text-gray-900 mb-4">Welcome to your dashboard</h2>
            <p class="text-gray-600 mb-4">
              You are successfully authenticated! Your user profile:
            </p>
            <pre class="bg-gray-50 p-4 rounded-md overflow-auto">
              {JSON.stringify(state.user, null, 2)}
            </pre>
          </div>
        </div>
      </main>
    </div>
  );
}