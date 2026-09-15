import { Link } from '@solidjs/router';
import { useAuth } from '../contexts/AuthContext';

export function Home() {
  const { state } = useAuth();

  return (
    <div class="min-h-screen bg-gradient-to-b from-blue-50 to-white">
      <header class="bg-white shadow-sm">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div class="flex justify-between h-16">
            <div class="flex items-center">
              <h1 class="text-2xl font-bold text-blue-600">InterChangableTrade</h1>
            </div>
            <div class="flex items-center gap-4">
              {state.isAuthenticated ? (
                <Link
                  href="/dashboard"
                  class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
                >
                  Go to Dashboard
                </Link>
              ) : (
                <>
                  <Link
                    href="/sign-in"
                    class="px-4 py-2 text-blue-600 hover:text-blue-700 transition-colors"
                  >
                    Sign in
                  </Link>
                  <Link
                    href="/sign-up"
                    class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
                  >
                    Sign up
                  </Link>
                </>
              )}
            </div>
          </div>
        </div>
      </header>

      <main class="max-w-7xl mx-auto py-16 px-4 sm:px-6 lg:px-8">
        <div class="text-center">
          <h2 class="text-4xl font-extrabold text-gray-900 sm:text-5xl sm:tracking-tight lg:text-6xl">
            Cross-Chain Trading Protocol
          </h2>
          <p class="max-w-3xl mx-auto mt-6 text-xl text-gray-500">
            A secure, decentralized trading protocol for seamless cross-chain asset exchange.
            Start trading today with our robust and reliable platform.
          </p>
          <div class="mt-10 flex justify-center gap-4">
            {!state.isAuthenticated && (
              <Link
                href="/sign-up"
                class="px-8 py-3 text-lg font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 transition-colors"
              >
                Get started
              </Link>
            )}
            <a
              href="#learn-more"
              class="px-8 py-3 text-lg font-medium text-blue-600 bg-white border border-blue-600 rounded-md hover:bg-blue-50 transition-colors"
            >
              Learn more
            </a>
          </div>
        </div>

        <div id="learn-more" class="mt-24 grid grid-cols-1 gap-8 md:grid-cols-3">
          <div class="bg-white p-6 rounded-lg shadow-sm">
            <h3 class="text-lg font-semibold text-gray-900">Secure Trading</h3>
            <p class="mt-2 text-gray-600">
              Built with industry-standard security practices to protect your assets and data.
            </p>
          </div>
          <div class="bg-white p-6 rounded-lg shadow-sm">
            <h3 class="text-lg font-semibold text-gray-900">Cross-Chain Support</h3>
            <p class="mt-2 text-gray-600">
              Trade assets across multiple blockchain networks seamlessly.
            </p>
          </div>
          <div class="bg-white p-6 rounded-lg shadow-sm">
            <h3 class="text-lg font-semibold text-gray-900">Decentralized</h3>
            <p class="mt-2 text-gray-600">
              Fully decentralized protocol with no single point of failure.
            </p>
          </div>
        </div>
      </main>
    </div>
  );
}