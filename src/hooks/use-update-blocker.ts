import { useId, useLayoutEffect } from "react";
import { useUpdateStore } from "@/stores/update-store";

/** An open editor owns this lease until its draft is saved or deliberately discarded. */
export function useUpdateBlocker(blocked: boolean) {
  const id = useId();
  useLayoutEffect(() => {
    useUpdateStore.setState((s) => ({ blockers: { ...s.blockers, [id]: blocked } }));
    return () => {
      useUpdateStore.setState((s) => {
        const blockers = { ...s.blockers };
        delete blockers[id];
        return { blockers };
      });
    };
  }, [id, blocked]);
}
