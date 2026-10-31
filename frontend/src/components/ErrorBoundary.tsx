import { ErrorBoundary as SolidErrorBoundary, Show } from 'solid-js';

interface ErrorBoundaryProps {
  children: any;
}

/**
 * Top-level error boundary (issue #49).
 *
 * Wraps Solid's built-in ErrorBoundary so that an unexpected rendering error
 * anywhere in the subtree (e.g. a malformed API payload reaching a component)
 * is caught here instead of tearing down the whole app to a blank screen.
 * The user sees a friendly message with a retry option, and the error is
 * logged to the console for debugging.
 */
export function ErrorBoundary(props: ErrorBoundaryProps) {
  return (
    <SolidErrorBoundary
      fallback={(error, reset) => {
        // Log the failure so it can be diagnosed from the console (issue #49).
        console.error('ErrorBoundary caught an error:', error);

        return (
          <div
            class="min-h-[60vh] flex items-center justify-center px-4"
            data-testid="error-boundary"
            role="alert"
          >
            <div class="max-w-md w-full border rounded-lg shadow-sm bg-white p-8 text-center">
              <h2 class="text-2xl font-bold text-gray-900 mb-2">Something went wrong</h2>
              <p class="text-gray-600 mb-4">
                An unexpected error occurred while rendering this page. You can try
                again or return to the home page.
              </p>
              <Show when={error instanceof Error && error.message}>
                <p
                  class="text-sm text-red-600 mb-4 break-words"
                  data-testid="error-boundary-message"
                >
                  {error instanceof Error ? error.message : String(error)}
                </p>
              </Show>
              <div class="flex items-center justify-center gap-3">
                <button
                  type="button"
                  onClick={() => reset()}
                  class="px-4 py-2 rounded bg-blue-600 text-white text-sm font-medium hover:bg-blue-700"
                >
                  Retry
                </button>
                <a
                  href="/"
                  class="px-4 py-2 rounded border border-gray-300 text-gray-700 text-sm font-medium hover:bg-gray-50"
                >
                  Go home
                </a>
              </div>
            </div>
          </div>
        );
      }}
    >
      {props.children}
    </SolidErrorBoundary>
  );
}
