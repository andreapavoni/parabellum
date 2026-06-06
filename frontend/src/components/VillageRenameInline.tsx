import { useState } from "preact/hooks";
import type { ComponentChildren } from "preact";
import { useRenameVillageMutation } from "@/query/mutations";
import { Button } from "@/components/ui";

export function VillageRenameInline({
  villageId,
  currentName,
  onRenamed,
  className = "w-full mb-3",
  linkClassName = "p-0 text-xs text-green-700 underline hover:text-green-800 bg-transparent border-0",
  label = "Rename village",
}: {
  villageId: number;
  currentName: string;
  onRenamed?: () => Promise<void> | void;
  className?: string;
  linkClassName?: string;
  label?: ComponentChildren;
}) {
  const [editingName, setEditingName] = useState(false);
  const [villageName, setVillageName] = useState(currentName);
  const [renameError, setRenameError] = useState<string | null>(null);
  const renameVillage = useRenameVillageMutation();

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
          {label}
        </button>
      ) : (
        <form
          class="flex flex-wrap items-center gap-2"
          onSubmit={async (event) => {
            event.preventDefault();
            setRenameError(null);
            try {
              await renameVillage.mutateAsync({
                villageId,
                villageName,
              });
              setEditingName(false);
              if (onRenamed) {
                await onRenamed();
              }
            } catch (error) {
              setRenameError((error as Error).message);
            }
          }}
        >
          <input
            class="rounded border border-gray-300 px-2 py-1 text-sm"
            value={villageName}
            maxLength={32}
            onInput={(event) => setVillageName((event.target as HTMLInputElement).value)}
          />
          <Button
            type="submit"
            disabled={renameVillage.isPending}
            size="sm"
          >
            Save
          </Button>
          <Button
            type="button"
            variant="secondary"
            size="sm"
            onClick={() => {
              setEditingName(false);
              setRenameError(null);
              setVillageName(currentName);
            }}
          >
            Cancel
          </Button>
        </form>
      )}
      {renameError ? <div class="mt-2 text-xs text-red-600">{renameError}</div> : null}
    </div>
  );
}
