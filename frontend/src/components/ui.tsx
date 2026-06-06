import type { ComponentChildren, JSX } from "preact";

export function cx(...classes: Array<string | false | null | undefined>) {
  return classes.filter(Boolean).join(" ");
}

type ButtonVariant = "primary" | "secondary" | "danger" | "warning" | "ghost";
type ButtonSize = "sm" | "md";

const buttonVariants: Record<ButtonVariant, string> = {
  primary: "border-green-700 bg-green-700 text-white hover:bg-green-800",
  secondary: "border-stone-300 bg-white text-stone-800 hover:bg-stone-50",
  danger: "border-red-700 bg-red-700 text-white hover:bg-red-800",
  warning: "border-amber-700 bg-amber-700 text-white hover:bg-amber-800",
  ghost: "border-transparent bg-transparent text-green-800 hover:bg-green-50",
};

const buttonSizes: Record<ButtonSize, string> = {
  sm: "px-2.5 py-1 text-xs",
  md: "px-3.5 py-2 text-sm",
};

export function Button({
  variant = "primary",
  size = "md",
  class: className,
  children,
  ...props
}: JSX.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
  children: ComponentChildren;
}) {
  return (
    <button
      {...props}
      class={cx(
        "inline-flex items-center justify-center gap-1.5 rounded-md border font-semibold leading-none transition-colors disabled:opacity-60",
        buttonVariants[variant],
        buttonSizes[size],
        className,
      )}
    >
      {children}
    </button>
  );
}

export function IconButton({
  label,
  class: className,
  children,
  ...props
}: JSX.ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  children: ComponentChildren;
}) {
  return (
    <button
      {...props}
      aria-label={label}
      title={label}
      class={cx(
        "inline-flex h-8 w-8 items-center justify-center rounded-md border border-stone-300 bg-white text-xs font-semibold text-stone-700 transition-colors hover:bg-stone-50 disabled:opacity-60",
        className,
      )}
    >
      {children}
    </button>
  );
}

export function Panel({
  class: className,
  children,
}: {
  class?: string;
  children: ComponentChildren;
}) {
  return (
    <section class={cx("rounded-md border border-stone-200 bg-white p-4 shadow-sm", className)}>
      {children}
    </section>
  );
}

export function SectionHeader({
  title,
  aside,
  class: className,
}: {
  title: ComponentChildren;
  aside?: ComponentChildren;
  class?: string;
}) {
  return (
    <div class={cx("mb-3 flex items-center justify-between gap-3", className)}>
      <h2 class="text-sm font-semibold uppercase tracking-wide text-stone-600">{title}</h2>
      {aside ? <div class="shrink-0 text-xs text-stone-500">{aside}</div> : null}
    </div>
  );
}

type BadgeVariant = "neutral" | "success" | "warning" | "danger";

const badgeVariants: Record<BadgeVariant, string> = {
  neutral: "border-stone-200 bg-stone-100 text-stone-700",
  success: "border-green-200 bg-green-50 text-green-800",
  warning: "border-amber-200 bg-amber-50 text-amber-800",
  danger: "border-red-200 bg-red-50 text-red-800",
};

export function Badge({
  variant = "neutral",
  class: className,
  children,
}: {
  variant?: BadgeVariant;
  class?: string;
  children: ComponentChildren;
}) {
  return (
    <span class={cx("inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-semibold", badgeVariants[variant], className)}>
      {children}
    </span>
  );
}

export function Field({
  label,
  children,
  class: className,
}: {
  label: ComponentChildren;
  children: ComponentChildren;
  class?: string;
}) {
  return (
    <label class={cx("block text-sm font-medium text-stone-600", className)}>
      <span>{label}</span>
      <div class="mt-1">{children}</div>
    </label>
  );
}

export function DataTable({
  class: className,
  children,
}: {
  class?: string;
  children: ComponentChildren;
}) {
  return (
    <div class={cx("overflow-hidden rounded-md border border-stone-200 bg-white", className)}>
      <div class="overflow-x-auto">
        <table class="min-w-full text-sm">{children}</table>
      </div>
    </div>
  );
}
