export function Loading({ label = "Loading..." }: { label?: string }) {
  return <div class="mx-auto max-w-4xl px-4 py-10 text-sm text-gray-500">{label}</div>;
}
