import type { ComponentChildren } from "preact";
import { navigate, shouldUseClientNavigation } from "@/lib/router";

type LinkProps = {
  to: string;
  class?: string;
  children: ComponentChildren;
};

export function Link(props: LinkProps) {
  return (
    <a
      href={props.to}
      class={props.class}
      onClick={(event) => {
        if (
          event.defaultPrevented ||
          event.button !== 0 ||
          event.metaKey ||
          event.ctrlKey ||
          event.shiftKey ||
          event.altKey
        ) {
          return;
        }
        if (!shouldUseClientNavigation(props.to)) {
          return;
        }
        event.preventDefault();
        navigate(props.to);
      }}
    >
      {props.children}
    </a>
  );
}
