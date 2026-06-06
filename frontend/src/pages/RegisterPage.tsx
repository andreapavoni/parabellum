import { useState } from "preact/hooks";
import { Button, Panel } from "@/components/ui";

export function RegisterPage({
  error,
  onSubmit,
}: {
  error: string | null;
  onSubmit: (payload: {
    username: string;
    email: string;
    password: string;
    tribe: string;
    quadrant: string;
  }) => Promise<void>;
}) {
  const [form, setForm] = useState({
    username: "",
    email: "",
    password: "",
    tribe: "Roman",
    quadrant: "NorthEast",
  });
  const [pending, setPending] = useState(false);

  return (
    <div class="mx-auto max-w-md px-4 py-10">
      <Panel class="p-6">
        <h1 class="text-2xl font-semibold text-gray-800">Register</h1>
        <form
          class="mt-4 space-y-4"
          onSubmit={async (event) => {
            event.preventDefault();
            setPending(true);
            try {
              await onSubmit(form);
            } catch {
              // Error state is rendered by the parent.
            } finally {
              setPending(false);
            }
          }}
        >
          <Field label="Username" value={form.username} onInput={(username) => setForm((current) => ({ ...current, username }))} />
          <Field label="Email" value={form.email} onInput={(email) => setForm((current) => ({ ...current, email }))} />
          <Field
            label="Password"
            type="password"
            value={form.password}
            onInput={(password) => setForm((current) => ({ ...current, password }))}
          />
          <Select
            label="Tribe"
            value={form.tribe}
            options={["Roman", "Gaul", "Teuton"]}
            onInput={(tribe) => setForm((current) => ({ ...current, tribe }))}
          />
          <Select
            label="Quadrant"
            value={form.quadrant}
            options={["NorthEast", "NorthWest", "SouthEast", "SouthWest"]}
            onInput={(quadrant) => setForm((current) => ({ ...current, quadrant }))}
          />
          {error ? <div class="rounded bg-red-50 px-3 py-2 text-sm text-red-700">{error}</div> : null}
          <Button type="submit" disabled={pending} class="w-full">
            {pending ? "Creating account..." : "Create account"}
          </Button>
        </form>
      </Panel>
    </div>
  );
}

function Field({
  label,
  value,
  onInput,
  type = "text",
}: {
  label: string;
  value: string;
  onInput: (value: string) => void;
  type?: string;
}) {
  return (
    <label class="block text-sm text-gray-700">
      {label}
      <input
        type={type}
        class="mt-1 w-full rounded border border-gray-300 px-3 py-2"
        value={value}
        onInput={(event) => onInput((event.target as HTMLInputElement).value)}
      />
    </label>
  );
}

function Select({
  label,
  value,
  options,
  onInput,
}: {
  label: string;
  value: string;
  options: string[];
  onInput: (value: string) => void;
}) {
  return (
    <label class="block text-sm text-gray-700">
      {label}
      <select
        class="mt-1 w-full rounded border border-gray-300 px-3 py-2"
        value={value}
        onInput={(event) => onInput((event.target as HTMLSelectElement).value)}
      >
        {options.map((option) => (
          <option value={option}>{option}</option>
        ))}
      </select>
    </label>
  );
}
