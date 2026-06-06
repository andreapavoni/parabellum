export function CapitalBadge({ compact = false }: { compact?: boolean }) {
  return (
    <span
      class={
        compact
          ? "ml-2 rounded bg-amber-100 px-1.5 py-0.5 text-[10px] font-semibold text-amber-800"
          : "ml-2 rounded bg-amber-100 px-2 py-0.5 text-xs font-semibold text-amber-800"
      }
    >
      Capital
    </span>
  );
}

