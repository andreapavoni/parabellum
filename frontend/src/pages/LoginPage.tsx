import { useState } from "preact/hooks";

export function LoginPage({
  error,
  onSubmit,
}: {
  error: string | null;
  onSubmit: (payload: { username: string; password: string }) => Promise<void>;
}) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [pending, setPending] = useState(false);

  return (
    <div class="mx-auto max-w-md px-4 py-10">
      <div class="rounded-xl border border-gray-200 bg-white p-6 shadow-sm">
        <h1 class="text-2xl font-semibold text-gray-800">Login</h1>
        <form
          class="mt-4 space-y-4"
          onSubmit={async (event) => {
            event.preventDefault();
            setPending(true);
            try {
              await onSubmit({ username, password });
            } catch {
              // Error state is rendered by the parent.
            } finally {
              setPending(false);
            }
          }}
        >
          <label class="block text-sm text-gray-700">
            Username
            <input
              class="mt-1 w-full rounded border border-gray-300 px-3 py-2"
              value={username}
              onInput={(event) => setUsername((event.target as HTMLInputElement).value)}
            />
          </label>
          <label class="block text-sm text-gray-700">
            Password
            <input
              type="password"
              class="mt-1 w-full rounded border border-gray-300 px-3 py-2"
              value={password}
              onInput={(event) => setPassword((event.target as HTMLInputElement).value)}
            />
          </label>
          {error ? <div class="rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div> : null}
          <button disabled={pending} class="w-full rounded bg-green-700 px-4 py-2 text-sm font-semibold text-white hover:bg-green-800 disabled:opacity-60">
            {pending ? "Signing in..." : "Sign in"}
          </button>
        </form>
      </div>
    </div>
  );
}
