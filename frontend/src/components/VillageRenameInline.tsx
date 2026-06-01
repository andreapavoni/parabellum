import { useState } from "preact/hooks";
import { api } from "@/lib/api";

export function VillageRenameInline({
  villageId,
  currentName,
  onRenamed,
  className = "w-full mb-3",
  linkClassName = "p-0 text-xs text-green-700 underline hover:text-green-800 bg-transparent border-0",
}: {
  villageId: number;
  currentName: string;
  onRenamed?: () => Promise<void> | void;
  className?: string;
  linkClassName?: string;
}) {
  const [editingName, setEditingName] = useState(false);
  const [villageName, setVillageName] = useState(currentName);
  const [renameError, setRenameError] = useState<string | null>(null);
  const [renaming, setRenaming] = useState(false);

  return (
    <div class={className}>
      {!editingName ? (
        <button
          type="button"
          class={linkClassName}
          onClick={() => {
            setVillageName(currentName);
            setRenameError(null);
            setEditingName(true);
          }}
        >
          Rename village
        </button>
      ) : (
        <form
          class="flex flex-wrap items-center gap-2"
          onSubmit={async (event) => {
            event.preventDefault();
            setRenameError(null);
            setRenaming(true);
            try {
              await api.renameVillage({
                villageId,
                villageName,
              });
              setEditingName(false);
              if (onRenamed) {
                await onRenamed();
              }
            } catch (error) {
              setRenameError((error as Error).message);
            } finally {
              setRenaming(false);
            }
          }}
        >
          <input
            class="rounded border border-gray-300 px-2 py-1 text-sm"
            value={villageName}
            maxLength={32}
            onInput={(event) => setVillageName((event.target as HTMLInputElement).value)}
          />
          <button
            type="submit"
            disabled={renaming}
            class="rounded bg-blue-600 px-2 py-1 text-xs text-white disabled:bg-blue-300"
          >
            Save
          </button>
          <button
            type="button"
            class="rounded border border-gray-300 px-2 py-1 text-xs hover:bg-gray-50"
            onClick={() => {
              setEditingName(false);
              setRenameError(null);
              setVillageName(currentName);
            }}
          >
            Cancel
          </button>
        </form>
      )}
      {renameError ? <div class="mt-2 text-xs text-red-600">{renameError}</div> : null}
    </div>
  );
}
